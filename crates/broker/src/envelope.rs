//! The request `Envelope`, the delivery-semantics + outcome vocabulary, the
//! per-attempt `Provenance` (F9), the caller-declared `RedactionPolicy` (the
//! `redaction-strategy-adr` decision), and the `ContinuationToken` used by the
//! paged interaction forms.
//!
//! The **`sent_hash`** is the anchor of the whole S3 edge story: it is the SHA-256
//! of the *canonical bytes that get sent* (the aggountant JCS profile, `ADR-2`), so an
//! approval that binds to `sent_hash` binds to the exact envelope, and every retry
//! of the same request carries the same `sent_hash` regardless of attempt count.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::BrokerError;

/// How many times, and with what duplication risk, an operation may reach the
/// authority (O-24 "assurance"). The adapter declares its own; the Broker records
/// it in per-attempt provenance so the audit trail says how the bytes were sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliverySemantics {
    /// May be delivered more than once; the *authority's* idempotency key
    /// (our `natural_key` / `sent_hash`) dedupes duplicates (a 409 recovery).
    AtLeastOnce,
    /// Must not be re-sent on an unknown outcome without human/driver intent (a
    /// filing submit: a blind retry risks a double filing).
    AtMostOnce,
}

/// The caller-declared redaction policy the Broker enforces when producing a
/// loggable / consult-visible view of an envelope (`redaction-strategy-adr`:
/// **caller-declared, Broker-enforced**, not a per-service Broker plugin). The
/// caller names the body fields that carry secrets/PII; the Broker never has to
/// know a service's schema to redact it safely.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionPolicy {
    /// Top-level body keys whose values must be replaced with `"[REDACTED]"` in any
    /// view that leaves the trust boundary (logs, `consult-llm`).
    pub secret_fields: Vec<String>,
}

/// One request to an authority. The `body` is the domain payload; `service` +
/// `operation` route it; `natural_key` is the source-based idempotency key the
/// authority (and our outbox) dedupe on.
#[derive(Debug, Clone, Serialize)]
pub struct Envelope {
    pub service: String,
    pub operation: String,
    pub natural_key: String,
    pub body: serde_json::Value,
    /// Not part of the sent bytes — a local policy for producing redacted views.
    #[serde(skip)]
    pub redaction: RedactionPolicy,
}

/// The subset of an [`Envelope`] that is actually transmitted, in a fixed field
/// order — the pre-image of `sent_hash`. Redaction policy is deliberately excluded
/// (it is local, not sent).
#[derive(Serialize)]
struct EnvelopeWire<'a> {
    service: &'a str,
    operation: &'a str,
    natural_key: &'a str,
    body: &'a serde_json::Value,
}

impl Envelope {
    pub fn new(
        service: impl Into<String>,
        operation: impl Into<String>,
        natural_key: impl Into<String>,
        body: serde_json::Value,
    ) -> Self {
        Envelope {
            service: service.into(),
            operation: operation.into(),
            natural_key: natural_key.into(),
            body,
            redaction: RedactionPolicy::default(),
        }
    }

    pub fn with_redaction(mut self, redaction: RedactionPolicy) -> Self {
        self.redaction = redaction;
        self
    }

    /// The canonical bytes that get sent (aggountant JCS profile, `ADR-2`).
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, BrokerError> {
        aggountant_wire::canonical_bytes(&EnvelopeWire {
            service: &self.service,
            operation: &self.operation,
            natural_key: &self.natural_key,
            body: &self.body,
        })
        .map_err(|e| BrokerError::EnvelopeEncode(e.to_string()))
    }

    /// `SHA-256` over the canonical sent bytes. Stable across attempts; the anchor
    /// a `Preview` gate binds an approval to.
    pub fn sent_hash(&self) -> Result<String, BrokerError> {
        let bytes = self.canonical_bytes()?;
        let mut h = Sha256::new();
        h.update(&bytes);
        Ok(hex::encode(h.finalize()))
    }

