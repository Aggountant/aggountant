//! The **Broker** (F11) and its `Registry`. `broker-structure-adr` decision for
//! this slice: the Broker is an **object with observability**, not a bare pipe —
//! it owns the Registry (service→adapter), the Vault (creds), the outbox, and the
//! retry policy, and it *records every attempt* (the observability surface is the
//! outbox trail). A pipe would scatter idempotency/provenance/retry across callers;
//! centralizing them here is what makes the M3 invariants enforceable in one place.
//!
//! What the Broker does **not** do is as important: it never reads a status code or
//! a domain body. It maps the adapter's [`AdapterError`]/[`AdapterResponse`] verdict
//! to a transport policy. In particular no HTTP status literal appears anywhere in
//! this file — already-processed interpretation is the adapter's (M3 split), and a
//! test asserts this file is free of that literal.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::adapter::{
    AdapterKind, AdapterResponse, ConsultChunk, RawRequest, RequestResponse, Stream,
};
use crate::envelope::{Envelope, Provenance, RemoteOutcome};
use crate::error::{AdapterError, BrokerError};
use crate::outbox::{AttemptOutcome, Outbox};
use crate::vault::Vault;

/// The outcome of one targeted **pull** (`email-pull` / `kauppatili-pull`, RF2). Unlike
/// a submit, a pull has no `Unknown` ambiguity; the load-bearing case is
/// [`PullOutcome::RateLimited`], which the Run turns into an `Effect::Wait`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PullOutcome {
    /// A candidate document body was returned for the transaction.
    Found(serde_json::Value),
    /// No candidate matched this transaction (a legitimate empty result).
    NotFound,
    /// The authority rate-limited the pull; park the Run on a durable wait keyed off
    /// `retry_after_secs`.
    RateLimited { retry_after_secs: Option<u64> },
}

/// The outcome of one `consult-llm` **stream turn** (RF1). Either the next chunk, or a
/// rate limit the Run parks on with an `Effect::Wait`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsultTurn {
    Chunk(ConsultChunk),
    RateLimited { retry_after_secs: Option<u64> },
}

/// O-24 module: **retry**. How many attempts a *retryable transport* failure may
/// consume. A `Timeout` (`Unknown`) is never counted against this — it exits
/// immediately, no retry. Defaults to 3.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy { max_attempts: 3 }
    }
}

/// The service→adapter registry (F11). Adapters register under `adapter.service()`.
/// `RequestResponse` adapters (incl. the RF2 pull adapters) and `Stream` adapters (the
/// RF1 `consult-llm` adapter) live in two maps keyed by the same service string.
#[derive(Default)]
pub struct Registry {
    adapters: HashMap<String, Box<dyn RequestResponse>>,
    streams: HashMap<String, Box<dyn Stream>>,
}

impl Registry {
    pub fn new() -> Self {
        Registry::default()
    }
    /// Register a request/response adapter under its own service key.
    pub fn register(&mut self, adapter: Box<dyn RequestResponse>) {
        self.adapters.insert(adapter.service().to_string(), adapter);
    }
    /// Register a stream adapter (`consult-llm`) under its own service key.
    pub fn register_stream(&mut self, adapter: Box<dyn Stream>) {
        self.streams.insert(adapter.service().to_string(), adapter);
    }
    fn get(&self, service: &str) -> Option<&dyn RequestResponse> {
        self.adapters.get(service).map(|b| b.as_ref())
    }
    fn get_stream(&self, service: &str) -> Option<&dyn Stream> {
        self.streams.get(service).map(|b| b.as_ref())
    }
}

/// The Broker (F11): submit an envelope to the right adapter, with idempotency,
/// per-attempt provenance, bounded retry, and a durable outbox.
pub struct Broker {
    home: PathBuf,
    vault: Vault,
    registry: Registry,
    retry: RetryPolicy,
}

