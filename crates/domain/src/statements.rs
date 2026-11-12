//! Deterministic **financial-statement projection** for `close-year` (RF6/S6,
//! scenario D2). Pure, I/O-free: fold the **sealed** vouchers of a fiscal year into
//! the KPA-kaava statement layout defined by the regime's **policy** (`kpa-rows.json`,
//! resolve-then-pin), producing `tuloslaskelma` (income statement), `tase` (balance
//! sheet), `tase-erittelyt` and the determined parts of `liitetiedot`.
//!
//! The S6 face of the LLM/law boundary (`s6-spec-ahead.md §3.1`): **the statement
//! layout is policy, the numbers are Domain, the agent interprets only what a
//! transaction *is*, never where a line *goes*.** The account→KPA-row mapping lives
//! in the pinned policy (longest-prefix match); the fold of sealed postings into the
//! layout is a pure projection (the S6 analogue of `compute_vat_filing`). The result
//! reproduces byte-for-byte from (sealed inputs + pinned policy) → a golden fixture.
//!
//! Opening balances are explicit [`OpeningBalance`] inputs. They are never spoofed as
//! numbered vouchers, so statement source references distinguish fiscal records from
//! a prior sealed close or an external ADR-8 chart cut-over.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use aggountant_model::{
    Account, AccountKind, Currency, Date, FiscalYear, Money, OpeningBalance, OpeningBalanceSource,
    VoucherBody,
};

use crate::DomainError;

/// Which statement a KPA row belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementKind {
    Income,
    Balance,
}

/// The nature of a KPA row — fixes its sign convention and (for the income
/// statement) whether it adds to or subtracts from the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowNature {
    /// Income-statement revenue (credit-normal); adds to the result.
    Revenue,
    /// Income-statement expense (debit-normal); subtracts from the result.
    Expense,
    /// Balance-sheet asset (debit-normal).
    Assets,
    /// Balance-sheet equity + liabilities (credit-normal).
    EquityLiabilities,
}

/// One row of the KPA statement layout (policy data).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KpaRow {
    pub row_id: String,
    pub statement: StatementKind,
    pub nature: RowNature,
    /// Presentation order within the statement (ascending).
    pub order: i64,
    pub label: String,
    /// Account-number prefixes that map to this row; an account belongs to the row
    /// whose listed prefix is the **longest** matching prefix of the account id.
    pub accounts: Vec<String>,
}

/// The bundled, pinned KPA-kaava policy (`kpa-rows.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KpaPolicy {
    pub policy: String,
    pub version: String,
    pub rows: Vec<KpaRow>,
}

const KPA_ROWS_JSON: &str = include_str!("../data/kpa-rows.json");

/// Parse + validate the bundled KPA policy once. Rejects duplicate `row_id`s and a
/// prefix claimed by two different rows within the same statement (an ambiguous
/// account→row mapping would make the projection order-dependent).
pub fn kpa_policy() -> &'static KpaPolicy {
    static POLICY: OnceLock<KpaPolicy> = OnceLock::new();
    POLICY.get_or_init(|| {
        let policy: KpaPolicy =
            serde_json::from_str(KPA_ROWS_JSON).expect("bundled kpa-rows.json parses");
        validate_kpa_policy(&policy).expect("bundled kpa-rows.json is a valid policy");
        policy
    })
}

/// The pinned KPA policy version identifier (stamped to statement provenance).
pub fn kpa_policy_version() -> &'static str {
    &kpa_policy().version
}

fn validate_kpa_policy(policy: &KpaPolicy) -> Result<(), DomainError> {
    let mut seen_ids = std::collections::BTreeSet::new();
    // (statement, prefix) → row_id, to catch a prefix owned by two rows.
    let mut prefix_owner: BTreeMap<(StatementKind, &str), &str> = BTreeMap::new();
    for row in &policy.rows {
        if !seen_ids.insert(&row.row_id) {
            return Err(DomainError::Statements(format!(
                "duplicate KPA row_id {:?}",
                row.row_id
            )));
        }
        // The nature must agree with the statement (a revenue row on the balance
        // sheet is a policy error).
        let ok = match row.statement {
            StatementKind::Income => {
                matches!(row.nature, RowNature::Revenue | RowNature::Expense)
            }
            StatementKind::Balance => {
                matches!(row.nature, RowNature::Assets | RowNature::EquityLiabilities)
            }
        };
        if !ok {
            return Err(DomainError::Statements(format!(
                "KPA row {:?} nature does not match its statement",
                row.row_id
            )));
        }
        for prefix in &row.accounts {
            if prefix.is_empty() || !prefix.bytes().all(|b| b.is_ascii_digit()) {
                return Err(DomainError::Statements(format!(
                    "KPA row {:?} has a non-numeric/empty account prefix {prefix:?}",
                    row.row_id
                )));
            }
            if let Some(other) = prefix_owner.insert((row.statement, prefix.as_str()), &row.row_id)
            {
                return Err(DomainError::Statements(format!(
                    "account prefix {prefix:?} is claimed by both {other:?} and {:?}",
                    row.row_id
                )));
            }
        }
    }
    Ok(())
}