    /// A view of the body safe to log / send to `consult-llm`: every declared
    /// secret field is replaced with `"[REDACTED]"`. The Broker enforces this so a
    /// caller cannot accidentally leak a credential through an observability path.
    pub fn redacted_body(&self) -> serde_json::Value {
        let mut body = self.body.clone();
        if let Some(map) = body.as_object_mut() {
            for field in &self.redaction.secret_fields {
                if map.contains_key(field) {
                    map.insert(
                        field.clone(),
                        serde_json::Value::String("[REDACTED]".into()),
                    );
                }
            }
        }
        body
    }
}

/// The outcome of a Broker `submit`, as seen by the caller (the `file-vat` Run).
/// Note the deliberate asymmetry: [`RemoteOutcome::Unknown`] is **not** a failure
/// and **not** a success — it is the honest "we don't know", which the caller must
/// resolve (reconcile / a deliberate retry), never paper over with a blind resend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RemoteOutcome {
    /// The authority accepted the request this attempt (a fresh success).
    Delivered { receipt: String },
    /// The authority reported the request was **already processed** (the adapter
    /// interpreted a `409 already_submitted`) — an idempotent recovery, not a fresh
    /// send. Terminal-success-equivalent.
    RecoveredAlreadyProcessed { receipt: String },
    /// No definitive answer (a timeout). The request may or may not have landed.
    Unknown,
    /// The authority definitively rejected the request. Terminal failure.
    Rejected { code: String, message: String },
}

impl RemoteOutcome {
    /// A terminal *success* the outbox may replay for idempotency (`Delivered` /
    /// `RecoveredAlreadyProcessed`). `Unknown` and `Rejected` are **not** replayed:
    /// an `Unknown` deliberately leaves the door open for a driver-initiated retry.
    pub fn is_terminal_success(&self) -> bool {
        matches!(
            self,
            RemoteOutcome::Delivered { .. } | RemoteOutcome::RecoveredAlreadyProcessed { .. }
        )
    }

    /// The receipt string of a successful outcome, if any.
    pub fn receipt(&self) -> Option<&str> {
        match self {
            RemoteOutcome::Delivered { receipt }
            | RemoteOutcome::RecoveredAlreadyProcessed { receipt } => Some(receipt),
            _ => None,
        }
    }
}

/// Per-attempt provenance (F9): recorded once per attempt and **never
/// overwritten**, so the audit trail keeps every send — its `sent_hash`, its
/// attempt index, and how it was sent — not just the last one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub actor: String,
    pub service: String,
    pub operation: String,
    pub natural_key: String,
    pub sent_hash: String,
    pub delivery_semantics: DeliverySemantics,
    pub attempt: u32,
    pub ts: String,
}

/// An opaque cursor for the paged interaction forms (`Subscription`/`Stream`). Kept
/// minimal in this slice — no inbound adapter ships in v1 — but present so the
/// three-interaction-form contract is expressible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationToken(pub String);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sent_hash_is_stable_and_order_independent() {
        let a = Envelope::new("svc", "op", "nk", json!({"b": 1, "a": "x"}));
        let b = Envelope::new("svc", "op", "nk", json!({"a": "x", "b": 1}));
        assert_eq!(a.sent_hash().unwrap(), b.sent_hash().unwrap());
        // A different body → a different hash.
        let c = Envelope::new("svc", "op", "nk", json!({"a": "y", "b": 1}));
        assert_ne!(a.sent_hash().unwrap(), c.sent_hash().unwrap());
    }

    #[test]
    fn redaction_replaces_only_declared_fields() {
        let env = Envelope::new(
            "svc",
            "op",
            "nk",
            json!({"iban": "FI..", "amount": "10.00"}),
        )
        .with_redaction(RedactionPolicy {
            secret_fields: vec!["iban".into()],
        });
        let red = env.redacted_body();
        assert_eq!(red["iban"], json!("[REDACTED]"));
        assert_eq!(red["amount"], json!("10.00"));
        // The envelope's own sent bytes are unaffected by the redaction policy.
        assert_eq!(env.body["iban"], json!("FI.."));
    }
}
