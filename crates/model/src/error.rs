//! Model-layer errors. Each carries a stable machine-readable `error_code`
//! (see `candidate.md §4`, the "Paikalliset (Model)" invariant table).

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ModelError {
    #[error("INVALID_MONEY: {0}")]
    InvalidMoney(String),
    #[error("INVALID_CURRENCY: {0}")]
    InvalidCurrency(String),
    #[error("INVALID_ACCOUNT_ID: {0}")]
    InvalidAccountId(String),
    #[error("INVALID_DATE: {0}")]
    InvalidDate(String),
    #[error("INVALID_HASH: {0}")]
    InvalidHash(String),
    #[error("INVALID_VAT_CODE: {0}")]
    InvalidVatCode(String),
    #[error("INVALID_PERIOD_ID: {0}")]
    InvalidPeriodId(String),
    #[error("UNBALANCED_VOUCHER: {0}")]
    UnbalancedVoucher(String),
    #[error("INSUFFICIENT_POSTINGS: {0}")]
    InsufficientPostings(String),
    #[error("MIXED_CURRENCY: {0}")]
    MixedCurrency(String),
    #[error("EMPTY_DESCRIPTION: {0}")]
    EmptyDescription(String),
    #[error("INVALID_VOUCHER_BODY: {0}")]
    InvalidVoucherBody(String),
    #[error("INVALID_OPENING_BALANCE: {0}")]
    InvalidOpeningBalance(String),
    #[error("INVALID_VAT_EXPANSION: {0}")]
    InvalidVatExpansion(String),
    #[error("INVALID_IBAN: {0}")]
    InvalidIban(String),
    #[error("INVALID_PERSON_ID: {0}")]
    InvalidPersonId(String),
    #[error("INVALID_VERO_FILING_IDENTITY: {0}")]
    InvalidVeroFilingIdentity(String),
}

impl ModelError {
    /// Stable, machine-readable error code (the token before the colon).
    pub fn error_code(&self) -> &'static str {
        match self {
            ModelError::InvalidMoney(_) => "INVALID_MONEY",
            ModelError::InvalidCurrency(_) => "INVALID_CURRENCY",
            ModelError::InvalidAccountId(_) => "INVALID_ACCOUNT_ID",
            ModelError::InvalidDate(_) => "INVALID_DATE",
            ModelError::InvalidHash(_) => "INVALID_HASH",
            ModelError::InvalidVatCode(_) => "INVALID_VAT_CODE",
            ModelError::InvalidPeriodId(_) => "INVALID_PERIOD_ID",
            ModelError::UnbalancedVoucher(_) => "UNBALANCED_VOUCHER",
            ModelError::InsufficientPostings(_) => "INSUFFICIENT_POSTINGS",
            ModelError::MixedCurrency(_) => "MIXED_CURRENCY",
            ModelError::EmptyDescription(_) => "EMPTY_DESCRIPTION",
            ModelError::InvalidVoucherBody(_) => "INVALID_VOUCHER_BODY",
            ModelError::InvalidOpeningBalance(_) => "INVALID_OPENING_BALANCE",
            ModelError::InvalidVatExpansion(_) => "INVALID_VAT_EXPANSION",
            ModelError::InvalidIban(_) => "INVALID_IBAN",
            ModelError::InvalidPersonId(_) => "INVALID_PERSON_ID",
            ModelError::InvalidVeroFilingIdentity(_) => "INVALID_VERO_FILING_IDENTITY",
        }
    }
}