/// Provenance stamped onto a statement set so a historical close re-derives
/// identically (O-19/O-26): the chart version + the policy set + the pinned KPA
/// policy version the projection used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatementProvenance {
    pub chart_version: String,
    pub policy_set: String,
    pub kpa_policy_version: String,
}

/// One projected statement line (KPA row + its folded amount).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatementRow {
    pub row_id: String,
    pub label: String,
    pub nature: RowNature,
    /// The signed EUR amount folded into this row (normal-positive by nature).
    pub amount: String,
}

/// The full deterministic statement set for a fiscal year.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statements {
    pub fiscal_year: i32,
    /// The dates of a fiscal year that is not a calendar year. Absent for a calendar
    /// year, so a calendar year's statement bytes (and hash) are what they were before
    /// fiscal-year definitions existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period: Option<StatementPeriod>,
    /// `tuloslaskelma` rows, in policy order.
    pub income: Vec<StatementRow>,
    /// `tase` rows, in policy order.
    pub balance: Vec<StatementRow>,
    /// `tilikauden tulos` = Σ revenue − Σ expense (signed EUR). Also booked as the
    /// synthetic `BS-TILIKAUDEN-TULOS` equity row in `balance`.
    pub result: String,
    /// Σ asset rows.
    pub assets_total: String,
    /// Σ equity + liability rows, **including** the synthetic result row — so it
    /// equals `assets_total` for a balanced set.
    pub equity_liabilities_total: String,
    /// True iff `assets_total == equity_liabilities_total` (the balance-sheet
    /// identity, with the year's result folded into equity; every double-entry set
    /// satisfies it).
    pub balanced: bool,
    /// Hash-bound source of the opening position, including an external cut-over's
    /// chart mapping and live-period boundary when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening_balance_source: Option<OpeningBalanceSource>,
    pub provenance: StatementProvenance,
}

/// The first and last day of a non-calendar fiscal year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatementPeriod {
    pub start: Date,
    pub end: Date,
}

impl StatementPeriod {
    fn of(fiscal_year: &FiscalYear) -> Option<Self> {
        (!fiscal_year.is_calendar_year()).then(|| Self {
            start: fiscal_year.start(),
            end: fiscal_year.end(),
        })
    }
}

impl Statements {
    /// The canonical serialized bytes of the statement set — the exact bytes the
    /// `ApproveStatements` Preview gate binds and `SealYear` covers.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("Statements serializes")
    }

    /// `sha256` of the canonical bytes (the `statements_hash`).
    pub fn statements_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.canonical_bytes());
        hex::encode(h.finalize())
    }
}

fn eur() -> Currency {
    Currency::new("EUR").expect("EUR is a valid currency code")
}

/// The normal-positive balance of one account over a set of postings, given its
/// kind: debit-normal (asset/expense) = Σ signed_amount; credit-normal
/// (income/liability/equity) = −Σ signed_amount.
fn account_balance(kind: AccountKind, signed_sum: Decimal) -> Decimal {
    match kind {
        AccountKind::Asset | AccountKind::Expense => signed_sum,
        AccountKind::Income | AccountKind::Liability | AccountKind::Equity => -signed_sum,
    }
}

/// Find the KPA row for `account` within `statement`: the row whose listed prefix is
/// the **longest** prefix of the account id. `None` if no row claims it.
fn row_for_account<'a>(
    policy: &'a KpaPolicy,
    statement: StatementKind,
    account: &str,
) -> Option<&'a KpaRow> {
    let mut best: Option<(&KpaRow, usize)> = None;
    for row in &policy.rows {
        if row.statement != statement {
            continue;
        }
        for prefix in &row.accounts {
            if account.starts_with(prefix.as_str()) {
                let len = prefix.len();
                if best.map(|(_, bl)| len > bl).unwrap_or(true) {
                    best = Some((row, len));
                }
            }
        }
    }
    best.map(|(r, _)| r)
}

fn validate_opening_context(
    fiscal_year: &FiscalYear,
    opening: &OpeningBalance,
    provenance: &StatementProvenance,
) -> Result<(), DomainError> {
    if opening.fiscal_year() != fiscal_year.year() {
        return Err(DomainError::Statements(format!(
            "opening balance is for {}, not fiscal year {}",
            opening.fiscal_year(),
            fiscal_year.year()
        )));
    }
    if let OpeningBalanceSource::ExternalCutover {
        target_chart_version,
        live_period,
        ..
    } = opening.source()
    {
        if target_chart_version != &provenance.chart_version {
            return Err(DomainError::Statements(format!(
                "opening cut-over targets chart {target_chart_version}, but the projection uses {}",
                provenance.chart_version
            )));
        }
        let (first, last) = live_period.month_bounds();
        let inside = |month: u8| {
            Date::from_ymd(live_period.year(), month.into(), 1)
                .is_ok_and(|day| fiscal_year.contains(day))
        };
        if !inside(first) || !inside(last) {
            return Err(DomainError::Statements(format!(
                "cut-over live period {live_period} is not in fiscal year {} ({}..{})",
                fiscal_year.year(),
                fiscal_year.start(),
                fiscal_year.end()
            )));
        }
    }
    Ok(())
}

fn eur_str(d: Decimal) -> String {
    let mut d = d;
    d.rescale(2);
    Money::new(d, eur()).amount_str()
}

