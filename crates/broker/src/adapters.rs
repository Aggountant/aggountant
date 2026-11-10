//! F12 — the concrete adapters, all [`RequestResponse`]:
//! - **`vero-verotili`** — pull the tax-account (OmaVero) snapshot (`AtLeastOnce`:
//!   a read is safe to repeat).
//! - **`synthetic-submit`** — a test-only at-most-once write whose explicit
//!   contract uses `409 already_submitted` to exercise generic Broker recovery.
//! - **`pankki-maksu`** — submit a payment order (`AtMostOnce`: a payment must not
//!   double; `409` recovers a duplicate).
//!
//! Every adapter is a thin wrapper over an injected [`Transport`]: the *interpreted*
//! logic — mapping a raw status to an [`AdapterResponse`]/[`AdapterError`], and
//! **the `409 → RecoveredAlreadyProcessed` mapping (M3)** — is the tested F12 code;
//! the transport is dumb. There is no live transport in v1, so the mapping is
//! exercised through a [`ScriptedTransport`].
//!
//! **At-most-once hardening (`broker-5xx-atmostonce-semantics`).** For a submit the
//! `409` recovery is only accepted when the response *confirms our idempotency key*;
//! a `409` naming a different key is a definitive rejection, and an *unverifiable*
//! `409` is ambiguous → `Unknown` (never a terminal `Rejected`, since the effect may
//! be committed). And **every** `5xx` (the whole `500..=599` class) — including `503`
//! — is ambiguous → [`AdapterError::AmbiguousDelivery`] (`Unknown`), never a blind
//! retry, because a proxy `5xx` may follow an upstream commit. Both ambiguous causes
//! are surfaced as [`AdapterError::AmbiguousDelivery`] (a proxy `5xx` / an
//! unverifiable `409`) rather than [`AdapterError::Timeout`] (which stays reserved
//! for *no answer at all*), so the outbox trail can tell them apart
//! (`broker-unknown-cause-observability`). A pull (read) carries neither hazard and
//! retries a `5xx` freely.

use crate::adapter::{
    AdapterKind, AdapterResponse, ConsultChunk, RawRequest, RawResponse, RequestResponse, Stream,
    Transport, TransportError,
};
use crate::envelope::{DeliverySemantics, Envelope};
use crate::error::AdapterError;
use crate::vault::CredentialHandle;

/// Service key constants (match the envelope `service` and the registry keys).
pub const SVC_VERO_VEROTILI: &str = "vero-verotili";
pub const SVC_SYNTHETIC_SUBMIT: &str = "synthetic-submit";
pub const SVC_PANKKI_MAKSU: &str = "pankki-maksu";
/// The Tulorekisteri (incomes register) submit service (S4/RF4). One adapter serves
/// both report kinds — the operation (`palkkatietoilmoitus` / `erillisilmoitus`)
/// distinguishes them in the envelope, so the two distinct obligations are not
/// conflated at the transport layer.
pub const SVC_TULOREKISTERI: &str = "tulorekisteri";
/// The `consult-llm` Stream service (S5/RF1) — the external LLM boundary. Only a
/// **projected** disclosure ever reaches it (`redaction-strategy-options.md` §4).
pub const SVC_CONSULT_LLM: &str = "consult-llm";
/// The email pull service (S5/RF2): a targeted, per-transaction document pull.
pub const SVC_EMAIL_PULL: &str = "email-pull";
/// The webshop (kauppatili) pull service (S5/RF2): a targeted, per-transaction pull.
pub const SVC_KAUPPATILI_PULL: &str = "kauppatili-pull";

fn raw_request(envelope: &Envelope) -> RawRequest {
    RawRequest {
        service: envelope.service.clone(),
        operation: envelope.operation.clone(),
        // The authority's idempotency key = our source-based natural_key.
        idempotency_key: envelope.natural_key.clone(),
        body: envelope.body.clone(),
    }
}

/// Map a transport error to the adapter taxonomy. A timeout is the load-bearing
/// case: it becomes [`AdapterError::Timeout`] → the Broker's `Unknown`.
fn map_transport_error(e: TransportError) -> AdapterError {
    match e {
        TransportError::Timeout => AdapterError::Timeout("no definitive response".into()),
        TransportError::ConnectionRefused(m) => AdapterError::RetryableTransport(m),
    }
}