impl Broker {
    pub fn new(home: &Path, vault: Vault, registry: Registry) -> Self {
        Broker {
            home: home.to_path_buf(),
            vault,
            registry,
            retry: RetryPolicy::default(),
        }
    }

    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Resolve the request/response adapter for `service` and enforce that its
    /// [`AdapterKind`] matches the operation being invoked. A submit adapter reached
    /// through `fetch`/`pull`, or a pull/read adapter through `submit`, is refused
    /// **here** — before any transport dispatch or outbox write — so a
    /// committed-then-timed-out write can never enter the retrying read path and
    /// double-submit (the whole point of this guard).
    fn resolve_kind(
        &self,
        service: &str,
        expected: AdapterKind,
    ) -> Result<&dyn RequestResponse, BrokerError> {
        let adapter = self
            .registry
            .get(service)
            .ok_or_else(|| BrokerError::UnknownService(service.to_string()))?;
        let actual = adapter.kind();
        if actual != expected {
            return Err(BrokerError::AdapterKindMismatch {
                service: service.to_string(),
                expected,
                actual,
            });
        }
        Ok(adapter)
    }

    /// A read/pull request (e.g. `vero-verotili` → a `TaxAccountSnapshot`). Retries
    /// a *retryable transport* failure within policy and returns the response body
    /// on success. A read is safe to repeat, so it is **not** outbox-recorded and a
    /// timeout is a plain retryable error, not an `Unknown` (there is no
    /// double-effect hazard reading state). Already-processed recovery, which is a
    /// write-only concern, does not apply to a read.
    pub fn fetch(&self, envelope: &Envelope) -> Result<serde_json::Value, BrokerError> {
        let adapter = self.resolve_kind(&envelope.service, AdapterKind::Pull)?;
        let cred = self.vault.handle(&envelope.service)?;
        let mut attempt = 1u32;
        loop {
            match adapter.call(envelope, &cred) {
                Ok(AdapterResponse::Accepted { body, .. }) => return Ok(body),
                Ok(AdapterResponse::RecoveredAlreadyProcessed { .. }) => {
                    // A pull adapter never emits this; treat it defensively as a fetch
                    // failure rather than silently returning an empty body.
                    return Err(BrokerError::Fetch {
                        code: "UNEXPECTED_RECOVERY".into(),
                        message: "a read returned an already-processed verdict".into(),
                    });
                }
                // A read has no double-effect hazard, so a timeout is just retried
                // within policy (not turned into an `Unknown` the way a submit is). An
                // `AmbiguousDelivery` is unreachable on a pull today (its only sources
                // are the adapter's `AtMostOnce` recovery branches, which a read never
                // takes), but it is grouped here so that if a future read adapter emits it, a read
                // retries it freely — consistent with the "reads carry no ambiguity
                // hazard" rule rather than failing terminally.
                Err(
                    AdapterError::Timeout(_)
                    | AdapterError::AmbiguousDelivery { .. }
                    | AdapterError::RetryableTransport(_)
                    | AdapterError::RateLimited { .. },
                ) if attempt < self.retry.max_attempts => {
                    attempt += 1;
                    continue;
                }
                Err(e) => {
                    return Err(BrokerError::Fetch {
                        code: e_code(&e),
                        message: e.to_string(),
                    })
                }
            }
        }
    }

    /// A targeted **pull** for one bank transaction (RF2 `email-pull`/`kauppatili-pull`).
    /// A single attempt — the Run, not the Broker, owns the wait/backoff, so a
    /// rate-limit surfaces as [`PullOutcome::RateLimited`] (→ `Effect::Wait`) rather than
    /// being retried in a loop here. A `404`-class no-match (the adapter's verdict) is
    /// [`PullOutcome::NotFound`], a legitimate empty result, not an error.
    pub fn pull(&self, envelope: &Envelope) -> Result<PullOutcome, BrokerError> {
        let adapter = self.resolve_kind(&envelope.service, AdapterKind::Pull)?;
        let cred = self.vault.handle(&envelope.service)?;
        match adapter.call(envelope, &cred) {
            Ok(AdapterResponse::Accepted { body, .. }) => Ok(PullOutcome::Found(body)),
            Ok(AdapterResponse::RecoveredAlreadyProcessed { .. }) => Err(BrokerError::Fetch {
                code: "UNEXPECTED_RECOVERY".into(),
                message: "a pull returned an already-processed verdict".into(),
            }),
            Err(AdapterError::RateLimited { retry_after_secs }) => {
                Ok(PullOutcome::RateLimited { retry_after_secs })
            }
            // The adapter's own no-match interpretation (M3 split: the Broker does not
            // read a status code; it only reads the adapter's semantic verdict).
            Err(AdapterError::Rejected { code, .. }) if code == "NOT_FOUND" => {
                Ok(PullOutcome::NotFound)
            }
            Err(e) => Err(BrokerError::Fetch {
                code: e_code(&e),
                message: e.to_string(),
            }),
        }
    }