/// Compute the deterministic statement set for `fiscal_year` from its sealed
/// vouchers. Income-statement rows fold the year's income/expense postings; balance
/// rows fold the asset/equity/liability postings (single-year scope). Every account
/// touched must map to a KPA row and be EUR — an unmapped account or a foreign
/// currency fails loudly (a mis-mapping is a policy error surfaced at the gate, never
/// a silent omission — `s6-spec-ahead.md §2.3`).
pub fn compute_statements(
    fiscal_year: &FiscalYear,
    opening_balance: Option<&OpeningBalance>,
    vouchers: &[VoucherBody],
    chart: &[Account],
    policy: &KpaPolicy,
    provenance: StatementProvenance,
) -> Result<Statements, DomainError> {
    // Defense-in-depth: a caller-supplied policy is validated here too (the bundled
    // singleton is already validated, but `compute_statements` is public — an invalid
    // policy with duplicate row_ids / ambiguous prefixes must not silently double-count).
    validate_kpa_policy(policy)?;

    let kind_of: BTreeMap<&str, AccountKind> =
        chart.iter().map(|a| (a.id.as_str(), a.kind)).collect();

    // Signed sum per account over the explicit opening position and the year's sealed
    // vouchers. Opening data has no date/number and therefore cannot leak a sentinel.
    let mut signed: BTreeMap<String, Decimal> = BTreeMap::new();
    if let Some(opening) = opening_balance {
        validate_opening_context(fiscal_year, opening, &provenance)?;
        for posting in opening.postings() {
            match kind_of.get(posting.account.as_str()) {
                Some(AccountKind::Asset | AccountKind::Liability | AccountKind::Equity) => {}
                Some(kind) => {
                    return Err(DomainError::Statements(format!(
                        "opening balance cannot post to {:?} account {}",
                        kind, posting.account
                    )));
                }
                None => {
                    return Err(DomainError::Statements(format!(
                        "opening balance account {} is not in the chart",
                        posting.account
                    )));
                }
            }
            if posting.amount.currency() != &eur() {
                return Err(DomainError::Statements(format!(
                    "opening balance posts {} but statements are EUR-only",
                    posting.amount.currency()
                )));
            }
            *signed
                .entry(posting.account.as_str().to_string())
                .or_default() += posting.signed_amount();
        }
    }
    for voucher in vouchers {
        if !fiscal_year.contains(voucher.posting_date()) {
            continue;
        }
        for posting in voucher.postings() {
            if posting.amount.currency() != &eur() {
                return Err(DomainError::Statements(format!(
                    "voucher {} posts {} but statements are EUR-only",
                    voucher.number(),
                    posting.amount.currency()
                )));
            }
            *signed
                .entry(posting.account.as_str().to_string())
                .or_default() += posting.signed_amount();
        }
    }

    // Fold each account's normal balance into its KPA row (per statement).
    let mut income_amounts: BTreeMap<&str, Decimal> = BTreeMap::new();
    let mut balance_amounts: BTreeMap<&str, Decimal> = BTreeMap::new();
    for (account, signed_sum) in &signed {
        let kind = *kind_of.get(account.as_str()).ok_or_else(|| {
            DomainError::Statements(format!(
                "account {account} has postings but is not in the chart"
            ))
        })?;
        let bal = account_balance(kind, *signed_sum);
        let statement = match kind {
            AccountKind::Income | AccountKind::Expense => StatementKind::Income,
            AccountKind::Asset | AccountKind::Liability | AccountKind::Equity => {
                StatementKind::Balance
            }
        };
        let row = row_for_account(policy, statement, account).ok_or_else(|| {
            DomainError::Statements(format!(
                "account {account} maps to no KPA row in the {:?} statement — the policy \
                 does not cover it (a mis-mapping surfaces here, never a silent omission)",
                statement
            ))
        })?;
        // The row's nature must agree with the account's kind, or the sign convention
        // would silently invert (e.g. an Income account mapped to an Expense row would
        // subtract from the result instead of adding). Reject the mismatch loudly.
        let nature_ok = matches!(
            (kind, row.nature),
            (AccountKind::Income, RowNature::Revenue)
                | (AccountKind::Expense, RowNature::Expense)
                | (AccountKind::Asset, RowNature::Assets)
                | (AccountKind::Liability, RowNature::EquityLiabilities)
                | (AccountKind::Equity, RowNature::EquityLiabilities)
        );
        if !nature_ok {
            return Err(DomainError::Statements(format!(
                "account {account} (kind {:?}) maps to KPA row {} (nature {:?}) — the \
                 kind/nature mismatch would invert its sign",
                kind, row.row_id, row.nature
            )));
        }
        match statement {
            StatementKind::Income => *income_amounts.entry(&row.row_id).or_default() += bal,
            StatementKind::Balance => *balance_amounts.entry(&row.row_id).or_default() += bal,
        }
    }

    // Emit rows in policy order (only rows with a non-zero fold, for a clean set —
    // deterministic because the policy order is fixed).
    let mut rows_sorted: Vec<&KpaRow> = policy.rows.iter().collect();
    rows_sorted.sort_by_key(|r| (r.order, r.row_id.clone()));

    let mut income = Vec::new();
    let mut balance = Vec::new();
    let mut revenue_total = Decimal::ZERO;
    let mut expense_total = Decimal::ZERO;
    let mut assets_total = Decimal::ZERO;
    let mut eq_liab_total = Decimal::ZERO;

    for row in rows_sorted {
        match row.statement {
            StatementKind::Income => {
                let amt = income_amounts.get(row.row_id.as_str()).copied();
                if let Some(amt) = amt {
                    match row.nature {
                        RowNature::Revenue => revenue_total += amt,
                        RowNature::Expense => expense_total += amt,
                        _ => {}
                    }
                    income.push(StatementRow {
                        row_id: row.row_id.clone(),
                        label: row.label.clone(),
                        nature: row.nature,
                        amount: eur_str(amt),
                    });
                }
            }
            StatementKind::Balance => {
                let amt = balance_amounts.get(row.row_id.as_str()).copied();
                if let Some(amt) = amt {
                    match row.nature {
                        RowNature::Assets => assets_total += amt,
                        RowNature::EquityLiabilities => eq_liab_total += amt,
                        _ => {}
                    }
                    balance.push(StatementRow {
                        row_id: row.row_id.clone(),
                        label: row.label.clone(),
                        nature: row.nature,
                        amount: eur_str(amt),
                    });
                }
            }
        }
    }

    let result = revenue_total - expense_total;
    // Book the year's result as an explicit equity row ("Tilikauden tulos") so the
    // balance sheet actually sums: the income/expense accounts are not closed to
    // equity in the sealed postings, so without this the `tase` rows would be short
    // by exactly `result`. Appended after the policy rows (deterministic order) and
    // folded into the equity+liabilities total.
    balance.push(StatementRow {
        row_id: "BS-TILIKAUDEN-TULOS".to_string(),
        label: "Tilikauden tulos".to_string(),
        nature: RowNature::EquityLiabilities,
        amount: eur_str(result),
    });
    eq_liab_total += result;

    // Balance-sheet identity: assets == equity+liabilities (result now folded in).
    // Holds for any balanced double-entry set; a violation signals a corrupt/torn read.
    let balanced = assets_total == eq_liab_total;

    Ok(Statements {
        fiscal_year: fiscal_year.year(),
        period: StatementPeriod::of(fiscal_year),
        income,
        balance,
        result: eur_str(result),
        assets_total: eur_str(assets_total),
        equity_liabilities_total: eur_str(eq_liab_total),
        balanced,
        opening_balance_source: opening_balance.map(|o| o.source().clone()),
        provenance,
    })
}

