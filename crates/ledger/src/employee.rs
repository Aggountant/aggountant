//! F7 employee master-data — the **PII-isolated** `EmployeeState` (must-spec #2,
//! `candidate.md §2/§3`).
//!
//! An employee record carries **personal data** (name, HETU, IBAN) that must never
//! land in git (git history is not erasable, so a GDPR erasure cannot be satisfied
//! there). It therefore lives only in the **erasable PII SQLite store**
//! (`masterdata/pii.sqlite`, the boundary the git-native reference masterdata docs
//! reserve and the home `.gitignore` already excludes) — never in the git-native
//! reference tree and never in the (committable) process run store. That store has an
//! at-rest-encryption seam; in the S4 slice the cipher is a plaintext placeholder
//! (see `store::sqlite_pii`), so the boundary is *isolation + gitignore* today, with
//! real encryption a later slice. The tax-card *parameters* are non-PII on their own
//! but are stored alongside the identity for coherence, so the whole record shares one
//! erasable boundary.

use serde::{Deserialize, Serialize};

use aggountant_domain::TaxCard;

/// One employee's master data (F7). Persisted only in the encrypted PII store.
///
/// **Year-to-date gross earnings are deliberately NOT a field here.** They were once
/// an embedded `ytd_gross: Money` scalar, but a scalar has no tax year (2027 payroll
/// added to 2026 earnings forever) and an `employee add` upsert rewrites the whole
/// record — silently resetting the accumulator. YTD is now **derived** from an
/// idempotent, year-scoped earnings ledger (`ytd_earnings`) that lives beside this
/// record in the same PII store but is never touched by an identity upsert. Read a
/// year's total via `Ledger::ytd_gross(employee_id, tax_year)`; advance it via
/// `Ledger::accumulate_ytd(employee_id, tax_year, natural_key, add)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmployeeState {
    /// Stable non-PII identifier used as the run/effect key and the storage key.
    pub employee_id: String,
    /// Personal data — encrypted at rest, never in git.
    pub name: String,
    /// Finnish personal identity code (HETU) — PII. Validated on the Run's
    /// HETU-preflight, not here (a stored record may predate a rule change).
    pub hetu: String,
    /// Payment account (IBAN) — PII.
    pub iban: String,
    /// Tax-card parameters the payslip is computed from (non-PII percentage).
    pub tax_card: TaxCard,
}
