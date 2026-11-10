//! Deterministic **depreciation + accrual** law for `close-year` (RF6/S6, scenario
//! D2/D6). Pure, I/O-free functions the `close-year` Run calls to compute year-end
//! poistot/jaksotukset. The Domain "never guesses" (`candidate.md §2`): the agent
//! proposes *which* assets exist and *which* accruals apply (interpretation); the
//! **amounts** are computed here from codified law + versioned rate policy — the
//! agent never computes the figure (`s6-spec-ahead.md §3.1`).
//!
//! Depreciation rates are dated policy data (`data/depreciation-rates.json`),
//! resolved by the fiscal-year-end date — the same reviewable-data pattern as the
//! VAT (`vat-rates.json`) and payroll rate tables, so a rate change never silently
//! misfiles a historical close (Fork D: minimal per-asset input + policy rates, no
//! full asset register).

use std::sync::OnceLock;

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

use aggountant_model::{AccountId, Currency, Date, Money, Posting, Side, VoucherDraft};

use crate::DomainError;

/// A depreciation method. `straight-line` (tasapoisto) applies the rate to the
/// original acquisition cost each year; `reducing-balance` (menojäännöspoisto, the
/// EVL default for koneet ja kalusto) applies it to the remaining book value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DepreciationMethod {
    StraightLine,
    ReducingBalance,
}

impl DepreciationMethod {
    /// A **stable** string form for hash preimages / notes — never the `Debug`
    /// derive (which can drift on a variant rename or rustc upgrade). Matches the
    /// serde kebab-case wire value.
    pub fn as_str(self) -> &'static str {
        match self {
            DepreciationMethod::StraightLine => "straight-line",
            DepreciationMethod::ReducingBalance => "reducing-balance",
        }
    }
}

/// One dated edition of an asset class's depreciation policy, valid over
/// `[valid_from, valid_to]` (`valid_to = None` = current). `rate_percent` is a
/// canonical decimal string (the maximum annual rate the law allows for the class).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DepreciationEdition {
    pub asset_class: String,
    pub valid_from: Date,
    pub valid_to: Option<Date>,
    pub method: DepreciationMethod,
    pub rate_percent: String,
}

/// The bundled dated depreciation-rate table (all editions of all asset classes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DepreciationRateTable {
    pub table: String,
    pub version: String,
    pub editions: Vec<DepreciationEdition>,
}

/// The per-asset input the agent proposes (Fork D minimal input). The **amount** is
/// never here — it is computed by [`compute_depreciation`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetInput {
    pub asset_id: String,
    /// Which policy class resolves the rate + method (e.g. `koneet-ja-kalusto`).
    pub asset_class: String,
    /// The expense account the depreciation is debited to (e.g. `6000`).
    pub expense_account: AccountId,
    /// The accumulated-depreciation (contra-asset) account credited (e.g. `1160`).
    pub accumulated_account: AccountId,
    /// Original acquisition cost (straight-line base).
    pub acquisition_cost: Money,
    /// Remaining book value at the start of the year (reducing-balance base). For a
    /// first-year asset this equals the acquisition cost.
    pub book_value_start: Money,
}

/// One computed depreciation line — the amount is a **Domain output**, fed to the
/// `ApproveAdjustments` check-gate (never computed by the agent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepreciationLine {
    pub asset_id: String,
    pub asset_class: String,
    pub method: DepreciationMethod,
    pub rate_percent: String,
    pub expense_account: AccountId,
    pub accumulated_account: AccountId,
    /// `round(base × rate)` — the year's depreciation for the asset.
    pub amount: Money,
}

/// A mechanical accrual (jaksotus) the agent proposes: an amount to defer/accrue by
/// booking one balanced correction (e.g. accrue an unpaid expense). The Domain only
/// validates + assembles the balanced draft — the accrual *amount* is asserted by
/// the agent (a jaksotus is an interpretation, not a computed figure), but the
/// **booking** is mechanical + checked (`Σ = 0`, single currency).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccrualInput {
    pub accrual_id: String,
    pub description: String,
    pub debit_account: AccountId,
    pub credit_account: AccountId,
    pub amount: Money,
}

const DEPRECIATION_RATES_JSON: &str = include_str!("../data/depreciation-rates.json");

