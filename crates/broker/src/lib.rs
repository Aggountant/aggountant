//! # aggountant-broker (F11 + F12 + F9 outbox)
//!
//! The **edge substrate**: the boundary between aggountant's deterministic core and
//! the outside authorities (Vero-API, verotili, the bank). This is the crate the
//! S3 edge-spike exists to stress — v3's boldest edge assumption is that
//! provenance and outbox uncertainty survive authority submits. Generic synthetic
//! tests retain an explicit idempotency/409 contract; the direct Vero VAT adapter
//! deliberately does not.
//!
//! ## The six O-24 modules
//! The Broker is one object composed of six concerns (`candidate.md §6`, O-24):
//! - **cred** — [`Vault`] / [`CredentialHandle`] (secrets never reach an audit path).
//! - **retry** — [`RetryPolicy`] (retry *retryable transport* only; never a timeout).
//! - **prov** — per-attempt [`Provenance`] (`sent_hash`, attempt no, semantics).
//! - **assurance** — [`DeliverySemantics`] declared per adapter, recorded per send.
//! - **idempotency** — outbox terminal-success replay; `Unknown` is *not* replayed.
//! - **redaction** — caller-declared [`RedactionPolicy`] the Broker enforces.
//!
//! ## The adapter-vs-Broker split (M3)
//! The Broker owns *transport* policy and never reads a status code. The **adapter**
//! owns *domain interpretation* — critically, mapping a `409 already_submitted` to
//! [`AdapterResponse::RecoveredAlreadyProcessed`]. See [`adapter`] and [`adapters`].
//!
//! ## Three interaction forms
//! [`RequestResponse`] (concrete in v1), [`Subscription`] (inbound push — none in
//! v1; `push-forgery-adr`), [`Stream`] (`consult-llm`, S5). See [`adapter`].

#![forbid(unsafe_code)]

pub mod adapter;
pub mod adapters;
pub mod broker;
pub mod contract;
mod envelope;
mod error;
pub mod outbox;
mod vault;
pub mod vero_filing;
pub mod vero_vat_api;

pub use vault::{CredentialHandle, Vault};

pub use adapter::{
    AdapterKind, AdapterResponse, ConsultChunk, RawRequest, RawResponse, RequestResponse,
    ScriptedTransport, Stream, Subscription, Transport, TransportError,
};
pub use adapters::{
    classify_conflict_scripted, ConflictClassifier, ConflictVerdict, ConsultLlmAdapter,
    EmailPullAdapter, KauppatiliPullAdapter, PankkiMaksuAdapter, SyntheticSubmitAdapter,
    TulorekisteriAdapter, VeroVerotiliAdapter, SVC_CONSULT_LLM, SVC_EMAIL_PULL,
    SVC_KAUPPATILI_PULL, SVC_PANKKI_MAKSU, SVC_SYNTHETIC_SUBMIT, SVC_TULOREKISTERI,
    SVC_VERO_VEROTILI,
};
pub use broker::{Broker, ConsultTurn, PullOutcome, Registry, RetryPolicy};
pub use envelope::{
    ContinuationToken, DeliverySemantics, Envelope, Provenance, RedactionPolicy, RemoteOutcome,
};
pub use error::{AdapterError, BrokerError};
pub use outbox::{Attempt, AttemptOutcome, Outbox, OutboxEntry};
pub use vero_filing::{
    evaluate_vero_readiness, profile_path as vero_profile_path, read_vero_profile,
    write_vero_profile, TestVerificationEvidence, VeroAttestations, VeroCertificateMetadata,
    VeroCredentialTarget, VeroFilingMode, VeroFilingProfile, VeroReadinessBlocker,
    VeroReadinessPurpose, VeroReadinessReport, VeroReadinessWarning, VERO_PRODUCTION_BASE_URL,
    VERO_TEST_BASE_URL,
};
pub use vero_vat_api::{
    project_file_vat_return, FileVatReceipt, FileVatReturn, FiledVatReturn, GetVatPeriodsResponse,
    ScriptedVeroFileVatReturnAdapter, ScriptedVeroFiledVatReturnAdapter,
    ScriptedVeroVatPeriodsAdapter, VeroFileVatReturnAdapter, VeroFiledVatReturnAdapter,
    VeroVatPeriodsAdapter, SVC_VERO_FILED_VAT_RETURN, SVC_VERO_FILE_VAT_RETURN,
    SVC_VERO_VAT_PERIODS,
};

pub mod tulorekisteri;
pub mod tulorekisteri_xml;