/// The recovery posture a caller of [`interpret`] declares. A submit (`AtMostOnce`)
/// adapter passes [`Recover::AtMostOnce`] with the request's idempotency key so a
/// `409` can be *verified* against it; a pull passes [`Recover::None`] (a read has no
/// already-submitted semantics). This is where the adapter, not the Broker, owns the
/// `409` and ambiguous-`5xx` interpretation.
enum Recover<'a> {
    /// Not a recoverable submit — a pull/read. A `409` is a plain rejection and an
    /// ambiguous `5xx` is a safe-to-retry transport failure (no double-effect hazard).
    None,
    /// A submit whose idempotency key is `key`. A `409` recovers **only** when the
    /// authority's response confirms this exact key as already-processed; any `5xx`
    /// is ambiguous → `Unknown`, never a blind retry. `classify` is the adapter's own
    /// `409` body parser (each authority owns its response schema).
    AtMostOnce {
        key: &'a str,
        classify: ConflictClassifier,
    },
}

/// What a `409` body says about **our** request's identity (idempotency key `key`).
/// A `409` is the authority's response to *this* attempt, so it always means "not
/// accepted as-is" — the question is *why*, which decides the outcome:
/// - [`ConflictVerdict::OurKey`] — the body confirms our exact key as the
///   already-processed one → an idempotent recovery (a duplicate of a landed submit).
/// - [`ConflictVerdict::OtherKey`] — the body names a *different* key → the conflict is
///   with some other resource; our request was provably NOT the already-processed one
///   → a definitive rejection.
/// - [`ConflictVerdict::Unverifiable`] — no identity echoed (or contradictory fields):
///   we cannot tell "your prior attempt already landed" from "rejected for another
///   reason". For an `AtMostOnce` submit that is **ambiguous → `Unknown`**, never a
///   terminal state, because reporting `Rejected` for a possibly-committed effect is
///   the mirror-image hazard of laundering a false success.
///
/// This is the *verdict*, not the *outcome*: the ternary safety mapping
/// (unverifiable → `Unknown`, other-key → `Rejected`, our-key → recovery) lives in
/// [`interpret`]. A per-adapter [`ConflictClassifier`] owns only the body-schema
/// parsing that produces this verdict, so each authority may read its own response
/// shape without re-deciding the safety semantics.
///
/// Deliberately a *closed* ternary: a new variant must force a compile error in
/// [`interpret`] (where the safety mapping is exhaustive), so it is intentionally
/// **not** `#[non_exhaustive]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictVerdict {
    OurKey,
    OtherKey,
    Unverifiable,
}

/// A submit adapter's `409` conflict-body parser: given the raw `409` body and our
/// idempotency `key`, return the [`ConflictVerdict`]. An authority with a documented
/// idempotency contract may echo the key in its own response schema — nested under
/// `{error:{…}}`, renamed, or numeric — so **each submit adapter carries its own
/// classifier** rather than every adapter sharing one hardcoded top-level parser. That
/// keeps conflict-body interpretation adapter-owned (the M3 split): a live adapter
/// swaps in the authority's real schema parser via [`SyntheticSubmitAdapter::with_classifier`]
/// (and its siblings) without touching the shared safety semantics in [`interpret`].
///
/// In v1 every 409 comes from a [`ScriptedTransport`] speaking *our own* field names,
/// so all three submit adapters default to [`classify_conflict_scripted`].
///
/// # Safety contract (this is a trusted, effect-safety-bearing hook)
/// The verdict decides whether a possibly-committed effect is reported as recovered,
/// rejected, or ambiguous, so an implementation MUST:
/// - return [`ConflictVerdict::OurKey`] **only** on authoritative, exact proof that the
///   `409` names *our* `key` as the already-processed request;
/// - return [`ConflictVerdict::OtherKey`] **only** when the authority's contract proves
///   the named different key means *our* request was not the one on file;
/// - return [`ConflictVerdict::Unverifiable`] for any absent, malformed (wrong JSON
///   type), contradictory, or lossily-converted identity evidence — never launder such
///   a body into `OurKey`.
///
/// `body` may be **any** JSON value (a proxy can return a non-object, a string, or an
/// HTML-shaped payload), so the classifier must inspect it defensively and MUST NOT
/// panic — a panic here aborts the Broker's send path.
pub type ConflictClassifier = fn(&serde_json::Value, &str) -> ConflictVerdict;

