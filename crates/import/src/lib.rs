//! # aggountant-import (F23/F24 MVP — camt.053 tosite-loading)
//!
//! The smallest missing glue of the manual tosite-loading pipeline
//! (`briox-holvi-import-strategy/architecture-options.md` §3.2): a deterministic
//! **camt.053 parser** ([`camt053`]) and the lawful **projection** from a parsed
//! bank transaction to a [`aggountant_domain::DraftInput`] ([`project_draft`]).
//!
//! The split enforces the epic's determinism/judgment rule
//! (`phase4-imports` must-spec #2): the parser turns the format into canonical,
//! account-free rows; the projection fills only the *lawful* half of a voucher
//! (which leg is the bank account, which side, the amount) from a
//! **skill-supplied** [`Assignment`]. The contra account and VAT code are the
//! skill's judgment — this crate never guesses them.
//!
//! Everything downstream is the built S1 record→seal path: the caller feeds the
//! projected `DraftInput` through `aggountant_domain::build_draft` and
//! `aggountant_ledger::Ledger::{validate,seal}` — the import crate depends on
//! `aggountant-domain` and `aggountant-model` only, and touches no ledger internals.

#![forbid(unsafe_code)]

pub mod camt053;
mod error;

pub use camt053::{parse_camt053, BankTransaction, Direction, ParsedStatement};
pub use error::ImportError;

pub use aggountant_domain::{ReverseChargeScope, VatTreatment};

use serde::Deserialize;

use aggountant_domain::{
    build_draft, vat_code_is_tax_bearing, vat_expand, vat_expand_with_role_table, DomainError,
    DraftInput, VatAccountRoleTable, VatExpandInput,
};
use aggountant_model::{AccountId, Date, Hash, Money, Posting, Side, VatCode, VoucherDraft};

/// One skill-supplied contra leg of an [`Assignment`]. Its amount is a positive
/// gross allocation. A legacy `vat_code` is accepted only when the canonical
/// dated rate is zero; any non-zero-rate code is unsafe on this gross amount and
/// is refused rather than used as a VAT base.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContraPosting {
    pub account: AccountId,
    pub amount: Money,
    #[serde(default)]
    pub vat_code: Option<VatCode>,
}

/// The skill's Stage-3 judgment for one parsed entry: which ledger accounts the
/// bank leg and its contra legs post to, and how VAT (if any) is treated. Matched
/// to a [`BankTransaction`] by `bank_ref`. The CLI never fabricates these: an
/// entry with no matching assignment is refused ([`ImportError::MissingAssignment`]).
///
/// VAT is expressed either as semantic per-contra-posting `vat_code`s, or as one
/// ADR-7 `vat_treatment`. A treatment is limited to exactly one gross allocation,
/// because Domain's [`vat_expand`] expands one net account at a time.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assignment {
    /// The entry this assignment is for (`BankTransaction::bank_ref`).
    pub bank_ref: String,
    /// The ledger account the statement itself posts to (e.g. `1910 Pankkitili`).
    pub bank_account: AccountId,
    /// The skill's expense/income legs. Must be non-empty and sum exactly to the
    /// bank transaction's amount in its currency.
    pub contra_postings: Vec<ContraPosting>,
    /// The ADR-7 VAT treatment that drives the Domain [`vat_expand`] into the full
    /// posting set. Mutually exclusive with every contra posting's `vat_code` and
    /// limited to one contra posting. Purchase-side (money-out) only.
    #[serde(default)]
    pub vat_treatment: Option<VatTreatment>,
    /// The source document's date (invoice/receipt date). Required for
    /// `vat_treatment`, because dated VAT expansion must never substitute the bank
    /// posting date. For untagged or canonically zero-rate legacy transport, absence
    /// retains the historical bank-date fallback.
    #[serde(default)]
    pub document_date: Option<Date>,
    /// An optional human description override. When absent, the projection builds
    /// one from the transaction's counterparty + remittance info.
    #[serde(default)]
    pub description: Option<String>,
}