/// One line of `tase-erittelyt` (balance specifications): a balance-sheet account,
/// its closing balance, and the voucher numbers that touched it (the determined,
/// deterministic breakdown — `s6-spec-ahead.md §2.3` / Fork F).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaseErittelyLine {
    pub account: String,
    pub account_name: String,
    pub closing_balance: String,
    pub source_refs: Vec<StatementSourceRef>,
}

/// Auditable source of one balance-specification movement. Opening data is tagged
/// separately from sealed voucher numbers; no in-band `u64::MAX` sentinel exists.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StatementSourceRef {
    /// Declared first so deterministic source order follows accounting chronology:
    /// the opening position precedes in-year voucher movements. Variant order is
    /// hash-load-bearing for `TaseErittelyt::statements_hash`.
    OpeningBalance {
        source: OpeningBalanceSource,
    },
    SealedVoucher {
        number: u64,
    },
}

/// The `tase-erittelyt` set for a fiscal year.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaseErittelyt {
    pub fiscal_year: i32,
    pub lines: Vec<TaseErittelyLine>,
    pub provenance: StatementProvenance,
}

impl TaseErittelyt {
    pub fn statements_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(serde_json::to_vec(self).expect("TaseErittelyt serializes"));
        hex::encode(h.finalize())
    }
}

