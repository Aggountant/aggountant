//! Ledger errors with stable machine-readable codes (`candidate.md §4`,
//! "Aggregaatti + repository (Ledger)").

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("ACCOUNT_NOT_FOUND: {0}")]
    AccountNotFound(String),
    #[error("VOUCHER_NUMBER_TAKEN: {0}")]
    VoucherNumberTaken(String),
    /// A primitive referenced a voucher number that is not a live sealed voucher
    /// (e.g. `attribute_adjustment` on an unknown number). The dual of
    /// `VOUCHER_NUMBER_TAKEN`, which means the number is *already* sealed.
    #[error("VOUCHER_NOT_FOUND: {0}")]
    VoucherNotFound(String),
    /// A sealed-voucher read encountered non-canonical bytes or disagreement between
    /// the hash-bound body and duplicated store columns. This is source corruption,
    /// never caller input.
    #[error("VOUCHER_READ_CORRUPT: {0}")]
    VoucherReadCorrupt(String),
    /// A public voucher-read query violated its bounded filter contract.
    #[error("INVALID_VOUCHER_READ: {0}")]
    InvalidVoucherRead(String),
    #[error("HASH_CHAIN_BROKEN: {0}")]
    HashChainBroken(String),
    #[error("CAS_CONFLICT: {0}")]
    CasConflict(String),
    /// A write was refused because the target period is `Closing` and the caller
    /// presented no matching `ClosingAuthority` (ADR-6 §2, F5).
    #[error("PERIOD_CLOSING: {0}")]
    PeriodClosing(String),
    /// A write was refused because the target period is `Sealed` (ADR-6 §2, F5).
    #[error("PERIOD_SEALED: {0}")]
    PeriodSealed(String),
    /// A Closing-period write (or a period transition) was attempted with a
    /// `--run-id` or a `ClosingAuthority` that does not own the period — the
    /// capability check failed (`candidate.md §4`, ADR-6 fencing).
    #[error("VOUCHER_NOT_IN_RUN: {0}")]
    VoucherNotInRun(String),
    /// A period CAS transition (`TransitionPeriod`/`SealPeriod`/reopen) lost its
    /// race or failed its guard (wrong from-state, or reopen after filing) — the
    /// CAS matched 0 rows, so the transition is refused with no partial state
    /// (ADR-6 §2/§5).
    #[error("PERIOD_TRANSITION_CONFLICT: {0}")]
    PeriodTransitionConflict(String),
    /// A VAT return was requested for a period that is not `Sealed` (the RF3/B2
    /// precondition). Filing requires a sealed period.
    #[error("PERIOD_NOT_SEALED: {0}")]
    PeriodNotSealed(String),
    #[error("ARTIFACT_NOT_FOUND: {0}")]
    ArtifactNotFound(String),
    /// A `natural_key` was reused with a different draft (`request_hash`
    /// mismatch) — a caller bug we refuse loudly rather than silently replay.
    #[error("IDEMPOTENCY_KEY_REUSE: {0}")]
    IdempotencyKeyReuse(String),
    /// The rejection could not be durably recorded, so we refuse to return a
    /// silent `Refused` (ADR-1 §Rejection log, guardrail 1).
    #[error("AUDIT_UNAVAILABLE: {0}")]
    AuditUnavailable(String),
    /// `aggountant init` was run against a home that already holds reference
    /// masterdata — refuse rather than clobber a diffable, review-gated tree.
    #[error("ALREADY_INITIALIZED: {0}")]
    AlreadyInitialized(String),
    /// A read touched a company home that has not been initialized yet
    /// (`aggountant init`).
    #[error("MASTERDATA_NOT_INITIALIZED: {0}")]
    MasterDataNotInitialized(String),
    /// The git-native reference masterdata failed its integrity cross-check
    /// (chart content and its content-hashed `chart_version` disagree).
    #[error("MASTERDATA_CORRUPT: {0}")]
    MasterDataCorrupt(String),
    /// A stateless approval was bound to one chart version, but another became active
    /// before the Ledger write boundary.
    #[error("CHART_VERSION_CHANGED: {0}")]
    ChartVersionChanged(String),
    /// An employee lookup (get/accumulate) found no record in the PII store (F7).
    /// A fiscal-year definition would change the year of something already
    /// recorded (issue `non-calendar-fiscal-year`).
    #[error("FISCAL_YEAR_CONFLICT: {0}")]
    FiscalYearConflict(String),
    #[error("EMPLOYEE_NOT_FOUND: {0}")]
    EmployeeNotFound(String),
    /// A statement/erittelyt projection folded for a close-package hash failed in the
    /// Domain layer (an unmapped account, a non-EUR posting, a kind/nature mismatch).
    /// Wraps the [`aggountant_domain::DomainError`] so its own stable code (`STATEMENTS`,
    /// `DEPRECIATION`, …) is preserved rather than flattened to a string.
    #[error("{0}")]
    Statements(#[from] aggountant_domain::DomainError),
    /// A value that must be a 64-char lowercase-hex sha256 digest — a bound
    /// `statements_hash`, or a close-package component hash — was malformed. A structural
    /// input-format failure, distinct from a store error or a projection failure.
    #[error("INVALID_HASH: {0}")]
    InvalidHash(String),
    #[error("WIRE: {0}")]
    Wire(#[from] aggountant_wire::WireError),
    #[error("MODEL: {0}")]
    Model(#[from] aggountant_model::ModelError),
    #[error("STORE: {0}")]
    Store(String),
}

impl LedgerError {
    pub fn error_code(&self) -> &'static str {
        match self {
            LedgerError::AccountNotFound(_) => "ACCOUNT_NOT_FOUND",
            LedgerError::VoucherNumberTaken(_) => "VOUCHER_NUMBER_TAKEN",
            LedgerError::VoucherNotFound(_) => "VOUCHER_NOT_FOUND",
            LedgerError::VoucherReadCorrupt(_) => "VOUCHER_READ_CORRUPT",
            LedgerError::InvalidVoucherRead(_) => "INVALID_VOUCHER_READ",
            LedgerError::HashChainBroken(_) => "HASH_CHAIN_BROKEN",
            LedgerError::CasConflict(_) => "CAS_CONFLICT",
            LedgerError::PeriodClosing(_) => "PERIOD_CLOSING",
            LedgerError::PeriodSealed(_) => "PERIOD_SEALED",
            LedgerError::VoucherNotInRun(_) => "VOUCHER_NOT_IN_RUN",
            LedgerError::PeriodTransitionConflict(_) => "PERIOD_TRANSITION_CONFLICT",
            LedgerError::PeriodNotSealed(_) => "PERIOD_NOT_SEALED",
            LedgerError::ArtifactNotFound(_) => "ARTIFACT_NOT_FOUND",
            LedgerError::IdempotencyKeyReuse(_) => "IDEMPOTENCY_KEY_REUSE",
            LedgerError::AuditUnavailable(_) => "AUDIT_UNAVAILABLE",
            LedgerError::AlreadyInitialized(_) => "ALREADY_INITIALIZED",
            LedgerError::MasterDataNotInitialized(_) => "MASTERDATA_NOT_INITIALIZED",
            LedgerError::MasterDataCorrupt(_) => "MASTERDATA_CORRUPT",
            LedgerError::ChartVersionChanged(_) => "CHART_VERSION_CHANGED",
            LedgerError::FiscalYearConflict(_) => "FISCAL_YEAR_CONFLICT",
            LedgerError::EmployeeNotFound(_) => "EMPLOYEE_NOT_FOUND",
            LedgerError::Statements(e) => e.error_code(),
            LedgerError::InvalidHash(_) => "INVALID_HASH",
            LedgerError::Wire(_) => "NON_CANONICAL_WIRE",
            LedgerError::Model(e) => e.error_code(),
            LedgerError::Store(_) => "STORE_ERROR",
        }
    }
}

impl From<rusqlite::Error> for LedgerError {
    fn from(e: rusqlite::Error) -> Self {
        LedgerError::Store(e.to_string())
    }
}