/// Project a parsed bank transaction into a `DraftInput`, given the skill's
/// account assignment and the retained-source attachment.
///
/// The lawful mapping: for a **Credit** (money in) the bank account is *debited*
/// (asset up) and every contra account *credited*; for a **Debit** (money out)
/// the bank account is *credited* and every contra account *debited*. Contra
/// amounts must sum exactly to the bank amount, so the draft balances (`Σ = 0`).
/// The VAT code, if any, rides on its own contra leg. The posting date is always
/// the KPL bank accounting date (`BookgDt`). The document date is the assignment's
/// source date when supplied; only untagged or zero-tax legacy transport may fall
/// back to the bank date for compatibility.
///
/// This validates the assignment's split invariant before assembling the draft;
/// `build_draft` then enforces the general voucher invariants.
pub fn project_draft(
    txn: &BankTransaction,
    assignment: &Assignment,
    attachment_ref: Option<Hash>,
) -> Result<DraftInput, ImportError> {
    validate_split(txn, assignment)?;
    if assignment.vat_treatment.is_some() {
        return Err(ImportError::InvalidAssignment(format!(
            "assignment for {} carries vat_treatment; use project_voucher so Domain expands its VAT legs",
            assignment.bank_ref
        )));
    }
    validate_legacy_vat_codes(txn, assignment)?;
    let (bank_side, contra_side) = match txn.direction {
        Direction::Credit => (Side::Debit, Side::Credit),
        Direction::Debit => (Side::Credit, Side::Debit),
    };

    let bank_leg = Posting {
        account: assignment.bank_account.clone(),
        amount: txn.amount.clone(),
        side: bank_side,
        vat_code: None,
    };
    let contra_legs = assignment.contra_postings.iter().map(|contra| Posting {
        account: contra.account.clone(),
        amount: contra.amount.clone(),
        side: contra_side,
        vat_code: contra.vat_code.clone(),
    });

    Ok(DraftInput {
        document_date: assignment.document_date.unwrap_or(txn.accounting_date),
        posting_date: txn.accounting_date,
        description: assignment
            .description
            .clone()
            .unwrap_or_else(|| default_description(txn)),
        postings: std::iter::once(bank_leg).chain(contra_legs).collect(),
        attachment_ref,
        // Bank-statement imports never project an EU-sale code, so no buyer VAT-ID
        // is threaded here (the domain's AVL 72 b § gate only fires on sales-eu-*).
        buyer_vat_id: None,
    })
}

/// Verify the judgment boundary that makes a split projection lawful. The error
/// names the bank reference and observed totals so a caller can correct its JSON
/// without guessing.
fn validate_split(txn: &BankTransaction, assignment: &Assignment) -> Result<(), ImportError> {
    if assignment.contra_postings.is_empty() {
        return Err(ImportError::InvalidAssignment(format!(
            "assignment for {} has no contra_postings; provide at least one posting",
            assignment.bank_ref
        )));
    }

    let currency = txn.amount.currency();
    let mut total = rust_decimal::Decimal::ZERO;
    for (index, posting) in assignment.contra_postings.iter().enumerate() {
        if posting.amount.currency() != currency {
            return Err(ImportError::InvalidAssignment(format!(
                "assignment for {} contra_postings[{index}] currency {} does not match bank transaction currency {}",
                assignment.bank_ref,
                posting.amount.currency(),
                currency
            )));
        }
        if posting.amount.amount() <= rust_decimal::Decimal::ZERO {
            return Err(ImportError::InvalidAssignment(format!(
                "assignment for {} contra_postings[{index}] amount must be > 0, got {}",
                assignment.bank_ref,
                posting.amount.amount_str()
            )));
        }
        total = total.checked_add(posting.amount.amount()).ok_or_else(|| {
            ImportError::InvalidAssignment(format!(
                "assignment for {} contra_postings total overflows Decimal",
                assignment.bank_ref
            ))
        })?;
    }
    if total != txn.amount.amount() {
        return Err(ImportError::InvalidAssignment(format!(
            "assignment for {} contra_postings total {} {} does not equal bank transaction amount {} {}",
            assignment.bank_ref,
            total,
            currency,
            txn.amount.amount_str(),
            currency
        )));
    }
    Ok(())
}

