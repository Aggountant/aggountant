//! Process-engine errors with stable machine-readable codes (`candidate.md §4`
//! "Prosessi (Process)"). Ledger errors pass through transparently so a caller
//! branching on `error_code` sees the underlying fiscal code (e.g.
//! `PERIOD_TRANSITION_CONFLICT`, `VOUCHER_NOT_IN_RUN`).

use thiserror::Error;

use aggountant_ledger::LedgerError;

#[derive(Debug, Error)]
pub enum ProcessError {
    /// The run directory does not exist (`run show`/`resume` on an unknown id).
    #[error("RUN_NOT_FOUND: {0}")]
    RunNotFound(String),
    /// Resume/answer refused because the run was started under a different
    /// workflow version (`candidate.md §9`). The user may `run migrate --confirm`.
    #[error("WORKFLOW_VERSION_MISMATCH: {0}")]
    WorkflowVersionMismatch(String),
    /// Another driver holds the run's lease (`candidate.md §4`, `RUN_LEASED`).
    #[error("RUN_LEASED: {0}")]
    RunLeased(String),
    /// The referenced gate is not open (never opened, or already answered).
    #[error("GATE_NOT_OPEN: {0}")]
    GateNotOpen(String),
    /// The gate answer was not one of the accepted values for its kind.
    #[error("INVALID_ANSWER: {0}")]
    InvalidAnswer(String),
    /// The run's journal is internally inconsistent (a corrupt/forged event
    /// chain, or a fold that cannot be reconciled).
    #[error("JOURNAL_CORRUPT: {0}")]
    JournalCorrupt(String),
    /// The VAT figures supplied to `file-vat` disagree with the aggregates
    /// derived from the period's sealed vouchers (issue
    /// `vat-figures-from-sealed-ledger`). The filing is refused: the approval must
    /// bind to figures the ledger itself can reproduce, not unverified input.
    #[error("VAT_FIGURES_MISMATCH: {0}")]
    VatFiguresMismatch(String),
    /// `file-vat status` could not read the authority's view of a filed return
    /// (network, credential, readiness or response failure). This never means
    /// "not filed": the stored receipt still proves the filing.
    #[error("VAT_STATUS_UNKNOWN: {0}")]
    VatStatusUnknown(String),
    /// `capture-receipt`'s approved draft would record a deductible input-VAT
    /// amount different from the deterministic eligibility assessment. The payload
    /// is never rewritten after approval; the caller must start a new Run with a
    /// corrected draft.
    #[error("CAPTURE_VAT_MISMATCH: {0}")]
    CaptureVatMismatch(String),
    /// An ordinary capture omitted or blanked a source fact required to make a
    /// treatment proposal reviewable.
    #[error("CAPTURE_FACT_REQUIRED: {0}")]
    CaptureFactRequired(String),
    /// The proposal explicitly says facts are unresolved; never coerce that to NoVat.
    #[error("CAPTURE_CLARIFICATION_REQUIRED: {0}")]
    CaptureClarificationRequired(String),
    /// The proposal names a treatment outside the ordinary capture slice.
    #[error("CAPTURE_UNSUPPORTED_TREATMENT: {0}")]
    CaptureUnsupportedTreatment(String),
    /// The approved stateless preparation binding no longer matches the projection.
    #[error("CAPTURE_PREPARATION_CHANGED: {0}")]
    CapturePreparationChanged(String),
    /// A fiscal effect failed; the Ledger's own code is preserved.
    #[error("{0}")]
    Ledger(#[from] LedgerError),
    /// A company-knowledge write or query is malformed or inconsistent.
    #[error("INVALID_KNOWLEDGE: {0}")]
    InvalidKnowledge(String),
    /// The requested company-local fact or revision does not exist.
    #[error("KNOWLEDGE_NOT_FOUND: {0}")]
    KnowledgeNotFound(String),
    /// Persisted company-knowledge history is malformed or inconsistent.
    #[error("KNOWLEDGE_HISTORY_CORRUPT: {0}")]
    KnowledgeHistoryCorrupt(String),
    /// A local I/O or serialization failure in the git-native process store.
    #[error("PROCESS_STORE: {0}")]
    Store(String),
    /// A `close-period` was refused because the period has an open obligation (F13,
    /// `candidate.md §4` process invariant `OPEN_OBLIGATION`). The driver must resolve
    /// the obligation first (e.g. supply the missing ajopäiväkirja).
    #[error("OPEN_OBLIGATION: {0}")]
    OpenObligation(String),
    /// A `close-year` step was refused because a precondition is unmet: a subperiod is
    /// not `Sealed`, must-seal intake/obligations are unresolved, or a D7 correction is
    /// unsealed. The message lists the blockers with stable ids (S6, ADR-5).
    #[error("CLOSE_YEAR_BLOCKED: {0}")]
    CloseYearBlocked(String),
}

impl ProcessError {
    pub fn error_code(&self) -> &'static str {
        match self {
            ProcessError::RunNotFound(_) => "RUN_NOT_FOUND",
            ProcessError::WorkflowVersionMismatch(_) => "WORKFLOW_VERSION_MISMATCH",
            ProcessError::RunLeased(_) => "RUN_LEASED",
            ProcessError::GateNotOpen(_) => "GATE_NOT_OPEN",
            ProcessError::InvalidAnswer(_) => "INVALID_ANSWER",
            ProcessError::JournalCorrupt(_) => "JOURNAL_CORRUPT",
            ProcessError::VatFiguresMismatch(_) => "VAT_FIGURES_MISMATCH",
            ProcessError::VatStatusUnknown(_) => "VAT_STATUS_UNKNOWN",
            ProcessError::CaptureVatMismatch(_) => "CAPTURE_VAT_MISMATCH",
            ProcessError::CaptureFactRequired(_) => "CAPTURE_FACT_REQUIRED",
            ProcessError::CaptureClarificationRequired(_) => "CAPTURE_CLARIFICATION_REQUIRED",
            ProcessError::CaptureUnsupportedTreatment(_) => "CAPTURE_UNSUPPORTED_TREATMENT",
            ProcessError::CapturePreparationChanged(_) => "CAPTURE_PREPARATION_CHANGED",
            ProcessError::InvalidKnowledge(_) => "INVALID_KNOWLEDGE",
            ProcessError::KnowledgeNotFound(_) => "KNOWLEDGE_NOT_FOUND",
            ProcessError::KnowledgeHistoryCorrupt(_) => "KNOWLEDGE_HISTORY_CORRUPT",
            ProcessError::Ledger(e) => e.error_code(),
            ProcessError::Store(_) => "PROCESS_STORE",
            ProcessError::OpenObligation(_) => "OPEN_OBLIGATION",
            ProcessError::CloseYearBlocked(_) => "CLOSE_YEAR_BLOCKED",
        }
    }

    /// Error family for the CLI's `error_kind` field: a `system` error is an
    /// infrastructure failure (exit 2); everything else is a `user`/logic error.
    pub fn kind(&self) -> &'static str {
        match self {
            ProcessError::Store(_)
            | ProcessError::VatStatusUnknown(_)
            | ProcessError::KnowledgeHistoryCorrupt(_)
            | ProcessError::Ledger(LedgerError::AuditUnavailable(_) | LedgerError::Store(_)) => {
                "system"
            }
            _ => "user",
        }
    }
}