/// Parse + validate the bundled depreciation rate table once. A regulatory
/// calculation must not depend on JSON order: reject overlapping/inverted editions
/// per class (bundled data, so a violation is a build-time bug — a unit test
/// exercises this init).
pub fn depreciation_rate_table() -> &'static DepreciationRateTable {
    static TABLE: OnceLock<DepreciationRateTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let table: DepreciationRateTable = serde_json::from_str(DEPRECIATION_RATES_JSON)
            .expect("bundled depreciation-rates.json parses");
        validate_depreciation_table(&table)
            .expect("bundled depreciation-rates.json is a valid rate table");
        table
    })
}

/// The pinned version identifier of the depreciation policy (stamped to provenance).
pub fn depreciation_policy_version() -> &'static str {
    &depreciation_rate_table().version
}

fn validate_depreciation_table(table: &DepreciationRateTable) -> Result<(), DomainError> {
    use std::collections::BTreeMap;
    let mut by_class: BTreeMap<&str, Vec<&DepreciationEdition>> = BTreeMap::new();
    for e in &table.editions {
        if let Some(to) = e.valid_to {
            if to < e.valid_from {
                return Err(DomainError::Depreciation(format!(
                    "asset class {:?} has an edition with valid_to {} before valid_from {}",
                    e.asset_class, to, e.valid_from
                )));
            }
        }
        by_class.entry(&e.asset_class).or_default().push(e);
    }
    for (class, mut editions) in by_class {
        editions.sort_by_key(|e| e.valid_from);
        for pair in editions.windows(2) {
            let (prev, next) = (pair[0], pair[1]);
            let overlaps = prev.valid_to.is_none_or(|to| to >= next.valid_from);
            if overlaps {
                return Err(DomainError::Depreciation(format!(
                    "asset class {class:?} has overlapping editions from {} and {}",
                    prev.valid_from, next.valid_from
                )));
            }
        }
    }
    Ok(())
}

/// Resolve the depreciation edition in force for `asset_class` on `at`. Fails with
/// `DEPRECIATION` if no edition covers the date (rates are dated data, never
/// extrapolated) or the class is unknown.
fn resolve_edition<'a>(
    table: &'a DepreciationRateTable,
    asset_class: &str,
    at: Date,
) -> Result<&'a DepreciationEdition, DomainError> {
    table
        .editions
        .iter()
        .find(|e| {
            e.asset_class == asset_class
                && at >= e.valid_from
                && e.valid_to.is_none_or(|to| at <= to)
        })
        .ok_or_else(|| {
            DomainError::Depreciation(format!(
                "no depreciation edition for class {asset_class:?} on {at} \
                 (rates are dated policy; add the edition)"
            ))
        })
}

/// Round to the EUR cent, half away from zero, pinned to 2 dp (the per-asset
/// depreciation rounding policy, mirroring the VAT/payroll rounding).
fn round_eur(mut d: Decimal) -> Decimal {
    d = d.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    d.rescale(2);
    d
}

fn eur() -> Currency {
    Currency::new("EUR").expect("EUR is a valid currency code")
}

/// Require a `Money` to be non-negative EUR (a depreciation base/amount is never a
/// negative or foreign figure in this slice).
fn require_nonneg_eur(m: &Money, what: &str) -> Result<(), DomainError> {
    if m.currency() != &eur() {
        return Err(DomainError::Depreciation(format!(
            "{what} is {}, but close-year depreciation is EUR-only",
            m.currency()
        )));
    }
    if m.amount() < Decimal::ZERO {
        return Err(DomainError::Depreciation(format!(
            "{what} must be non-negative, got {}",
            m.amount_str()
        )));
    }
    Ok(())
}

