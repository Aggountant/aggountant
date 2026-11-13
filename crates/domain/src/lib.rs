//! # aggountant-domain (F4 minimal + F6 policy stub)
//!
//! Deterministic, I/O-free domain functions (`candidate.md §2`). The S1 light
//! path needs only enough Domain to assemble one voucher draft from structured
//! input — **no** ALV/payroll/depreciation calculation, no eligibility, no VAT
//! rate resolution (those are later slices). The Domain "never guesses"
//! (`candidate.md §2`): the agent proposes accounts and VAT codes; here we only
//! build and locally validate.
//!
//! Policy is stubbed to a `PolicySet` identifier (F6): S1 records which policy
//! set and chart version a voucher was sealed under, into provenance, without
//! resolving any rates.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

use aggountant_model::{ModelError, Posting, VoucherDraft};

pub mod closeyear;
pub mod consult;
pub mod eligibility;
pub mod payroll;
pub mod statements;
pub mod vat;

pub use closeyear::{
    accrual_voucher, compute_depreciation, depreciation_policy_version, depreciation_rate_table,
    depreciation_voucher, AccrualInput, AssetInput, DepreciationEdition, DepreciationLine,
    DepreciationMethod, DepreciationRateTable,
};
pub use consult::{
    disclosure_source_hash, Advice, ConsultQuestion, DisclosureFact, DisclosureSafe,
    KnowledgeCandidate,
};
pub use eligibility::{
    assess_purchase, ExpenseNature, PurchaseAssessment, PurchaseAssessmentInput,
};
pub use payroll::{
    build_payroll_voucher, compute_payslip, erillisilmoitus, palkkatieto_deadline,
    palkkatietoilmoitus, payroll_accounts, resolve_rates, resolve_tulolaji, validate_hetu,
    EmployeeIntent, PayrollIntent, Payslip, RateEdition, RateTable, ResolvedRates, TaxCard,
    TulorekisteriReport,
};
pub use statements::{
    compute_statements, kpa_policy, kpa_policy_version, tase_erittelyt, KpaPolicy, KpaRow,
    Liitetiedot, RowNature, StatementKind, StatementPeriod, StatementProvenance, StatementRow,
    StatementSourceRef, Statements, TaseErittelyLine, TaseErittelyt,
};
pub use vat::{
    check_eu_sale_buyer_vat_id, check_no_eu_acquisition_code, check_vat_rate_class_mistag,
    check_vat_rate_class_mistag_with_role_table, check_voucher_vat_structure,
    classify_vat_account_postings, compute_derived_vat_filing, compute_margin_vat,
    compute_vat_filing, derive_filing_figures, derive_filing_figures_with_role_table,
    derive_vat_figures, draft_deductible_vat, draft_deductible_vat_with_role_table,
    reconcile_vat_accounts, reconcile_verotili, sum_vat_account_figures,
    sum_vat_account_figures_with_role_table, validate_eu_buyer_vat_id, vat_code_is_tax_bearing,
    vat_expand, vat_expand_with_role_table, verify_vat_expansion, verify_vat_expansion_for_seal,
    verify_vat_expansion_for_seal_with_role_table, verify_vat_expansion_with_role_table,
    ChartVatAccount, ChartVatRoles, DerivedVatFigures, MarginCategory, MarginVat, MistagGuard,
    Reconciliation, ReverseChargeScope, TaxAccountSnapshot, VatAccountMovement, VatAccountPosting,
    VatAccountPostingClass, VatAccountReconciliation, VatAccountRole, VatAccountRoleTable,
    VatDirectionReconciliation, VatExpandInput, VatFiling, VatFilingInput, VatRateEdition,
    VatRateTable, VatRecompute, VatReconciliationStatus, VatReturnField, VatReturnFields,
    VatReturnFieldsError, VatRoleTag, VatTreatment,
};

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("{0}")]
    Model(#[from] ModelError),
    /// A VAT filing mixed currencies or received a non-EUR / negative total (the
    /// S3 slice computes single-currency EUR filings only).
    #[error("VAT_CURRENCY: {0}")]
    VatCurrency(String),
    /// A sealed voucher's VAT code could not be classified/rate-resolved when
    /// deriving a period's filing aggregates (unknown direction prefix, unknown
    /// rate class, or no dated rate edition covers the document date).
    #[error("VAT_CODE: {0}")]
    VatCode(String),
    /// A draft voucher's VAT-tagged postings cancel to zero within one direction
    /// (gross base > 0 but net == 0) — the same-direction reverse-charge
    /// cancellation hole: both VAT lines tagged one direction, so self-assessed
    /// output VAT would later aggregate to a silent zero. Refused at record time
    /// (`build_draft`) before sealing. A per-voucher structural coherence
    /// violation, distinct from an unclassifiable code (`VAT_CODE`).
    #[error("VAT_STRUCTURE: {0}")]
    VatStructure(String),
    /// A draft voucher carries an EU-sale VAT code (`sales-eu-goods` /
    /// `sales-eu-services`, which zero-rate output VAT under AVL 72 b §) without a
    /// valid buyer VAT-ID registered in another EU member state. Refused at record
    /// time (`build_draft`) so a domestic sale cannot silently hide as a zero-rated
    /// intra-Community supply and under-declare its output VAT.
    #[error("VAT_BUYER_ID: {0}")]
    VatBuyerId(String),
    /// The dated statutory rate table could not resolve an edition for a
    /// `payment_date`, or an edition holds a malformed percentage (must-spec #1).
    #[error("PAYROLL_RATES: {0}")]
    PayrollRates(String),
    /// A tulolaji (income-type) code is not in the catalog data (must-spec #5).
    #[error("TULOLAJI: {0}")]
    Tulolaji(String),
    /// A HETU failed the format/checksum preflight (must-spec #2).
    #[error("INVALID_HETU: {0}")]
    Hetu(String),
    /// A payroll computation received invalid input (non-positive/non-EUR gross,
    /// out-of-range withholding, deductions exceeding gross).
    #[error("PAYROLL: {0}")]
    Payroll(String),
    /// A close-year depreciation/accrual computation failed (unknown asset class,
    /// unresolvable dated rate, non-EUR/negative base, or an un-bookable line).
    #[error("DEPRECIATION: {0}")]
    Depreciation(String),
    /// A close-year statement projection failed (an account with postings maps to no
    /// KPA row / is not in the chart, a foreign currency, or an invalid policy).
    #[error("STATEMENTS: {0}")]
    Statements(String),
}