/// The default v1 classifier: parse a `409` body that speaks *this system's* own
/// top-level identity fields (`idempotency_key`, `natural_key` — this system's key ==
/// the envelope's `natural_key`). This is what a [`ScriptedTransport`] emits; a live
/// adapter with a different or nested response schema supplies its own
/// [`ConflictClassifier`] instead. A confirmed key must NOT contradict itself: if any
/// present identity field disagrees with another, the verdict is `Unverifiable`, not a
/// recovery.
///
/// A *present but non-string* identity field (e.g. a numeric `natural_key`) is
/// **malformed evidence**, not absent evidence: it forces `Unverifiable`. Silently
/// dropping it (treating it as absent) could let a lone matching sibling field launder
/// a contradictory body into a false `OurKey` — a false terminal recovery, the exact
/// hazard the safety semantics guard against.
pub fn classify_conflict_scripted(body: &serde_json::Value, key: &str) -> ConflictVerdict {
    let mut matched_ours = false;
    let mut matched_other = false;
    for field in ["idempotency_key", "natural_key"] {
        match body.get(field) {
            None => {}
            Some(serde_json::Value::String(v)) if v == key => matched_ours = true,
            Some(serde_json::Value::String(_)) => matched_other = true,
            // Present but not a string → malformed identity evidence, cannot verify.
            Some(_) => return ConflictVerdict::Unverifiable,
        }
    }
    match (matched_ours, matched_other) {
        // Every present field is a string echoing our exact key.
        (true, false) => ConflictVerdict::OurKey,
        // Every present field is a string naming a different key.
        (false, true) => ConflictVerdict::OtherKey,
        // Nothing echoed, or contradictory (one field ours, another different) — not
        // trustworthy proof either way.
        (false, false) | (true, true) => ConflictVerdict::Unverifiable,
    }
}