    /// One outbound turn of a `consult-llm` stream (RF1). The `envelope.body` is the
    /// **projected** disclosure; the Broker sends exactly those bytes and returns the
    /// next chunk, never reading the `Final` body's domain meaning. A rate limit
    /// surfaces as [`ConsultTurn::RateLimited`].
    pub fn consult_turn(&self, envelope: &Envelope) -> Result<ConsultTurn, BrokerError> {
        let adapter = self
            .registry
            .get_stream(&envelope.service)
            .ok_or_else(|| BrokerError::UnknownService(envelope.service.clone()))?;
        let cred = self.vault.handle(&envelope.service)?;
        let req = RawRequest {
            service: envelope.service.clone(),
            operation: envelope.operation.clone(),
            idempotency_key: envelope.natural_key.clone(),
            body: envelope.body.clone(),
        };
        match adapter.turn(&req, &cred) {
            Ok(chunk) => Ok(ConsultTurn::Chunk(chunk)),
            Err(AdapterError::RateLimited { retry_after_secs }) => {
                Ok(ConsultTurn::RateLimited { retry_after_secs })
            }
            Err(e) => Err(BrokerError::Fetch {
                code: e_code(&e),
                message: e.to_string(),
            }),
        }
    }

    /// Read the current outbox entry for an effect (for reconciliation / display).
    pub fn outbox_entry(
        &self,
        service: &str,
        effect_key: &str,
    ) -> Result<Option<crate::outbox::OutboxEntry>, BrokerError> {
        Outbox::open(&self.home)?.load(service, effect_key)
    }