impl DomainError {
    pub fn error_code(&self) -> &'static str {
        match self {
            DomainError::Model(e) => e.error_code(),
            DomainError::VatCurrency(_) => "VAT_CURRENCY",
            DomainError::VatCode(_) => "VAT_CODE",
            DomainError::VatStructure(_) => "VAT_STRUCTURE",
            DomainError::VatBuyerId(_) => "VAT_BUYER_ID",
            DomainError::PayrollRates(_) => "PAYROLL_RATES",
            DomainError::Tulolaji(_) => "TULOLAJI",
            DomainError::Hetu(_) => "INVALID_HETU",
            DomainError::Payroll(_) => "PAYROLL",
            DomainError::Depreciation(_) => "DEPRECIATION",
            DomainError::Statements(_) => "STATEMENTS",
        }
    }
}

/// Structured `voucher draft` input (CLI stdin JSON). Field-for-field the draft
/// shape; the postings deserialize through the Model layer's validating
/// newtypes, so malformed money/accounts/dates are rejected here already.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftInput {
    pub document_date: aggountant_model::Date,
    pub posting_date: aggountant_model::Date,
    pub description: String,
    pub postings: Vec<Posting>,
    #[serde(default)]
    pub attachment_ref: Option<aggountant_model::Hash>,
    /// The buyer's EU VAT-ID, required when any posting carries an EU-sale code
    /// (`sales-eu-goods` / `sales-eu-services`) — the AVL 72 b § condition for a
    /// zero-rated intra-Community supply. Optional in the wire shape (`None` for
    /// ordinary domestic/purchase vouchers); `build_draft` enforces its presence
    /// and format when an EU-sale code is present.
    #[serde(default)]
    pub buyer_vat_id: Option<String>,
}

