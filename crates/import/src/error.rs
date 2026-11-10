//! Import-crate error taxonomy with machine-readable codes, in the same style as
//! `ModelError`/`LedgerError` (`AGENTS-AI-FIRST-CLI.md`): every variant maps to a
//! stable `error_code()` the CLI surfaces to stderr.

use thiserror::Error;

use aggountant_domain::DomainError;
use aggountant_model::ModelError;

#[derive(Debug, Error)]
pub enum ImportError {
    /// The camt.053 document is not well-formed XML, or is missing a required
    /// element/attribute the ISO 20022 statement shape mandates (account IBAN,
    /// `Ntry` amount/direction/date, or a per-entry reference for the dedup key).
    #[error("INVALID_CAMT053: {0}")]
    Camt053(String),
    /// A scalar (amount, currency, date, account id) failed the Model layer's
    /// validation while decoding an entry or projecting a draft.
    #[error("{0}")]
    Model(#[from] ModelError),
    /// A Domain function refused the projection — a VAT expansion (`vat_expand`) or
    /// the draft builder (`build_draft`) rejected the assembled voucher (unbalanced,
    /// unresolvable rate, structural VAT guard, …). Its `error_code` is carried
    /// through verbatim so the CLI surfaces the Domain reason.
    #[error("{0}")]
    Domain(#[from] DomainError),
    /// A parsed entry has no account/VAT assignment in the skill-supplied input —
    /// the CLI never auto-assigns accounts (`phase4-imports` determinism/judgment
    /// rule), so an unassigned entry is refused rather than guessed or skipped.
    #[error("MISSING_ASSIGNMENT: {0}")]
    MissingAssignment(String),
    /// An assignment is internally inconsistent — it sets both a `vat_code` and a
    /// `vat_treatment` (mutually exclusive), or carries a purchase-side
    /// `vat_treatment` on a money-in entry. Refused before any write.
    #[error("INVALID_ASSIGNMENT: {0}")]
    InvalidAssignment(String),
    /// A gross bank allocation carries a legacy VAT code whose dated rate is
    /// non-zero. Such an amount is not a VAT base and must not reach the seal.
    #[error("UNSAFE_LEGACY_VAT_CODE: {0}")]
    UnsafeLegacyVatCode(String),
    /// VAT expansion needs the source document date; the bank posting date is not
    /// a safe substitute across a statutory rate boundary.
    #[error("MISSING_DOCUMENT_DATE: {0}")]
    MissingDocumentDate(String),
}

impl ImportError {
    pub fn error_code(&self) -> &'static str {
        match self {
            ImportError::Camt053(_) => "INVALID_CAMT053",
            ImportError::Model(e) => e.error_code(),
            ImportError::Domain(e) => e.error_code(),
            ImportError::MissingAssignment(_) => "MISSING_ASSIGNMENT",
            ImportError::InvalidAssignment(_) => "INVALID_ASSIGNMENT",
            ImportError::UnsafeLegacyVatCode(_) => "UNSAFE_LEGACY_VAT_CODE",
            ImportError::MissingDocumentDate(_) => "MISSING_DOCUMENT_DATE",
        }
    }
}