/// The shared status→outcome interpretation. `recover` says whether this is a
/// recoverable submit (and carries the idempotency key to verify a `409` against) or
/// a plain pull. **This is where the adapter, not the Broker, owns the `409` and the
/// ambiguous-`5xx` interpretation.**
fn interpret(resp: RawResponse, recover: Recover<'_>) -> Result<AdapterResponse, AdapterError> {
    // TODO(live-adapter): receipt extraction is still hardcoded top-level. An authority
    // that nests/renames its idempotency key (see `ConflictClassifier`) will likely nest
    // its receipt too — a nested-key recovery would then yield `receipt: ""`. When a live
    // adapter lands, this should move behind the same per-adapter seam as the classifier.
    let receipt = || {
        resp.body
            .get("receipt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    match resp.status {
        200 | 201 => Ok(AdapterResponse::Accepted {
            receipt: receipt(),
            body: resp.body,
        }),
        // A `409` recovers to already-processed ONLY for a submit whose exact
        // idempotency key the authority confirms. A `409` that names a *different* key
        // is a genuine conflict (our request provably not the one on file) → a
        // definitive rejection. A `409` we cannot verify (no key echoed) is AMBIGUOUS
        // for a submit — the effect may in fact be committed — so it becomes `Unknown`,
        // never a terminal `Rejected` a caller could safely resubmit under a fresh key
        // (the mirror-image of the false-success hazard). A pull has no recovery
        // semantics at all: any `409` is a plain rejection.
        // NOTE on payload identity: within one effect the Broker reuses a stable
        // `sent_hash`, so a confirmed key matched to *our* replayed bytes is our
        // request. Across independent effects/clients sharing a key this echo is not
        // proof — a residual trust in the authority's key→payload binding (single-user
        // v1; see `issues/multi-user-considerations`).
        409 => match recover {
            Recover::AtMostOnce { key, classify } => match classify(&resp.body, key) {
                ConflictVerdict::OurKey => {
                    Ok(AdapterResponse::RecoveredAlreadyProcessed { receipt: receipt() })
                }
                ConflictVerdict::OtherKey => Err(AdapterError::Rejected {
                    code: "CONFLICT_OTHER_KEY".into(),
                    message: "409 names a different idempotency key — our request was not \
                              the already-processed one"
                        .into(),
                }),
                // Ambiguous: cannot prove our effect was NOT processed → Unknown, not a
                // terminal state. Surfaced as `AmbiguousDelivery` (an answer arrived but
                // is unverifiable) so the Broker returns `Unknown` while the outbox trail
                // still distinguishes it from a socket timeout.
                ConflictVerdict::Unverifiable => Err(AdapterError::AmbiguousDelivery {
                    code: "HTTP_409".into(),
                    detail: "409 without a verifiable idempotency key — request may have \
                             been processed"
                        .into(),
                }),
            },
            Recover::None => Err(AdapterError::Rejected {
                code: "HTTP_409".into(),
                message: "conflict on a non-idempotent-recoverable operation".into(),
            }),
        },
        400 | 422 => Err(AdapterError::Rejected {
            code: format!("HTTP_{}", resp.status),
            message: resp
                .body
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("validation failed")
                .to_string(),
        }),
        401 | 403 => Err(AdapterError::Unauthorized(format!("HTTP_{}", resp.status))),
        429 => Err(AdapterError::RateLimited {
            retry_after_secs: resp.body.get("retry_after_secs").and_then(|v| v.as_u64()),
        }),
        // EVERY `5xx` (the whole `500..=599` class, incl. proxy-only codes like 520-527)
        // is AMBIGUOUS for an `AtMostOnce` submit: a `500`/`502`/`504` may be a
        // post-commit crash, and a `503` — even though an origin server means "not
        // served" — may come from a *proxy* that already forwarded the request and saw
        // the upstream commit before the connection dropped. That is exactly the
        // double-effect hazard the ambiguous-outcome mapping guards, so surface it as
        // `AmbiguousDelivery` → the Broker returns `Unknown` and does NOT blind-retry;
        // the driver's deliberate retry reconciles via the authority's `409`. Unlike a
        // socket timeout, an answer *did* arrive here, so it carries its own `HTTP_<n>`
        // cause code — the outbox trail keeps the two apart. This is a deliberate
        // conservative trade: a genuine transient origin `5xx` no longer auto-retries (it
        // needs a deliberate retry), buying at-most-once safety with a throughput cost. A
        // pull (idempotent read) has no such hazard and may safely retry any `5xx`.
        // Exhaustive inner match on `recover` (not a guard + fall-through): adding a
        // `Recover` variant must force a compile error here rather than silently drop a
        // new posture into the safe-to-retry arm — the same closed-set discipline the
        // `ConflictVerdict` mapping relies on.
        500..=599 => match recover {
            Recover::AtMostOnce { .. } => Err(AdapterError::AmbiguousDelivery {
                code: format!("HTTP_{}", resp.status),
                detail: format!(
                    "HTTP {} — ambiguous; request may have been processed",
                    resp.status
                ),
            }),
            Recover::None => Err(AdapterError::RetryableTransport(format!(
                "HTTP_{}",
                resp.status
            ))),
        },
        other => Err(AdapterError::Rejected {
            code: format!("HTTP_{other}"),
            message: "unexpected status".into(),
        }),
    }
}

/// Build a submit-style adapter (`synthetic-submit`, `pankki-maksu`): `AtMostOnce`,
/// interprets a *verified* `409` as an idempotent recovery (and every `5xx` as
/// ambiguous → `Unknown`).
macro_rules! submit_adapter {
    ($name:ident, $service:expr, $semantics:expr) => {
        pub struct $name {
            transport: Box<dyn Transport>,
            /// This adapter's own `409` conflict-body parser. In v1 it is
            /// [`classify_conflict_scripted`]; a live transport swaps in the
            /// authority's real schema parser via [`Self::with_classifier`].
            classify: ConflictClassifier,
        }
        impl $name {
            /// Build the adapter with the default v1 (scripted-schema) `409`
            /// classifier — the `409` bodies come from a [`ScriptedTransport`]
            /// speaking this system's own field names.
            pub fn new(transport: Box<dyn Transport>) -> Self {
                $name {
                    transport,
                    classify: classify_conflict_scripted,
                }
            }
            /// Build the adapter with a per-authority `409` [`ConflictClassifier`] —
            /// the hook a live transport uses so this authority owns its own
            /// conflict-body schema (nested `{error:{…}}`, renamed, or numeric key
            /// echoes) without touching the shared safety semantics in [`interpret`].
            pub fn with_classifier(
                transport: Box<dyn Transport>,
                classify: ConflictClassifier,
            ) -> Self {
                $name {
                    transport,
                    classify,
                }
            }
        }
        impl RequestResponse for $name {
            fn service(&self) -> &str {
                $service
            }
            fn kind(&self) -> AdapterKind {
                AdapterKind::Submit
            }
            fn assurance(&self) -> DeliverySemantics {
                $semantics
            }
            fn call(
                &self,
                envelope: &Envelope,
                _cred: &CredentialHandle,
            ) -> Result<AdapterResponse, AdapterError> {
                match self.transport.round_trip(&raw_request(envelope)) {
                    // A submit verifies a `409` against its own idempotency key
                    // (== the envelope's `natural_key`, per `raw_request`), using
                    // this adapter's own conflict-body classifier.
                    Ok(resp) => interpret(
                        resp,
                        Recover::AtMostOnce {
                            key: &envelope.natural_key,
                            classify: self.classify,
                        },
                    ),
                    Err(e) => Err(map_transport_error(e)),
                }
            }
        }
    };
}

submit_adapter!(
    SyntheticSubmitAdapter,
    SVC_SYNTHETIC_SUBMIT,
    DeliverySemantics::AtMostOnce
);
submit_adapter!(
    PankkiMaksuAdapter,
    SVC_PANKKI_MAKSU,
    DeliverySemantics::AtMostOnce
);
submit_adapter!(
    TulorekisteriAdapter,
    SVC_TULOREKISTERI,
    DeliverySemantics::AtMostOnce
);

/// The `vero-verotili` pull adapter: `AtLeastOnce` (a read is safe to repeat), no
/// `409` recovery semantics. On success the body is the [`TaxAccountSnapshot`]
/// JSON the caller parses.
///
/// [`TaxAccountSnapshot`]: aggountant_domain::TaxAccountSnapshot
pub struct VeroVerotiliAdapter {
    transport: Box<dyn Transport>,
}

impl VeroVerotiliAdapter {
    pub fn new(transport: Box<dyn Transport>) -> Self {
        VeroVerotiliAdapter { transport }
    }
}

impl RequestResponse for VeroVerotiliAdapter {
    fn service(&self) -> &str {
        SVC_VERO_VEROTILI
    }
    fn kind(&self) -> AdapterKind {
        AdapterKind::Pull
    }
    fn assurance(&self) -> DeliverySemantics {
        DeliverySemantics::AtLeastOnce
    }
    fn call(
        &self,
        envelope: &Envelope,
        _cred: &CredentialHandle,
    ) -> Result<AdapterResponse, AdapterError> {
        match self.transport.round_trip(&raw_request(envelope)) {
            Ok(resp) => interpret(resp, Recover::None),
            Err(e) => Err(map_transport_error(e)),
        }
    }
}

/// The pull's own status→verdict interpretation: a `404` becomes a **semantic**
/// `NOT_FOUND` rejection (a legitimate "no evidence for this transaction"), so the Broker
/// reads the adapter's *verdict code*, never an HTTP status — the M3 split. Everything
/// else defers to the shared [`interpret`] (no `409` recovery for a pull).
fn interpret_pull(resp: RawResponse) -> Result<AdapterResponse, AdapterError> {
    if resp.status == 404 {
        return Err(AdapterError::Rejected {
            code: "NOT_FOUND".into(),
            message: "no candidate document for this transaction".into(),
        });
    }
    interpret(resp, Recover::None)
}

/// Build a **pull** adapter (`email-pull`, `kauppatili-pull`): `AtLeastOnce` (a read is
/// safe to repeat), no `409` recovery. A targeted per-transaction document pull; the
/// Broker's `pull` reads the adapter's verdict (a `200` candidate document, a `NOT_FOUND`
/// no-match, a `429` rate limit) without ever reading the document schema or a status code.
macro_rules! pull_adapter {
    ($name:ident, $service:expr) => {
        pub struct $name {
            transport: Box<dyn Transport>,
        }
        impl $name {
            pub fn new(transport: Box<dyn Transport>) -> Self {
                $name { transport }
            }
        }
        impl RequestResponse for $name {
            fn service(&self) -> &str {
                $service
            }
            fn kind(&self) -> AdapterKind {
                AdapterKind::Pull
            }
            fn assurance(&self) -> DeliverySemantics {
                DeliverySemantics::AtLeastOnce
            }
            fn call(
                &self,
                envelope: &Envelope,
                _cred: &CredentialHandle,
            ) -> Result<AdapterResponse, AdapterError> {
                match self.transport.round_trip(&raw_request(envelope)) {
                    Ok(resp) => interpret_pull(resp),
                    Err(e) => Err(map_transport_error(e)),
                }
            }
        }
    };
}

pull_adapter!(EmailPullAdapter, SVC_EMAIL_PULL);
pull_adapter!(KauppatiliPullAdapter, SVC_KAUPPATILI_PULL);

/// The `consult-llm` **Stream** adapter (F12, S5/RF1). It talks to the injected
/// [`Transport`] like every other adapter; the *interpreted* logic is mapping a raw
/// response to a [`ConsultChunk`] (the consult wire protocol) and surfacing a `429` as
/// [`AdapterError::RateLimited`] so the Run can park on an `Effect::Wait`. It never reads
/// the `Final` body's domain meaning — that is `Advice`, a Domain concern.
pub struct ConsultLlmAdapter {
    transport: Box<dyn Transport>,
}

impl ConsultLlmAdapter {
    pub fn new(transport: Box<dyn Transport>) -> Self {
        ConsultLlmAdapter { transport }
    }
}

/// Map a consult `RawResponse` to a chunk. The `type` field is the consult protocol
/// (adapter-owned, like a status code); the `Final` body stays opaque.
fn interpret_consult(resp: RawResponse) -> Result<ConsultChunk, AdapterError> {
    match resp.status {
        200 => {
            let ty = resp.body.get("type").and_then(|v| v.as_str()).unwrap_or("");
            match ty {
                "partial" => Ok(ConsultChunk::Partial {
                    text: resp
                        .body
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                }),
                "tool_call" => Ok(ConsultChunk::ToolCall {
                    tool: resp
                        .body
                        .get("tool")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    args: resp
                        .body
                        .get("args")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                }),
                "final" => Ok(ConsultChunk::Final {
                    body: resp.body.get("advice").cloned().unwrap_or(resp.body),
                }),
                other => Err(AdapterError::Malformed(format!(
                    "unknown consult chunk type {other:?}"
                ))),
            }
        }
        401 | 403 => Err(AdapterError::Unauthorized(format!("HTTP_{}", resp.status))),
        400 | 422 => Err(AdapterError::Rejected {
            code: format!("HTTP_{}", resp.status),
            message: "consult request rejected".into(),
        }),
        429 => Err(AdapterError::RateLimited {
            retry_after_secs: resp.body.get("retry_after_secs").and_then(|v| v.as_u64()),
        }),
        // A stream read has no double-effect hazard, so the WHOLE `5xx` class is safely
        // retryable — including proxy-only codes (`507`, `520-527`, `599`). Matching the
        // full range (as `interpret` does) keeps an uncommon `5xx` from falling through
        // to the terminal `Malformed` arm below and killing a retryable stream turn.
        500..=599 => Err(AdapterError::RetryableTransport(format!(
            "HTTP_{}",
            resp.status
        ))),
        other => Err(AdapterError::Malformed(format!(
            "unexpected consult status {other}"
        ))),
    }
}

impl Stream for ConsultLlmAdapter {
    fn service(&self) -> &str {
        SVC_CONSULT_LLM
    }
    fn turn(
        &self,
        request: &RawRequest,
        _cred: &CredentialHandle,
    ) -> Result<ConsultChunk, AdapterError> {
        match self.transport.round_trip(request) {
            Ok(resp) => interpret_consult(resp),
            Err(e) => Err(map_transport_error(e)),
        }
    }
}