/// Assemble a validated `VoucherDraft` from structured input. Local invariants
/// (`Σ = 0`, `>= 2` postings, single currency) are enforced by `VoucherDraft::new`.
///
/// Also runs the F4 reverse-charge structural guard ([`check_voucher_vat_structure`])
/// **before** the draft can be sealed: a same-direction reverse-charge malformation
/// (both VAT lines tagged `sales-`/both `purchase-`, so the direction cancels to a
/// silent zero) is refused here at record time — never allowed to become immutable
/// ledger data, where it could neither be detected without bricking the aggregator
/// nor be edited out.
///
/// It also runs the AVL 72 b § EU-sale buyer-VAT-ID gate
/// ([`check_eu_sale_buyer_vat_id`]): a voucher tagged `sales-eu-goods` /
/// `sales-eu-services` (which zero-rate output VAT) is refused unless a buyer
/// VAT-ID from another EU member state is supplied, so a domestic sale cannot
/// silently masquerade as a zero-rated intra-Community supply.
pub fn build_draft(input: DraftInput) -> Result<VoucherDraft, DomainError> {
    check_voucher_vat_structure(&input.postings)?;
    check_no_eu_acquisition_code(&input.postings)?;
    let buyer_vat_id = check_eu_sale_buyer_vat_id(&input.postings, input.buyer_vat_id.as_deref())?;
    let draft = VoucherDraft::new(
        input.document_date,
        input.posting_date,
        input.description,
        input.postings,
        input.attachment_ref,
    )?;
    Ok(match buyer_vat_id {
        Some(buyer_vat_id) => draft.with_buyer_vat_id(buyer_vat_id)?,
        None => draft,
    })
}

/// S1 policy stub (F6). A real `PolicySet` (candidate.md §7) is a map of
/// `PolicyKind -> PolicyRef` with content hashes and test vectors; S1 only needs
/// a stable identifier to stamp into provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicySet {
    pub id: String,
}

impl Default for PolicySet {
    fn default() -> Self {
        PolicySet {
            id: "fi.core.2026-01@stub".to_string(),
        }
    }
}