    /// Submit `envelope` for `effect_key`. Idempotent on a terminal success; a
    /// timeout yields [`RemoteOutcome::Unknown`] with **no** blind retry; a
    /// retryable transport failure retries within the [`RetryPolicy`], every attempt
    /// reusing the same `sent_hash`; the adapter's `RecoveredAlreadyProcessed` (its
    /// already-submitted interpretation) passes straight through as a terminal
    /// success.
    pub fn submit(
        &self,
        actor: &str,
        effect_key: &str,
        envelope: &Envelope,
    ) -> Result<RemoteOutcome, BrokerError> {
        // Kind guard first: a pull/read adapter must never reach the submit path, and
        // the refusal must happen before ANY outbox write (no `in_flight` row for an
        // effect that will never dispatch).
        let adapter = self.resolve_kind(&envelope.service, AdapterKind::Submit)?;
        let outbox = Outbox::open(&self.home)?;
        let sent_hash = envelope.sent_hash()?;

        // O-24 idempotency: a terminal success short-circuits a re-submit (the same
        // effect never fires twice). `Unknown` is intentionally NOT short-circuited.
        // The replay is bound to the effect's identity: an entry recorded for a
        // *different* `natural_key`, or a success over *different* sent bytes, is a
        // key collision, not a replay — refuse it loudly rather than assert a
        // payload was delivered that never was.
        let existing = outbox.load(&envelope.service, effect_key)?;
        if let Some(entry) = &existing {
            if entry.natural_key != envelope.natural_key {
                return Err(BrokerError::IdempotencyMismatch(format!(
                    "effect {effect_key} was bound to natural_key {:?}, incoming {:?}",
                    entry.natural_key, envelope.natural_key
                )));
            }
            // Every attempt of an effect must reuse the same sent bytes. A resubmit
            // after an `Unknown` (or any prior attempt) that changed the payload is a
            // key collision, not a replay — the authority would see different bytes
            // under the same key, defeating the already-processed recovery the
            // `Unknown` path leans on. All attempts share one hash, so checking the
            // first is sufficient.
            if let Some(first) = entry.attempts.first() {
                if first.sent_hash != sent_hash {
                    return Err(BrokerError::IdempotencyMismatch(format!(
                        "effect {effect_key} was bound to sent bytes {} (incoming {sent_hash}); \
                         a deliberate retry must reuse the original payload",
                        first.sent_hash
                    )));
                }
            }
            if let Some(success) = entry.terminal_success() {
                return Ok(replay(&success.outcome));
            }
        }

        let cred = self
            .vault
            .handle(&envelope.service)
            .map_err(|_| BrokerError::MissingCredential(envelope.service.clone()))?;
        let semantics = adapter.assurance();

        // `attempt_no` is the durable, effect-lifetime index (used for the audit row).
        // `local_attempt` is this submit call's retry budget — it starts at 1 so a
        // deliberate retry after prior `Unknown`s gets the FULL `RetryPolicy` again
        // (a timeout/`Unknown` must never consume a later call's transport retries).
        let mut attempt_no = existing.map(|e| e.next_attempt_no()).unwrap_or(1);
        let mut local_attempt = 1u32;

        loop {
            let provenance = Provenance {
                actor: actor.to_string(),
                service: envelope.service.clone(),
                operation: envelope.operation.clone(),
                natural_key: envelope.natural_key.clone(),
                sent_hash: sent_hash.clone(),
                delivery_semantics: semantics,
                attempt: attempt_no,
                ts: chrono::Utc::now().to_rfc3339(),
            };

            // Pre-dispatch intent: record the attempt durably BEFORE the network call,
            // so a crash in the dispatch→resolve window leaves an `in_flight` row that
            // recovers as `Unknown` (at-most-once, no lost/blind-retried effect).
            outbox.begin_attempt(
                &envelope.service,
                effect_key,
                &envelope.natural_key,
                attempt_no,
                &sent_hash,
                &provenance,
            )?;

            let (outcome, remote): (AttemptOutcome, LoopStep) = match adapter.call(envelope, &cred)
            {
                Ok(AdapterResponse::Accepted { receipt, .. }) => (
                    AttemptOutcome::Delivered {
                        receipt: receipt.clone(),
                    },
                    LoopStep::Return(RemoteOutcome::Delivered { receipt }),
                ),
                Ok(AdapterResponse::RecoveredAlreadyProcessed { receipt }) => (
                    AttemptOutcome::RecoveredAlreadyProcessed {
                        receipt: receipt.clone(),
                    },
                    LoopStep::Return(RemoteOutcome::RecoveredAlreadyProcessed { receipt }),
                ),
                // No definitive answer → Unknown, and STOP. Retrying blindly here is
                // the exact double-filing hazard the M3 gate forbids. The two ambiguous
                // causes stay distinct in the durable trail (a socket timeout vs. an
                // ambiguous HTTP answer) even though both collapse to the same
                // caller-facing `RemoteOutcome::Unknown`.
                Err(AdapterError::Timeout(detail)) => (
                    AttemptOutcome::Unknown {
                        code: "TIMEOUT".into(),
                        detail,
                    },
                    LoopStep::Return(RemoteOutcome::Unknown),
                ),
                Err(AdapterError::AmbiguousDelivery { code, detail }) => (
                    AttemptOutcome::Unknown { code, detail },
                    LoopStep::Return(RemoteOutcome::Unknown),
                ),
                // Safe-to-retry transport failures (incl. rate limiting) retry within
                // policy; the SAME sent_hash is reused every attempt.
                Err(AdapterError::RetryableTransport(detail)) => retryable_step(
                    local_attempt,
                    self.retry.max_attempts,
                    "RETRYABLE_TRANSPORT",
                    &detail,
                ),
                Err(AdapterError::RateLimited { retry_after_secs }) => retryable_step(
                    local_attempt,
                    self.retry.max_attempts,
                    "RATE_LIMITED",
                    &format!("retry_after={retry_after_secs:?}"),
                ),
                Err(AdapterError::Rejected { code, message }) => (
                    AttemptOutcome::Failed {
                        code: code.clone(),
                        detail: message.clone(),
                    },
                    LoopStep::Return(RemoteOutcome::Rejected { code, message }),
                ),
                Err(AdapterError::Unauthorized(detail)) => (
                    AttemptOutcome::Failed {
                        code: "UNAUTHORIZED".into(),
                        detail: detail.clone(),
                    },
                    LoopStep::Return(RemoteOutcome::Rejected {
                        code: "UNAUTHORIZED".into(),
                        message: detail,
                    }),
                ),
                Err(AdapterError::Malformed(detail)) => (
                    AttemptOutcome::Failed {
                        code: "MALFORMED".into(),
                        detail: detail.clone(),
                    },
                    LoopStep::Return(RemoteOutcome::Rejected {
                        code: "MALFORMED".into(),
                        message: detail,
                    }),
                ),
            };

            // Resolve the pre-dispatch intent to its terminal outcome (provenance was
            // already recorded by `begin_attempt`, so it is never overwritten).
            outbox.resolve_attempt(&envelope.service, effect_key, attempt_no, &outcome)?;

            match remote {
                LoopStep::Return(o) => return Ok(o),
                LoopStep::Retry => {
                    attempt_no += 1;
                    local_attempt += 1;
                    continue;
                }
            }
        }
    }
}