/// Refuse a legacy VAT tag when the canonical Domain rate query says it bears tax.
/// Every contra amount in this adapter is a gross allocation, while the legacy
/// filing path interprets a tagged posting as a VAT base. Zero-rate tags remain a
/// valid transport path; malformed/unknown tags propagate `VAT_CODE` rather than
/// being treated as zero.
fn validate_legacy_vat_codes(
    txn: &BankTransaction,
    assignment: &Assignment,
) -> Result<(), ImportError> {
    let document_date = assignment.document_date.unwrap_or(txn.accounting_date);
    for (index, posting) in assignment.contra_postings.iter().enumerate() {
        let Some(code) = &posting.vat_code else {
            continue;
        };
        if vat_code_is_tax_bearing(code, document_date)? {
            return Err(ImportError::UnsafeLegacyVatCode(format!(
                "assignment for {} contra_postings[{index}].vat_code {:?} resolves to non-zero VAT on {}; contra_postings amounts are gross allocations, not VAT bases — if this is a supported purchase, use vat_treatment with an explicit document_date; sales VAT expansion is unsupported here, so do not import this assignment",
                assignment.bank_ref,
                code.as_str(),
                document_date
            )));
        }
    }
    Ok(())
}

/// Project a parsed bank transaction into the sealed-ready `VoucherDraft`, choosing
/// the projection from the assignment's VAT shape:
///
/// * no `vat_treatment` → the simple 2-leg `bank ↔ contra` projection
///   ([`project_draft`]) through [`build_draft`] — the S1 path, unchanged;
/// * a `vat_treatment` → the ADR-7 (VAT-A) Domain [`vat_expand`], which seals the
///   explicit VAT legs. **Expansion lives in the Domain, not this adapter** — the
///   adapter only routes the transaction's amount/date and the skill's treatment
///   into `vat_expand` and returns its draft.
///
/// A `vat_treatment` is purchase-side and requires `Assignment.document_date`: it
/// is refused on a money-in (`Credit`) entry, without that source date, or alongside
/// a `vat_code` (the two VAT paths are mutually exclusive), so a mis-shaped
/// assignment fails loudly rather than sealing a wrong image.
pub fn project_voucher(
    txn: &BankTransaction,
    assignment: &Assignment,
    attachment_ref: Option<Hash>,
) -> Result<VoucherDraft, ImportError> {
    project_voucher_using(txn, assignment, attachment_ref, vat_expand)
}

/// Project a voucher using the exact active company-chart VAT role table. Production
/// callers use this path so reverse-charge expansion cannot fall back to bundled
/// account-number conventions.
pub fn project_voucher_with_role_table(
    txn: &BankTransaction,
    assignment: &Assignment,
    attachment_ref: Option<Hash>,
    chart_version: &str,
    role_table: &VatAccountRoleTable,
) -> Result<VoucherDraft, ImportError> {
    project_voucher_using(txn, assignment, attachment_ref, |input| {
        vat_expand_with_role_table(input, chart_version, role_table)
    })
}