/// Compute the deterministic `tase-erittelyt`: for each balance-sheet account with a
/// non-zero closing balance, its normal balance + the (sorted) voucher numbers that
/// touched it.
pub fn tase_erittelyt(
    fiscal_year: &FiscalYear,
    opening_balance: Option<&OpeningBalance>,
    vouchers: &[VoucherBody],
    chart: &[Account],
    provenance: StatementProvenance,
) -> Result<TaseErittelyt, DomainError> {
    let account_info: BTreeMap<&str, (AccountKind, &str)> = chart
        .iter()
        .map(|a| (a.id.as_str(), (a.kind, a.name.as_str())))
        .collect();

    let mut signed: BTreeMap<String, Decimal> = BTreeMap::new();
    let mut touched: BTreeMap<String, std::collections::BTreeSet<StatementSourceRef>> =
        BTreeMap::new();
    if let Some(opening) = opening_balance {
        validate_opening_context(fiscal_year, opening, &provenance)?;
        let source = StatementSourceRef::OpeningBalance {
            source: opening.source().clone(),
        };
        for posting in opening.postings() {
            match account_info.get(posting.account.as_str()) {
                Some((AccountKind::Asset | AccountKind::Liability | AccountKind::Equity, _)) => {}
                Some((kind, _)) => {
                    return Err(DomainError::Statements(format!(
                        "opening balance cannot post to {:?} account {}",
                        kind, posting.account
                    )));
                }
                None => {
                    return Err(DomainError::Statements(format!(
                        "opening balance account {} is not in the chart",
                        posting.account
                    )));
                }
            }
            if posting.amount.currency() != &eur() {
                return Err(DomainError::Statements(format!(
                    "opening balance posts {} but tase-erittelyt is EUR-only",
                    posting.amount.currency()
                )));
            }
            let acct = posting.account.as_str().to_string();
            *signed.entry(acct.clone()).or_default() += posting.signed_amount();
            touched.entry(acct).or_default().insert(source.clone());
        }
    }
    for voucher in vouchers {
        if !fiscal_year.contains(voucher.posting_date()) {
            continue;
        }
        for posting in voucher.postings() {
            if posting.amount.currency() != &eur() {
                return Err(DomainError::Statements(format!(
                    "voucher {} posts {} but tase-erittelyt is EUR-only",
                    voucher.number(),
                    posting.amount.currency()
                )));
            }
            let acct = posting.account.as_str().to_string();
            *signed.entry(acct.clone()).or_default() += posting.signed_amount();
            touched
                .entry(acct)
                .or_default()
                .insert(StatementSourceRef::SealedVoucher {
                    number: voucher.number(),
                });
        }
    }

    let mut lines = Vec::new();
    for (account, signed_sum) in &signed {
        let (kind, name) = *account_info.get(account.as_str()).ok_or_else(|| {
            DomainError::Statements(format!("account {account} not in the chart"))
        })?;
        // Only balance-sheet accounts appear in tase-erittelyt.
        if matches!(kind, AccountKind::Income | AccountKind::Expense) {
            continue;
        }
        let bal = account_balance(kind, *signed_sum);
        if bal.is_zero() {
            continue;
        }
        lines.push(TaseErittelyLine {
            account: account.clone(),
            account_name: name.to_string(),
            closing_balance: eur_str(bal),
            source_refs: touched
                .get(account)
                .map(|s| s.iter().cloned().collect())
                .unwrap_or_default(),
        });
    }
    lines.sort_by(|a, b| a.account.cmp(&b.account));

    Ok(TaseErittelyt {
        fiscal_year: fiscal_year.year(),
        lines,
        provenance,
    })
}

/// The determined, deterministic parts of `liitetiedot` (notes). Narrative notes
/// (accounting-policy prose, going-concern) are **not** generated here — Fork F:
/// deterministic determined-parts only; narrative is flagged as a follow-up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Liitetiedot {
    pub fiscal_year: i32,
    /// The depreciation-policy note, derived from the pinned depreciation policy:
    /// one line per asset class actually depreciated (`class`, `method`, `rate`).
    pub depreciation_notes: Vec<DepreciationNote>,
    /// The pinned depreciation policy version the notes are derived from.
    pub depreciation_policy_version: String,
    /// True: narrative notes (policy prose, going-concern) are a documented
    /// follow-up, not produced deterministically in S6.
    pub narrative_pending: bool,
    pub provenance: StatementProvenance,
}

/// A determined depreciation note (from the policy, not the agent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DepreciationNote {
    pub asset_class: String,
    pub method: String,
    pub rate_percent: String,
}