/// The Broker's next move after recording an attempt.
enum LoopStep {
    Return(RemoteOutcome),
    Retry,
}

/// Decide whether a retryable failure retries or exhausts, and build its attempt
/// outcome. Every retried attempt reuses the caller's `sent_hash` (set by `submit`).
fn retryable_step(
    attempt_no: u32,
    max_attempts: u32,
    code: &str,
    detail: &str,
) -> (AttemptOutcome, LoopStep) {
    if attempt_no < max_attempts {
        (
            AttemptOutcome::Failed {
                code: code.to_string(),
                detail: detail.to_string(),
            },
            LoopStep::Retry,
        )
    } else {
        (
            AttemptOutcome::Failed {
                code: "RETRY_EXHAUSTED".into(),
                detail: format!("{code}: {detail}"),
            },
            LoopStep::Return(RemoteOutcome::Rejected {
                code: "RETRY_EXHAUSTED".into(),
                message: format!("{code}: {detail}"),
            }),
        )
    }
}

/// A stable short code for an adapter error (used by `fetch`/`pull`/`consult_turn`'s
/// failure mapping into [`BrokerError::Fetch`]). For the variants that carry their own
/// structured cause — [`AdapterError::AmbiguousDelivery`] and [`AdapterError::Rejected`]
/// — the adapter's specific code (the `HTTP_<status>` / conflict code the adapter
/// produced) is preserved rather than flattened to a generic family label, so the
/// `broker-unknown-cause-observability` distinction survives onto the read/consult
/// error surface too; the rest map to a stable family label. The code strings
/// themselves are the adapter's, never re-derived here — this file reads no status.
fn e_code(e: &AdapterError) -> String {
    match e {
        AdapterError::Timeout(_) => "TIMEOUT".into(),
        AdapterError::AmbiguousDelivery { code, .. } => code.clone(),
        AdapterError::RetryableTransport(_) => "RETRYABLE_TRANSPORT".into(),
        AdapterError::Rejected { code, .. } => code.clone(),
        AdapterError::Unauthorized(_) => "UNAUTHORIZED".into(),
        AdapterError::Malformed(_) => "MALFORMED".into(),
        AdapterError::RateLimited { .. } => "RATE_LIMITED".into(),
    }
}

/// Reconstruct the caller-facing outcome from a replayed terminal-success attempt.
fn replay(outcome: &AttemptOutcome) -> RemoteOutcome {
    match outcome {
        AttemptOutcome::Delivered { receipt } => RemoteOutcome::Delivered {
            receipt: receipt.clone(),
        },
        AttemptOutcome::RecoveredAlreadyProcessed { receipt } => {
            RemoteOutcome::RecoveredAlreadyProcessed {
                receipt: receipt.clone(),
            }
        }
        // Only terminal successes are ever passed here (see `submit`'s
        // `terminal_success()` guard). A non-terminal outcome reaching this point is a
        // broken invariant on the idempotency-replay path — fail loudly, never silently
        // report `Unknown` for an effect the caller believes succeeded.
        other => unreachable!("replay called on non-terminal outcome: {other:?}"),
    }
}