fn project_voucher_using(
    txn: &BankTransaction,
    assignment: &Assignment,
    attachment_ref: Option<Hash>,
    expand: impl FnOnce(&VatExpandInput) -> Result<VoucherDraft, DomainError>,
) -> Result<VoucherDraft, ImportError> {
    let Some(treatment) = &assignment.vat_treatment else {
        return Ok(build_draft(project_draft(
            txn,
            assignment,
            attachment_ref,
        )?)?);
    };
    // A treatment still carries one gross allocation. Validating it here makes
    // that allocation equal to `txn.amount` before Domain expands its VAT legs.
    validate_split(txn, assignment)?;
    if assignment.contra_postings.len() != 1 {
        return Err(ImportError::InvalidAssignment(format!(
            "assignment for {} carries a vat_treatment with {} contra_postings; VAT expansion supports exactly one net account",
            assignment.bank_ref,
            assignment.contra_postings.len()
        )));
    }
    let contra = &assignment.contra_postings[0];
    if contra.vat_code.is_some() {
        return Err(ImportError::InvalidAssignment(format!(
            "assignment for {} sets both contra_postings[0].vat_code and vat_treatment; set one VAT path",
            assignment.bank_ref
        )));
    }
    if txn.direction != Direction::Debit {
        return Err(ImportError::InvalidAssignment(format!(
            "assignment for {} carries a vat_treatment, but the entry is money-in (Credit); \
             deductible input VAT and reverse-charge output are purchase-side (money-out) legs",
            assignment.bank_ref
        )));
    }
    let document_date = assignment.document_date.ok_or_else(|| {
        ImportError::MissingDocumentDate(format!(
            "assignment for {} field document_date is required when vat_treatment is set; provide the source invoice/receipt date (YYYY-MM-DD), not the bank accounting date {}",
            assignment.bank_ref, txn.accounting_date
        ))
    })?;
    let input = VatExpandInput {
        amount: txn.amount.clone(),
        // Under a treatment the assignment's contra account is ADR-7's net account;
        // the cash leg squares against the bank account and the VAT legs are derived.
        net_account: contra.account.clone(),
        contra_account: assignment.bank_account.clone(),
        treatment: treatment.clone(),
        document_date,
        posting_date: txn.accounting_date,
        description: assignment
            .description
            .clone()
            .unwrap_or_else(|| default_description(txn)),
        attachment_ref,
    };
    Ok(expand(&input)?)
}

/// Build a non-empty description from the transaction when the skill supplies no
/// override: `"<counterparty> — <remittance>"`, falling back to whichever half is
/// present, and finally to the bank reference (so the draft never has an empty
/// description, which `VoucherDraft::new` rejects).
fn default_description(txn: &BankTransaction) -> String {
    match (&txn.counterparty, &txn.description) {
        (Some(cp), Some(rmt)) => format!("{cp} — {rmt}"),
        (Some(cp), None) => cp.clone(),
        (None, Some(rmt)) => rmt.clone(),
        (None, None) => format!("Bank entry {}", txn.bank_ref),
    }
}

/// Reject an assignment list that names the same `bank_ref` twice. Assignments
/// match transactions by `bank_ref`; a duplicate would let the first row silently
/// win and post the wrong account/VAT for a later entry, so it is refused up front.
pub fn check_assignments_unique(assignments: &[Assignment]) -> Result<(), ImportError> {
    let mut seen = std::collections::HashSet::new();
    for a in assignments {
        if !seen.insert(a.bank_ref.as_str()) {
            return Err(ImportError::InvalidAssignment(format!(
                "duplicate assignment for bank_ref {} — each entry must have exactly one assignment",
                a.bank_ref
            )));
        }
    }
    Ok(())
}

/// Look up the [`Assignment`] for a transaction by `bank_ref`, or refuse.
pub fn assignment_for<'a>(
    txn: &BankTransaction,
    assignments: &'a [Assignment],
) -> Result<&'a Assignment, ImportError> {
    assignments
        .iter()
        .find(|a| a.bank_ref == txn.bank_ref)
        .ok_or_else(|| {
            ImportError::MissingAssignment(format!(
                "no account assignment for entry {} ({}) — the CLI does not auto-assign accounts",
                txn.bank_ref, txn.natural_key
            ))
        })
}
