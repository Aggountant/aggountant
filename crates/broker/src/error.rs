//! The `AdapterError` taxonomy (F11) and the Broker's own errors. The taxonomy is
//! the whole point of the M3 "is the `AdapterError` taxonomy sufficient?" question:
//! it must let the **Broker** decide *transport* policy (retry / give up / declare
//! the outcome unknown) **without** ever reading a domain status code — that
//! interpretation belongs to the adapter (`AdapterResponse`), not here.

use thiserror::Error;

use crate::adapter::AdapterKind;

/// What one adapter attempt can go wrong as. The Broker maps each variant to a
/// transport policy; it never inspects an HTTP status or a domain body.
///
/// The two *ambiguous-outcome* variants — [`AdapterError::Timeout`] (no answer at
/// all) and [`AdapterError::AmbiguousDelivery`] (an answer we cannot interpret as
/// definitive) — are the ones that become [`crate::RemoteOutcome::Unknown`]; the
/// rest are definitive (a negative answer). An ambiguous outcome must **not**
/// trigger a blind retry, because the request may in fact have been processed. The
/// two are kept **separate** (rather than both folded into `Timeout(String)`) so an
/// operator triaging a parked `Unknown` in the outbox — and any log/metric — can
/// tell a socket timeout from a proxy `5xx` from an unverifiable `409`
/// (`broker-unknown-cause-observability`).
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AdapterError {
    /// **No answer at all**: the request timed out or the connection dropped after
    /// send, so no bytes came back. The remote may or may not have processed the
    /// request → `Unknown`. Never auto-retried by the Broker. Contrast
    /// [`AdapterError::AmbiguousDelivery`], where an answer *did* arrive but is
    /// itself ambiguous.
    #[error("ADAPTER_TIMEOUT: {0}")]
    Timeout(String),
    /// **An answer arrived, but it is ambiguous** as to whether the effect committed:
    /// a proxy `5xx` that may sit *after* an upstream commit, or a `409` whose
    /// idempotency-key identity we cannot verify. Like [`AdapterError::Timeout`] it
    /// maps to `Unknown` and is **never** auto-retried, but it is a distinct cause:
    /// keeping it separate is what lets the outbox trail / logs / metrics distinguish
    /// "no answer" from "ambiguous answer" (`broker-unknown-cause-observability`).
    /// `code` is a stable short cause (e.g. `HTTP_503`, `HTTP_409`); `detail` is the
    /// human-facing elaboration.
    #[error("ADAPTER_AMBIGUOUS[{code}]: {detail}")]
    AmbiguousDelivery { code: String, detail: String },
    /// A transport-level failure that is *safe* to retry (connection refused before
    /// the request was delivered, or a `5xx` on an idempotent **pull**). Retried within
    /// the Broker's [`crate::RetryPolicy`], every attempt reusing the same `sent_hash`.
    /// Note a `5xx` on an `AtMostOnce` submit is NOT this — it is ambiguous and surfaces
    /// as [`AdapterError::AmbiguousDelivery`] (→ `Unknown`), because a proxy `5xx` may
    /// follow an upstream commit.
    #[error("ADAPTER_RETRYABLE: {0}")]
    RetryableTransport(String),
    /// The authority **definitively rejected** the request (validation error,
    /// business rule). Terminal — never retried.
    #[error("ADAPTER_REJECTED[{code}]: {message}")]
    Rejected { code: String, message: String },
    /// Credential / authorization failure. Terminal for this attempt.
    #[error("ADAPTER_UNAUTHORIZED: {0}")]
    Unauthorized(String),
    /// The response was not parseable into the adapter's contract (a malformed body
    /// where a definitive one was expected). Terminal.
    #[error("ADAPTER_MALFORMED: {0}")]
    Malformed(String),
    /// The authority rate-limited the request (`429`). Carries the optional
    /// `Retry-After` the authority advertised — the `rate-limit-surface-adr` touch.
    /// Treated by the Broker as retryable within policy.
    #[error("ADAPTER_RATE_LIMITED (retry_after={retry_after_secs:?})")]
    RateLimited { retry_after_secs: Option<u64> },
}

/// Broker-level failures that are not an adapter's fault (a misconfigured
/// registry, a missing credential, an outbox I/O error).
#[derive(Debug, Error)]
pub enum BrokerError {
    /// No adapter is registered for the envelope's `service`.
    #[error("UNKNOWN_SERVICE: {0}")]
    UnknownService(String),
    /// The resolved adapter's kind does not match the operation invoked — a submit
    /// adapter reached through `fetch`/`pull`, or a pull/read adapter through
    /// `submit`. Refused *before* any dispatch or outbox write, so a write can never
    /// enter the retrying read path (the double-submit hazard this guards). Carries
    /// the `service` and both kinds structurally so a caller can inspect the mismatch
    /// without parsing a message string.
    #[error(
        "ADAPTER_KIND_MISMATCH: service {service:?} is a {actual:?} adapter, but a \
         {expected:?} operation was invoked"
    )]
    AdapterKindMismatch {
        service: String,
        expected: AdapterKind,
        actual: AdapterKind,
    },
    /// The Vault holds no credential for the service.
    #[error("MISSING_CREDENTIAL: {0}")]
    MissingCredential(String),
    /// A submit reused an effect key that the outbox already bound to a different
    /// `natural_key` or a success over different sent bytes — a key collision, not
    /// an idempotent replay.
    #[error("IDEMPOTENCY_MISMATCH: {0}")]
    IdempotencyMismatch(String),
    /// The durable outbox could not be read or written.
    #[error("OUTBOX_UNAVAILABLE: {0}")]
    OutboxUnavailable(String),
    /// The envelope could not be canonically encoded for hashing.
    #[error("ENVELOPE_ENCODE: {0}")]
    EnvelopeEncode(String),
    /// A read/pull (`fetch`) could not obtain a definitive response (timeout or a
    /// definitive rejection). Unlike a submit, a read has no `Unknown` ambiguity —
    /// it is simply retried later.
    #[error("FETCH_FAILED[{code}]: {message}")]
    Fetch { code: String, message: String },
}

impl BrokerError {
    pub fn error_code(&self) -> &'static str {
        match self {
            BrokerError::UnknownService(_) => "UNKNOWN_SERVICE",
            BrokerError::AdapterKindMismatch { .. } => "ADAPTER_KIND_MISMATCH",
            BrokerError::MissingCredential(_) => "MISSING_CREDENTIAL",
            BrokerError::IdempotencyMismatch(_) => "IDEMPOTENCY_MISMATCH",
            BrokerError::OutboxUnavailable(_) => "OUTBOX_UNAVAILABLE",
            BrokerError::EnvelopeEncode(_) => "ENVELOPE_ENCODE",
            BrokerError::Fetch { .. } => "FETCH_FAILED",
        }
    }
}