impl Liitetiedot {
    pub fn statements_hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(serde_json::to_vec(self).expect("Liitetiedot serializes"));
        hex::encode(h.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aggountant_model::{AccountId, Date, Posting, Side, VoucherDraft};

    fn eur_money(s: &str) -> Money {
        Money::parse(s, eur()).unwrap()
    }
    fn date(s: &str) -> Date {
        Date::parse(s).unwrap()
    }

    fn y2026() -> FiscalYear {
        FiscalYear::calendar(2026).unwrap()
    }

    fn prov() -> StatementProvenance {
        StatementProvenance {
            chart_version: "chart-test@0".into(),
            policy_set: "fi.core.2026-01@stub".into(),
            kpa_policy_version: kpa_policy_version().to_string(),
        }
    }

    /// Build a sealed-body voucher from balanced (account, amount, side) lines.
    fn voucher(number: u64, posting_date: &str, lines: &[(&str, &str, Side)]) -> VoucherBody {
        let postings: Vec<Posting> = lines
            .iter()
            .map(|(acct, amt, side)| Posting {
                account: AccountId::new(*acct).unwrap(),
                amount: eur_money(amt),
                side: *side,
                vat_code: None,
            })
            .collect();
        let draft = VoucherDraft::new(
            date(posting_date),
            date(posting_date),
            "test".into(),
            postings,
            None,
        )
        .unwrap();
        VoucherBody::from_draft(&draft, number, "chart-test@0".into()).unwrap()
    }

    fn test_chart() -> Vec<Account> {
        [
            ("3000", "Myynti", AccountKind::Income),
            ("7690", "Atk-kulut", AccountKind::Expense),
            ("6000", "Poistot", AccountKind::Expense),
            ("1910", "Pankkitili", AccountKind::Asset),
            ("1150", "Koneet ja kalusto", AccountKind::Asset),
            ("1160", "Kertyneet poistot", AccountKind::Asset),
            ("2000", "Osakepääoma", AccountKind::Equity),
            ("2251", "Tilikauden tulos", AccountKind::Equity),
        ]
        .into_iter()
        .map(|(id, name, kind)| Account {
            id: AccountId::new(id).unwrap(),
            name: name.into(),
            kind,
        })
        .collect()
    }

    #[test]
    fn income_statement_folds_revenue_and_expense_into_the_result() {
        // Sale 10000 (credit 3000 / debit 1910); atk-expense 2000 (debit 7690 /
        // credit 1910). Result = 10000 − 2000 = 8000.
        let vouchers = vec![
            voucher(
                1,
                "2026-03-31",
                &[
                    ("1910", "10000.00", Side::Debit),
                    ("3000", "10000.00", Side::Credit),
                ],
            ),
            voucher(
                2,
                "2026-06-30",
                &[
                    ("7690", "2000.00", Side::Debit),
                    ("1910", "2000.00", Side::Credit),
                ],
            ),
        ];
        let s = compute_statements(
            &y2026(),
            None,
            &vouchers,
            &test_chart(),
            kpa_policy(),
            prov(),
        )
        .unwrap();
        assert_eq!(s.result, "8000.00");
        let liikevaihto = s
            .income
            .iter()
            .find(|r| r.row_id == "IS-LIIKEVAIHTO")
            .unwrap();
        assert_eq!(liikevaihto.amount, "10000.00");
        let muut = s
            .income
            .iter()
            .find(|r| r.row_id == "IS-MUUT-KULUT")
            .unwrap();
        assert_eq!(muut.amount, "2000.00");
    }

    #[test]
    fn balance_sheet_identity_holds() {
        // Capital injection 5000 (debit 1910 / credit 2000); sale 10000; expense 2000.
        // Assets: 1910 = 5000 + 10000 − 2000 = 13000. Equity(2000) = 5000. Result = 8000.
        // 13000 == 5000 + 8000.
        let vouchers = vec![
            voucher(
                1,
                "2026-01-05",
                &[
                    ("1910", "5000.00", Side::Debit),
                    ("2000", "5000.00", Side::Credit),
                ],
            ),
            voucher(
                2,
                "2026-03-31",
                &[
                    ("1910", "10000.00", Side::Debit),
                    ("3000", "10000.00", Side::Credit),
                ],
            ),
            voucher(
                3,
                "2026-06-30",
                &[
                    ("7690", "2000.00", Side::Debit),
                    ("1910", "2000.00", Side::Credit),
                ],
            ),
        ];
        let s = compute_statements(
            &y2026(),
            None,
            &vouchers,
            &test_chart(),
            kpa_policy(),
            prov(),
        )
        .unwrap();
        assert!(s.balanced, "balance-sheet identity must hold: {s:?}");
        assert_eq!(s.assets_total, "13000.00");
        // equity+liabilities now folds in the year's result via the synthetic
        // "Tilikauden tulos" row, so the balance sheet actually sums to assets.
        assert_eq!(s.equity_liabilities_total, "13000.00");
        assert_eq!(s.result, "8000.00");
        let tulos = s
            .balance
            .iter()
            .find(|r| r.row_id == "BS-TILIKAUDEN-TULOS")
            .unwrap();
        assert_eq!(tulos.amount, "8000.00");
    }

    #[test]
    fn a_net_loss_produces_a_negative_result_without_panicking() {
        // Rebuts the "eur_str panics on negative Money" claim: expenses > revenue →
        // negative result, and the balance sheet still balances (result folded in).
        let vouchers = vec![
            voucher(
                1,
                "2026-01-05",
                &[
                    ("1910", "1000.00", Side::Debit),
                    ("2000", "1000.00", Side::Credit),
                ],
            ),
            voucher(
                2,
                "2026-03-31",
                &[
                    ("1910", "100.00", Side::Debit),
                    ("3000", "100.00", Side::Credit),
                ],
            ),
            voucher(
                3,
                "2026-06-30",
                &[
                    ("7690", "400.00", Side::Debit),
                    ("1910", "400.00", Side::Credit),
                ],
            ),
        ];
        let s = compute_statements(
            &y2026(),
            None,
            &vouchers,
            &test_chart(),
            kpa_policy(),
            prov(),
        )
        .unwrap();
        assert_eq!(s.result, "-300.00");
        assert!(s.balanced, "a loss year still balances: {s:?}");
        let tulos = s
            .balance
            .iter()
            .find(|r| r.row_id == "BS-TILIKAUDEN-TULOS")
            .unwrap();
        assert_eq!(tulos.amount, "-300.00");
    }

    #[test]
    fn a_kind_nature_mismatch_is_refused() {
        // An income account mapped (via chart kind) but whose KPA row nature disagrees
        // would invert the sign — reject rather than silently miscompute. Use account
        // 8300 declared Income in the chart; policy maps 8300 → IS-RAHOITUS-KULUT
        // (nature Expense).
        let mut chart = test_chart();
        chart.push(Account {
            id: AccountId::new("8300").unwrap(),
            name: "Korkokulut (miskinded)".into(),
            kind: AccountKind::Income,
        });
        let vouchers = vec![voucher(
            1,
            "2026-03-31",
            &[
                ("1910", "50.00", Side::Debit),
                ("8300", "50.00", Side::Credit),
            ],
        )];
        let err = compute_statements(&y2026(), None, &vouchers, &chart, kpa_policy(), prov())
            .unwrap_err();
        assert_eq!(err.error_code(), "STATEMENTS");
    }

    #[test]
    fn only_the_fiscal_year_is_folded() {
        let vouchers = vec![
            voucher(
                1,
                "2026-03-31",
                &[
                    ("1910", "1000.00", Side::Debit),
                    ("3000", "1000.00", Side::Credit),
                ],
            ),
            // 2027 — excluded.
            voucher(
                2,
                "2027-03-31",
                &[
                    ("1910", "9999.00", Side::Debit),
                    ("3000", "9999.00", Side::Credit),
                ],
            ),
        ];
        let s = compute_statements(
            &y2026(),
            None,
            &vouchers,
            &test_chart(),
            kpa_policy(),
            prov(),
        )
        .unwrap();
        assert_eq!(s.result, "1000.00");
    }

    #[test]
    fn a_july_to_june_year_folds_its_own_dates_and_states_them() {
        let year = FiscalYear::new(date("2026-07-01"), date("2027-06-30")).unwrap();
        let sale = |number, day| {
            voucher(
                number,
                day,
                &[
                    ("1910", "100.00", Side::Debit),
                    ("3000", "100.00", Side::Credit),
                ],
            )
        };
        let vouchers = vec![
            sale(1, "2026-06-30"),
            sale(2, "2026-07-01"),
            sale(3, "2027-06-30"),
            sale(4, "2027-07-01"),
        ];
        let s = compute_statements(&year, None, &vouchers, &test_chart(), kpa_policy(), prov())
            .unwrap();
        assert_eq!(s.fiscal_year, 2027);
        assert_eq!(s.result, "200.00");
        assert_eq!(
            s.period,
            Some(StatementPeriod {
                start: date("2026-07-01"),
                end: date("2027-06-30")
            })
        );
        let te = tase_erittelyt(&year, None, &vouchers, &test_chart(), prov()).unwrap();
        assert_eq!(te.lines[0].closing_balance, "200.00");

        // A calendar year serializes exactly as before fiscal-year definitions.
        let calendar = compute_statements(
            &y2026(),
            None,
            &vouchers,
            &test_chart(),
            kpa_policy(),
            prov(),
        )
        .unwrap();
        assert!(calendar.period.is_none());
        assert!(!String::from_utf8(calendar.canonical_bytes())
            .unwrap()
            .contains("\"period\""));
    }

    #[test]
    fn a_cutover_live_period_must_lie_in_the_opening_fiscal_year() {
        use aggountant_model::{Hash, PeriodId};
        let year = FiscalYear::new(date("2026-07-01"), date("2027-06-30")).unwrap();
        let opening = |live: &str| {
            OpeningBalance::new(
                2027,
                vec![
                    Posting {
                        account: AccountId::new("1910").unwrap(),
                        amount: eur_money("1.00"),
                        side: Side::Debit,
                        vat_code: None,
                    },
                    Posting {
                        account: AccountId::new("2000").unwrap(),
                        amount: eur_money("1.00"),
                        side: Side::Credit,
                        vat_code: None,
                    },
                ],
                OpeningBalanceSource::ExternalCutover {
                    prior_year: 2026,
                    source_head_hash: Hash::from_bytes([1; 32]),
                    source_chart_version: "old".into(),
                    target_chart_version: prov().chart_version,
                    account_mapping_hash: Hash::from_bytes([2; 32]),
                    live_period: PeriodId::parse(live).unwrap(),
                },
            )
            .unwrap()
        };
        let run = |live: &str| {
            compute_statements(
                &year,
                Some(&opening(live)),
                &[],
                &test_chart(),
                kpa_policy(),
                prov(),
            )
        };
        assert!(run("2026-09").is_ok());
        assert!(run("2027-Q2").is_ok());
        assert_eq!(run("2026-06").unwrap_err().error_code(), "STATEMENTS");
        assert_eq!(run("2027-Q3").unwrap_err().error_code(), "STATEMENTS");
    }

    #[test]
    fn accumulated_depreciation_reduces_net_assets() {
        // Buy machine 8000 (debit 1150 / credit 1910); depreciate 2000 (debit 6000 /
        // credit 1160). Assets: 1150=8000, 1160=−2000, 1910=−8000 → net −2000 asset
        // side; but 1910 also... let's keep it self-contained with a capital inject.
        let vouchers = vec![
            voucher(
                1,
                "2026-01-05",
                &[
                    ("1910", "8000.00", Side::Debit),
                    ("2000", "8000.00", Side::Credit),
                ],
            ),
            voucher(
                2,
                "2026-02-01",
                &[
                    ("1150", "8000.00", Side::Debit),
                    ("1910", "8000.00", Side::Credit),
                ],
            ),
            voucher(
                3,
                "2026-12-31",
                &[
                    ("6000", "2000.00", Side::Debit),
                    ("1160", "2000.00", Side::Credit),
                ],
            ),
        ];
        let s = compute_statements(
            &y2026(),
            None,
            &vouchers,
            &test_chart(),
            kpa_policy(),
            prov(),
        )
        .unwrap();
        // Aineelliset = 1150 (8000, debit-normal) + 1160 (−2000, credit balance on an
        // asset account) = 6000.
        let aineelliset = s
            .balance
            .iter()
            .find(|r| r.row_id == "BS-AINEELLISET")
            .unwrap();
        assert_eq!(aineelliset.amount, "6000.00");
        assert!(s.balanced);
    }

    #[test]
    fn a_golden_statement_reproduces_byte_for_byte() {
        let vouchers = vec![
            voucher(
                1,
                "2026-01-05",
                &[
                    ("1910", "5000.00", Side::Debit),
                    ("2000", "5000.00", Side::Credit),
                ],
            ),
            voucher(
                2,
                "2026-03-31",
                &[
                    ("1910", "10000.00", Side::Debit),
                    ("3000", "10000.00", Side::Credit),
                ],
            ),
            voucher(
                3,
                "2026-06-30",
                &[
                    ("7690", "2000.00", Side::Debit),
                    ("1910", "2000.00", Side::Credit),
                ],
            ),
        ];
        let s1 = compute_statements(
            &y2026(),
            None,
            &vouchers,
            &test_chart(),
            kpa_policy(),
            prov(),
        )
        .unwrap();
        // Same sealed inputs + pinned policy → identical bytes + hash.
        let s2 = compute_statements(
            &y2026(),
            None,
            &vouchers,
            &test_chart(),
            kpa_policy(),
            prov(),
        )
        .unwrap();
        assert_eq!(s1.canonical_bytes(), s2.canonical_bytes());
        assert_eq!(s1.statements_hash(), s2.statements_hash());
        // Pin the exact hash: any layout, label, formatting, or policy drift that
        // changes the sealed statement bytes breaks this literal (a real golden).
        assert_eq!(
            s1.statements_hash(),
            "fa3986337ce22d72d9c370b69c2ae8f8f276c54b448886c816d4106e2808c1f5",
            "golden statement hash drifted — update deliberately if the change is intended"
        );
    }

    #[test]
    fn an_unmapped_account_is_refused_not_silently_dropped() {
        // 9000 is not in any KPA row (and not in the chart) → loud failure.
        let vouchers = vec![voucher(
            1,
            "2026-03-31",
            &[
                ("1910", "100.00", Side::Debit),
                ("3000", "100.00", Side::Credit),
            ],
        )];
        let mut chart = test_chart();
        // Add an income account with no KPA prefix coverage.
        chart.push(Account {
            id: AccountId::new("3400").unwrap(),
            name: "Kummajainen".into(),
            kind: AccountKind::Income,
        });
        let vouchers2 = vec![voucher(
            2,
            "2026-03-31",
            &[
                ("1910", "50.00", Side::Debit),
                ("3400", "50.00", Side::Credit),
            ],
        )];
        let all: Vec<VoucherBody> = vouchers.into_iter().chain(vouchers2).collect();
        let err =
            compute_statements(&y2026(), None, &all, &chart, kpa_policy(), prov()).unwrap_err();
        assert_eq!(err.error_code(), "STATEMENTS");
    }

    #[test]
    fn tase_erittelyt_lists_balance_accounts_with_their_vouchers() {
        let vouchers = vec![
            voucher(
                1,
                "2026-01-05",
                &[
                    ("1910", "5000.00", Side::Debit),
                    ("2000", "5000.00", Side::Credit),
                ],
            ),
            voucher(
                2,
                "2026-03-31",
                &[
                    ("1910", "10000.00", Side::Debit),
                    ("3000", "10000.00", Side::Credit),
                ],
            ),
        ];
        let te = tase_erittelyt(&y2026(), None, &vouchers, &test_chart(), prov()).unwrap();
        let bank = te.lines.iter().find(|l| l.account == "1910").unwrap();
        assert_eq!(bank.closing_balance, "15000.00");
        assert_eq!(
            bank.source_refs,
            vec![
                StatementSourceRef::SealedVoucher { number: 1 },
                StatementSourceRef::SealedVoucher { number: 2 },
            ]
        );
        // 3000 is an income account → not in tase-erittelyt.
        assert!(te.lines.iter().all(|l| l.account != "3000"));
    }

    #[test]
    fn the_bundled_kpa_policy_loads_and_validates() {
        let p = kpa_policy();
        assert_eq!(p.policy, "fi-kpa-rows");
        validate_kpa_policy(p).unwrap();
    }

    #[test]
    fn a_prefix_claimed_by_two_rows_is_rejected() {
        let p = KpaPolicy {
            policy: "t".into(),
            version: "v".into(),
            rows: vec![
                KpaRow {
                    row_id: "A".into(),
                    statement: StatementKind::Income,
                    nature: RowNature::Revenue,
                    order: 1,
                    label: "A".into(),
                    accounts: vec!["30".into()],
                },
                KpaRow {
                    row_id: "B".into(),
                    statement: StatementKind::Income,
                    nature: RowNature::Revenue,
                    order: 2,
                    label: "B".into(),
                    accounts: vec!["30".into()],
                },
            ],
        };
        assert!(validate_kpa_policy(&p).is_err());
    }
}