pub mod wage_report;
pub use wage_report::{TravelLine, WageReport};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_draft_accepts_balanced_input() {
        let json = r#"{
            "document_date":"2026-03-14","posting_date":"2026-03-14",
            "description":"Hosting",
            "postings":[
                {"account":"7690","amount":{"amount":"124.00","currency":"EUR"},"side":"debit"},
                {"account":"1910","amount":{"amount":"124.00","currency":"EUR"},"side":"credit"}
            ]}"#;
        let input: DraftInput = serde_json::from_str(json).unwrap();
        let draft = build_draft(input).unwrap();
        assert_eq!(draft.postings().len(), 2);
        assert_eq!(draft.currency().as_str(), "EUR");
    }

    #[test]
    fn build_draft_propagates_model_error_code() {
        let json = r#"{
            "document_date":"2026-03-14","posting_date":"2026-03-14",
            "description":"bad",
            "postings":[
                {"account":"7690","amount":{"amount":"124.00","currency":"EUR"},"side":"debit"},
                {"account":"1910","amount":{"amount":"100.00","currency":"EUR"},"side":"credit"}
            ]}"#;
        let input: DraftInput = serde_json::from_str(json).unwrap();
        let err = build_draft(input).unwrap_err();
        assert_eq!(err.error_code(), "UNBALANCED_VOUCHER");
    }

    #[test]
    fn build_draft_refuses_same_direction_reverse_charge_before_sealing() {
        // F4 — a same-direction reverse-charge malformation (both VAT lines tagged
        // `purchase-`, so the purchase direction cancels to a silent zero) must be
        // refused at record time, before the draft can be sealed into the immutable
        // ledger where it could neither be detected without bricking the aggregator
        // nor edited out. The balanced 10000 debit/credit voucher is otherwise valid,
        // so only the structural guard rejects it.
        let json = r#"{
            "document_date":"2026-02-15","posting_date":"2026-02-15",
            "description":"EU reverse-charge acquisition (mistagged)",
            "postings":[
                {"account":"1763","amount":{"amount":"10000.00","currency":"EUR"},"side":"debit","vat_code":"purchase-domestic-standard"},
                {"account":"2939","amount":{"amount":"10000.00","currency":"EUR"},"side":"credit","vat_code":"purchase-domestic-standard"}
            ]}"#;
        let input: DraftInput = serde_json::from_str(json).unwrap();
        let err = build_draft(input).unwrap_err();
        assert_eq!(err.error_code(), "VAT_STRUCTURE");
    }

    #[test]
    fn build_draft_accepts_a_correct_reverse_charge() {
        // The well-formed reverse charge (output `sales-` on Credit, deductible
        // `purchase-` on Debit) passes the guard and builds normally.
        let json = r#"{
            "document_date":"2026-02-15","posting_date":"2026-02-15",
            "description":"EU reverse-charge acquisition",
            "postings":[
                {"account":"1763","amount":{"amount":"10000.00","currency":"EUR"},"side":"debit","vat_code":"purchase-domestic-standard"},
                {"account":"2939","amount":{"amount":"10000.00","currency":"EUR"},"side":"credit","vat_code":"sales-domestic-standard"}
            ]}"#;
        let input: DraftInput = serde_json::from_str(json).unwrap();
        let draft = build_draft(input).unwrap();
        assert_eq!(draft.postings().len(), 2);
    }

    #[test]
    fn build_draft_refuses_a_purchase_eu_zero_rated_acquisition() {
        // A `purchase-eu-goods` line would resolve to 0 %, but an intra-Community
        // acquisition must be reverse-charged — refused at record time.
        let json = r#"{
            "document_date":"2026-05-31","posting_date":"2026-05-31",
            "description":"EU acquisition mis-booked as zero-rated purchase",
            "postings":[
                {"account":"1763","amount":{"amount":"10000.00","currency":"EUR"},"side":"debit","vat_code":"purchase-eu-goods"},
                {"account":"2939","amount":{"amount":"10000.00","currency":"EUR"},"side":"credit"}
            ]}"#;
        let input: DraftInput = serde_json::from_str(json).unwrap();
        let err = build_draft(input).unwrap_err();
        assert_eq!(err.error_code(), "VAT_CODE");
    }

    #[test]
    fn build_draft_accepts_a_partial_deductibility_purchase() {
        // AVL 117 § — a car with 50 % private use: the VAT-coded line carries the full
        // base and the `-deduct50` marker; the deductible VAT (half) is derived later.
        // The voucher is a balanced, well-formed purchase, so it builds normally.
        let json = r#"{
            "document_date":"2026-02-15","posting_date":"2026-02-15",
            "description":"Company car (50 % private use)",
            "postings":[
                {"account":"7330","amount":{"amount":"10000.00","currency":"EUR"},"side":"debit","vat_code":"purchase-domestic-standard-deduct50"},
                {"account":"1910","amount":{"amount":"10000.00","currency":"EUR"},"side":"credit"}
            ]}"#;
        let input: DraftInput = serde_json::from_str(json).unwrap();
        let draft = build_draft(input).unwrap();
        assert_eq!(draft.postings().len(), 2);
    }

    #[test]
    fn build_draft_refuses_a_partial_deductibility_marker_on_a_sale() {
        // Output VAT on a sale is never fractional — a `sales-*-deduct*` code is
        // refused at record time so it can never be sealed.
        let json = r#"{
            "document_date":"2026-02-15","posting_date":"2026-02-15",
            "description":"sale with a nonsensical deduct marker",
            "postings":[
                {"account":"1910","amount":{"amount":"1000.00","currency":"EUR"},"side":"debit"},
                {"account":"3000","amount":{"amount":"1000.00","currency":"EUR"},"side":"credit","vat_code":"sales-domestic-standard-deduct50"}
            ]}"#;
        let input: DraftInput = serde_json::from_str(json).unwrap();
        let err = build_draft(input).unwrap_err();
        assert_eq!(err.error_code(), "VAT_CODE");
    }

    #[test]
    fn build_draft_refuses_eu_sale_without_a_buyer_vat_id() {
        // AVL 72 b § — a `sales-eu-goods` line zero-rates output VAT, so without a
        // buyer VAT-ID this is exactly a domestic sale masquerading as a zero-rated
        // intra-Community supply. The balanced voucher is otherwise valid, so only
        // the buyer-VAT-ID gate rejects it. No `buyer_vat_id` in the JSON.
        let json = r#"{
            "document_date":"2026-05-31","posting_date":"2026-05-31",
            "description":"EU goods sale (buyer VAT-ID missing)",
            "postings":[
                {"account":"1910","amount":{"amount":"10000.00","currency":"EUR"},"side":"debit"},
                {"account":"3000","amount":{"amount":"10000.00","currency":"EUR"},"side":"credit","vat_code":"sales-eu-goods"}
            ]}"#;
        let input: DraftInput = serde_json::from_str(json).unwrap();
        let err = build_draft(input).unwrap_err();
        assert_eq!(err.error_code(), "VAT_BUYER_ID");
    }

    #[test]
    fn build_draft_accepts_a_margin_scheme_sale() {
        // AVL 79 a–k § — a second-hand margin sale: the sales line carries the VAT-
        // inclusive margin (1255) tagged sales-margin-used-goods. The voucher is a
        // balanced, well-formed sale, so it builds; the extracted VAT is derived later.
        let json = r#"{
            "document_date":"2026-02-15","posting_date":"2026-02-15",
            "description":"Used-goods resale (margin scheme)",
            "postings":[
                {"account":"1910","amount":{"amount":"1255.00","currency":"EUR"},"side":"debit"},
                {"account":"3000","amount":{"amount":"1255.00","currency":"EUR"},"side":"credit","vat_code":"sales-margin-used-goods"}
            ]}"#;
        let input: DraftInput = serde_json::from_str(json).unwrap();
        let draft = build_draft(input).unwrap();
        assert_eq!(draft.postings().len(), 2);
    }

    #[test]
    fn build_draft_refuses_a_purchase_margin_code() {
        // The margin scheme is a reseller's output VAT; a purchase-margin-* code is
        // refused at record time before it can be sealed.
        let json = r#"{
            "document_date":"2026-02-15","posting_date":"2026-02-15",
            "description":"nonsensical purchase margin code",
            "postings":[
                {"account":"4000","amount":{"amount":"1255.00","currency":"EUR"},"side":"debit","vat_code":"purchase-margin-used-goods"},
                {"account":"1910","amount":{"amount":"1255.00","currency":"EUR"},"side":"credit"}
            ]}"#;
        let input: DraftInput = serde_json::from_str(json).unwrap();
        let err = build_draft(input).unwrap_err();
        assert_eq!(err.error_code(), "VAT_CODE");
    }

    #[test]
    fn build_draft_accepts_an_eu_sale_with_a_valid_buyer_vat_id() {
        // The same voucher with a valid other-member-state buyer VAT-ID builds.
        let json = r#"{
            "document_date":"2026-05-31","posting_date":"2026-05-31",
            "description":"EU goods sale to a Swedish buyer",
            "buyer_vat_id":"SE556677889901",
            "postings":[
                {"account":"1910","amount":{"amount":"10000.00","currency":"EUR"},"side":"debit"},
                {"account":"3000","amount":{"amount":"10000.00","currency":"EUR"},"side":"credit","vat_code":"sales-eu-goods"}
            ]}"#;
        let input: DraftInput = serde_json::from_str(json).unwrap();
        let draft = build_draft(input).unwrap();
        assert_eq!(draft.postings().len(), 2);
        assert_eq!(draft.buyer_vat_id(), Some("SE556677889901"));
    }

    #[test]
    fn build_draft_normalizes_the_persisted_buyer_vat_id() {
        let json = r#"{
            "document_date":"2026-05-31","posting_date":"2026-05-31",
            "description":"EU services sale to a Swedish buyer",
            "buyer_vat_id":"se 556677889901",
            "postings":[
                {"account":"1910","amount":{"amount":"10000.00","currency":"EUR"},"side":"debit"},
                {"account":"3000","amount":{"amount":"10000.00","currency":"EUR"},"side":"credit","vat_code":"sales-eu-services"}
            ]}"#;
        let draft = build_draft(serde_json::from_str(json).unwrap()).unwrap();
        assert_eq!(draft.buyer_vat_id(), Some("SE556677889901"));
    }
}