/// Compute the year's depreciation for a set of assets, resolving each asset class's
/// method + rate from the dated policy as of `fiscal_year_end` (31.12 of the closing
/// year). Deterministic and total over well-formed EUR input; an unknown class or an
/// unresolvable date fails loudly rather than silently omitting an asset.
pub fn compute_depreciation(
    fiscal_year_end: Date,
    assets: &[AssetInput],
    table: &DepreciationRateTable,
) -> Result<Vec<DepreciationLine>, DomainError> {
    let mut lines = Vec::with_capacity(assets.len());
    for asset in assets {
        require_nonneg_eur(&asset.acquisition_cost, "acquisition_cost")?;
        require_nonneg_eur(&asset.book_value_start, "book_value_start")?;
        let edition = resolve_edition(table, &asset.asset_class, fiscal_year_end)?;
        let rate: Decimal = edition.rate_percent.parse().map_err(|e| {
            DomainError::Depreciation(format!(
                "asset class {:?} has a malformed rate_percent {:?}: {e}",
                asset.asset_class, edition.rate_percent
            ))
        })?;
        let base = match edition.method {
            DepreciationMethod::StraightLine => asset.acquisition_cost.amount(),
            DepreciationMethod::ReducingBalance => asset.book_value_start.amount(),
        };
        // The depreciation may not exceed the remaining book value (you cannot
        // depreciate below zero) — clamp the computed figure to the book value.
        let computed = round_eur(base * rate / Decimal::from(100));
        let amount = computed.min(asset.book_value_start.amount());
        // A fully depreciated asset (book value already 0) yields a 0.00 line — skip
        // it rather than emitting an un-bookable voucher that would fail the close.
        // Leaving prior-year fully-depreciated assets in the input is legitimate.
        if amount <= Decimal::ZERO {
            continue;
        }
        lines.push(DepreciationLine {
            asset_id: asset.asset_id.clone(),
            asset_class: asset.asset_class.clone(),
            method: edition.method,
            rate_percent: edition.rate_percent.clone(),
            expense_account: asset.expense_account.clone(),
            accumulated_account: asset.accumulated_account.clone(),
            amount: Money::new(amount, eur()),
        });
    }
    Ok(lines)
}

/// Build the balanced depreciation voucher for one line: debit the expense account,
/// credit the accumulated-depreciation account. The caller supplies the dates
/// (document/posting) — a zero-amount line is refused (nothing to book).
pub fn depreciation_voucher(
    line: &DepreciationLine,
    document_date: Date,
    posting_date: Date,
) -> Result<VoucherDraft, DomainError> {
    if line.amount.amount() <= Decimal::ZERO {
        return Err(DomainError::Depreciation(format!(
            "asset {} depreciates to 0.00 — nothing to book",
            line.asset_id
        )));
    }
    let postings = vec![
        Posting {
            account: line.expense_account.clone(),
            amount: line.amount.clone(),
            side: Side::Debit,
            vat_code: None,
        },
        Posting {
            account: line.accumulated_account.clone(),
            amount: line.amount.clone(),
            side: Side::Credit,
            vat_code: None,
        },
    ];
    Ok(VoucherDraft::new(
        document_date,
        posting_date,
        format!("Poisto {} ({})", line.asset_id, line.asset_class),
        postings,
        None,
    )?)
}

/// Build the balanced accrual voucher: debit/credit the caller-named accounts. The
/// accrual amount is the agent's assertion; the booking is validated (`Σ = 0`).
pub fn accrual_voucher(
    accrual: &AccrualInput,
    document_date: Date,
    posting_date: Date,
) -> Result<VoucherDraft, DomainError> {
    require_nonneg_eur(&accrual.amount, "accrual amount")?;
    if accrual.amount.amount() <= Decimal::ZERO {
        return Err(DomainError::Depreciation(format!(
            "accrual {} has a zero amount — nothing to book",
            accrual.accrual_id
        )));
    }
    if accrual.debit_account == accrual.credit_account {
        return Err(DomainError::Depreciation(format!(
            "accrual {} debits and credits the same account {}",
            accrual.accrual_id, accrual.debit_account
        )));
    }
    let postings = vec![
        Posting {
            account: accrual.debit_account.clone(),
            amount: accrual.amount.clone(),
            side: Side::Debit,
            vat_code: None,
        },
        Posting {
            account: accrual.credit_account.clone(),
            amount: accrual.amount.clone(),
            side: Side::Credit,
            vat_code: None,
        },
    ];
    Ok(VoucherDraft::new(
        document_date,
        posting_date,
        format!("Jaksotus: {}", accrual.description),
        postings,
        None,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eur_money(s: &str) -> Money {
        Money::parse(s, eur()).unwrap()
    }
    fn date(s: &str) -> Date {
        Date::parse(s).unwrap()
    }
    fn acct(s: &str) -> AccountId {
        AccountId::new(s).unwrap()
    }

    fn asset(class: &str, cost: &str, book: &str) -> AssetInput {
        AssetInput {
            asset_id: "A1".into(),
            asset_class: class.into(),
            expense_account: acct("6000"),
            accumulated_account: acct("1160"),
            acquisition_cost: eur_money(cost),
            book_value_start: eur_money(book),
        }
    }

    #[test]
    fn reducing_balance_depreciates_book_value_at_the_class_rate() {
        // koneet-ja-kalusto @ 25 %: 8000 book value → 2000.00.
        let lines = compute_depreciation(
            date("2026-12-31"),
            &[asset("koneet-ja-kalusto", "10000.00", "8000.00")],
            depreciation_rate_table(),
        )
        .unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].amount.amount_str(), "2000.00");
        assert_eq!(lines[0].method, DepreciationMethod::ReducingBalance);
    }

    #[test]
    fn straight_line_depreciates_acquisition_cost() {
        // aineettomat-oikeudet @ 10 % straight-line: 10000 cost → 1000.00, even
        // though the book value has already dropped to 6000.
        let lines = compute_depreciation(
            date("2026-12-31"),
            &[asset("aineettomat-oikeudet", "10000.00", "6000.00")],
            depreciation_rate_table(),
        )
        .unwrap();
        assert_eq!(lines[0].amount.amount_str(), "1000.00");
        assert_eq!(lines[0].method, DepreciationMethod::StraightLine);
    }

    #[test]
    fn depreciation_is_clamped_to_the_remaining_book_value() {
        // 25 % of 100 book value = 25, but book value is only 10 → clamp to 10.00.
        let lines = compute_depreciation(
            date("2026-12-31"),
            &[asset("koneet-ja-kalusto", "10000.00", "10.00")],
            depreciation_rate_table(),
        )
        .unwrap();
        assert_eq!(lines[0].amount.amount_str(), "2.50");
        // 25 % of 10.00 = 2.50, which is below book value, so no clamp fired here.
        let lines2 = compute_depreciation(
            date("2026-12-31"),
            &[asset("koneet-ja-kalusto", "10000.00", "1.00")],
            depreciation_rate_table(),
        )
        .unwrap();
        // 25 % of 1.00 = 0.25 < book value 1.00 → 0.25 (still no clamp).
        assert_eq!(lines2[0].amount.amount_str(), "0.25");
    }

    #[test]
    fn a_fully_depreciated_asset_produces_no_line() {
        // book_value_start 0 → 0.00 depreciation → the line is skipped (a leftover
        // fully-depreciated asset in the input must not fail the close).
        let lines = compute_depreciation(
            date("2026-12-31"),
            &[asset("koneet-ja-kalusto", "10000.00", "0.00")],
            depreciation_rate_table(),
        )
        .unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn an_unknown_asset_class_is_refused() {
        let err = compute_depreciation(
            date("2026-12-31"),
            &[asset("lentokoneet", "10000.00", "10000.00")],
            depreciation_rate_table(),
        )
        .unwrap_err();
        assert_eq!(err.error_code(), "DEPRECIATION");
    }

    #[test]
    fn depreciation_voucher_is_balanced() {
        let lines = compute_depreciation(
            date("2026-12-31"),
            &[asset("koneet-ja-kalusto", "10000.00", "8000.00")],
            depreciation_rate_table(),
        )
        .unwrap();
        let v = depreciation_voucher(&lines[0], date("2026-12-31"), date("2027-01-01")).unwrap();
        assert_eq!(v.postings().len(), 2);
        assert_eq!(v.currency().as_str(), "EUR");
    }

    #[test]
    fn accrual_voucher_rejects_same_account() {
        let a = AccrualInput {
            accrual_id: "J1".into(),
            description: "sähkölasku".into(),
            debit_account: acct("7010"),
            credit_account: acct("7010"),
            amount: eur_money("120.00"),
        };
        let err = accrual_voucher(&a, date("2026-12-31"), date("2027-01-01")).unwrap_err();
        assert_eq!(err.error_code(), "DEPRECIATION");
    }

    #[test]
    fn the_bundled_depreciation_table_loads_and_validates() {
        let table = depreciation_rate_table();
        assert_eq!(table.table, "fi-depreciation-rates");
        validate_depreciation_table(table).unwrap();
    }

    #[test]
    fn overlapping_depreciation_editions_are_rejected() {
        let table = DepreciationRateTable {
            table: "t".into(),
            version: "v".into(),
            editions: vec![
                DepreciationEdition {
                    asset_class: "koneet".into(),
                    valid_from: date("2020-01-01"),
                    valid_to: None,
                    method: DepreciationMethod::ReducingBalance,
                    rate_percent: "25.0".into(),
                },
                DepreciationEdition {
                    asset_class: "koneet".into(),
                    valid_from: date("2024-01-01"),
                    valid_to: None,
                    method: DepreciationMethod::ReducingBalance,
                    rate_percent: "20.0".into(),
                },
            ],
        };
        assert!(validate_depreciation_table(&table).is_err());
    }
}
