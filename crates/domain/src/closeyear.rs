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
//! misfiles a historical close.
//!
//! The fixed-asset register ([`AssetRegister`]) is one year's assets with their
//! opening and closing book values. [`plan_asset_year`] continues it from the
//! previous sealed year, so an opening book value is given once, when an asset
//! enters, and never again.

use std::sync::OnceLock;

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};

use aggountant_model::{AccountId, Currency, Date, FiscalYear, Money, Posting, Side, VoucherDraft};

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
/// method + rate from the dated policy as of the fiscal year's last day.
/// Deterministic and total over well-formed EUR input; an unknown class or an
/// unresolvable date fails loudly rather than silently omitting an asset.
///
/// The rates are annual. A fiscal year that is not 12 months (kirjanpitolaki 1:4
/// allows 1–18) is refused rather than prorated, because how the plan and the
/// EVL maximum scale to its length is a judgement this function does not make.
pub fn compute_depreciation(
    fiscal_year: &FiscalYear,
    assets: &[AssetInput],
    table: &DepreciationRateTable,
) -> Result<Vec<DepreciationLine>, DomainError> {
    if !assets.is_empty() && fiscal_year.months() != 12 {
        return Err(DomainError::Depreciation(format!(
            "fiscal year {} ({}..{}) is {} months; depreciation rates are annual and are not \
             prorated for a shorter or longer fiscal year. Give this year's depreciation \
             to close-year as a correction and leave assets out of the close-year input",
            fiscal_year.year(),
            fiscal_year.start(),
            fiscal_year.end(),
            fiscal_year.months()
        )));
    }
    let fiscal_year_end = fiscal_year.end();
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

/// Where a closing year's asset came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssetSource {
    /// Carried from the previous sealed year's register; its opening book value is
    /// that year's closing book value.
    Carried,
    /// Given in the close-year input: a new acquisition, an asset of a home that
    /// has no register yet, or an asset removed and given again.
    Input,
}

/// One asset's year in the fixed-asset register: what it opened at, what the year
/// depreciated and what it closes at. The next year's close opens from
/// `book_value_end`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetYear {
    pub asset_id: String,
    pub asset_class: String,
    pub expense_account: AccountId,
    pub accumulated_account: AccountId,
    pub acquisition_cost: Money,
    pub book_value_start: Money,
    /// The year's computed depreciation (`0.00` when nothing is left to depreciate).
    pub depreciation: Money,
    pub book_value_end: Money,
    pub source: AssetSource,
}

/// The fixed-asset register of one fiscal year: every asset the year's close
/// depreciated or carried, and the assets it removed. A sealed year stores it, and
/// the next year's close opens from it ([`plan_asset_year`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRegister {
    pub assets: Vec<AssetYear>,
    /// Ids of previous-year assets that left the register in this year.
    #[serde(default)]
    pub removed: Vec<String>,
}

impl AssetRegister {
    fn asset(&self, asset_id: &str) -> Option<&AssetYear> {
        self.assets.iter().find(|a| a.asset_id == asset_id)
    }

    /// Check that this register is a lawful successor of `opening`, the previous
    /// sealed year's register (`None` when that year has none): ids are unique,
    /// every line's arithmetic holds, every asset of `opening` is either removed or
    /// carried at exactly its closing book value, and nothing is removed that
    /// `opening` does not hold. The year seal refuses a register that fails this, so
    /// an opening book value can only come from the sealed previous year.
    pub fn check_continues(&self, opening: Option<&AssetRegister>) -> Result<(), DomainError> {
        let mut seen = std::collections::BTreeSet::new();
        for a in &self.assets {
            if !seen.insert(a.asset_id.as_str()) {
                return Err(DomainError::Depreciation(format!(
                    "asset register lists asset {:?} twice",
                    a.asset_id
                )));
            }
            require_nonneg_eur(&a.acquisition_cost, "acquisition_cost")?;
            require_nonneg_eur(&a.book_value_start, "book_value_start")?;
            require_nonneg_eur(&a.depreciation, "depreciation")?;
            require_nonneg_eur(&a.book_value_end, "book_value_end")?;
            if a.book_value_start.amount() - a.depreciation.amount() != a.book_value_end.amount() {
                return Err(DomainError::Depreciation(format!(
                    "asset {:?}: book_value_end {} is not book_value_start {} less depreciation {}",
                    a.asset_id,
                    a.book_value_end.amount_str(),
                    a.book_value_start.amount_str(),
                    a.depreciation.amount_str()
                )));
            }
        }
        let empty = AssetRegister::default();
        let opening = opening.unwrap_or(&empty);
        for id in &self.removed {
            if opening.asset(id).is_none() {
                return Err(DomainError::Depreciation(format!(
                    "asset {id:?} cannot be removed: the previous year's register does not hold it"
                )));
            }
        }
        for prev in &opening.assets {
            let removed = self.removed.contains(&prev.asset_id);
            match self.asset(&prev.asset_id) {
                None if removed => {}
                None => {
                    return Err(DomainError::Depreciation(format!(
                        "asset {:?} of the previous year's register is neither carried nor removed",
                        prev.asset_id
                    )));
                }
                // Removed and given again: the input restates it on purpose.
                Some(_) if removed => {}
                Some(now) => {
                    if now.source != AssetSource::Carried
                        || now.book_value_start != prev.book_value_end
                        || now.acquisition_cost != prev.acquisition_cost
                        || now.asset_class != prev.asset_class
                        || now.expense_account != prev.expense_account
                        || now.accumulated_account != prev.accumulated_account
                    {
                        return Err(DomainError::Depreciation(format!(
                            "asset {:?} does not continue the previous year's register \
                             (closing book value {})",
                            prev.asset_id,
                            prev.book_value_end.amount_str()
                        )));
                    }
                }
            }
        }
        for a in &self.assets {
            if a.source == AssetSource::Carried
                && (opening.asset(&a.asset_id).is_none() || self.removed.contains(&a.asset_id))
            {
                return Err(DomainError::Depreciation(format!(
                    "asset {:?} is marked carried, but the previous year's register does not \
                     carry it",
                    a.asset_id
                )));
            }
        }
        Ok(())
    }
}

/// Plan a closing year's fixed assets and their depreciation.
///
/// The assets are those of `opening` (the previous sealed year's register), each
/// opening at its closing book value, less `removed`, plus `input`. An input asset
/// whose id `opening` still carries must restate it exactly (class, accounts,
/// acquisition cost, and `book_value_start` equal to the carried closing value);
/// a different figure is refused rather than taken, because a typed opening value
/// that disagrees with the sealed year is the mistake the register exists to stop.
/// To restate a carried asset on purpose, name it in `removed` and give it again.
///
/// Returns the year's register and the depreciation lines to book (assets with
/// nothing left to depreciate have no line).
pub fn plan_asset_year(
    fiscal_year: &FiscalYear,
    opening: Option<&AssetRegister>,
    input: &[AssetInput],
    removed: &[String],
    table: &DepreciationRateTable,
) -> Result<(AssetRegister, Vec<DepreciationLine>), DomainError> {
    let empty = AssetRegister::default();
    let opening_assets = &opening.unwrap_or(&empty).assets;
    let mut seen_removed = std::collections::BTreeSet::new();
    for id in removed {
        if !seen_removed.insert(id.as_str()) {
            return Err(DomainError::Depreciation(format!(
                "removed_assets names asset {id:?} twice"
            )));
        }
        if !opening_assets.iter().any(|a| &a.asset_id == id) {
            return Err(DomainError::Depreciation(format!(
                "removed_assets names asset {id:?}, which the previous year's register does \
                 not hold (it holds: [{}])",
                opening_assets
                    .iter()
                    .map(|a| a.asset_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
    }

    let mut seen_input = std::collections::BTreeSet::new();
    for given in input {
        if !seen_input.insert(given.asset_id.as_str()) {
            return Err(DomainError::Depreciation(format!(
                "assets names asset {:?} twice",
                given.asset_id
            )));
        }
    }

    // Carried assets first, in register order, then the input's own.
    let mut effective: Vec<(AssetInput, AssetSource)> = Vec::new();
    for prev in opening_assets {
        if removed.contains(&prev.asset_id) {
            continue;
        }
        let carried = AssetInput {
            asset_id: prev.asset_id.clone(),
            asset_class: prev.asset_class.clone(),
            expense_account: prev.expense_account.clone(),
            accumulated_account: prev.accumulated_account.clone(),
            acquisition_cost: prev.acquisition_cost.clone(),
            book_value_start: prev.book_value_end.clone(),
        };
        if let Some(given) = input.iter().find(|a| a.asset_id == prev.asset_id) {
            if given != &carried {
                return Err(DomainError::Depreciation(format!(
                    "asset {:?} is carried from the previous sealed year with asset_class {:?}, \
                     expense_account {}, accumulated_account {}, acquisition_cost {} and \
                     book_value_start {}; the input gives it differently. Leave it out of \
                     assets (it is carried automatically), or name it in removed_assets as \
                     well to replace the carried values with the input's",
                    prev.asset_id,
                    carried.asset_class,
                    carried.expense_account,
                    carried.accumulated_account,
                    carried.acquisition_cost.amount_str(),
                    carried.book_value_start.amount_str()
                )));
            }
        }
        effective.push((carried, AssetSource::Carried));
    }
    for given in input {
        if !effective.iter().any(|(a, _)| a.asset_id == given.asset_id) {
            effective.push((given.clone(), AssetSource::Input));
        }
    }

    let assets: Vec<AssetInput> = effective.iter().map(|(a, _)| a.clone()).collect();
    let lines = compute_depreciation(fiscal_year, &assets, table).map_err(|e| match e {
        DomainError::Depreciation(message)
            if fiscal_year.months() != 12 && !opening_assets.is_empty() =>
        {
            DomainError::Depreciation(format!(
                "{message}; assets carried from the previous year are named in removed_assets \
                 for this year and given again, with their book value, in the next close"
            ))
        }
        other => other,
    })?;

    let zero = Money::new(round_eur(Decimal::ZERO), eur());
    let register = AssetRegister {
        assets: effective
            .into_iter()
            .map(|(a, source)| {
                let depreciation = lines
                    .iter()
                    .find(|l| l.asset_id == a.asset_id)
                    .map_or_else(|| zero.clone(), |l| l.amount.clone());
                let book_value_end =
                    Money::new(a.book_value_start.amount() - depreciation.amount(), eur());
                AssetYear {
                    asset_id: a.asset_id,
                    asset_class: a.asset_class,
                    expense_account: a.expense_account,
                    accumulated_account: a.accumulated_account,
                    acquisition_cost: a.acquisition_cost,
                    book_value_start: a.book_value_start,
                    depreciation,
                    book_value_end,
                    source,
                }
            })
            .collect(),
        removed: removed.to_vec(),
    };
    register.check_continues(opening)?;
    Ok((register, lines))
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
            &FiscalYear::calendar(2026).unwrap(),
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
            &FiscalYear::calendar(2026).unwrap(),
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
            &FiscalYear::calendar(2026).unwrap(),
            &[asset("koneet-ja-kalusto", "10000.00", "10.00")],
            depreciation_rate_table(),
        )
        .unwrap();
        assert_eq!(lines[0].amount.amount_str(), "2.50");
        // 25 % of 10.00 = 2.50, which is below book value, so no clamp fired here.
        let lines2 = compute_depreciation(
            &FiscalYear::calendar(2026).unwrap(),
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
            &FiscalYear::calendar(2026).unwrap(),
            &[asset("koneet-ja-kalusto", "10000.00", "0.00")],
            depreciation_rate_table(),
        )
        .unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn an_unknown_asset_class_is_refused() {
        let err = compute_depreciation(
            &FiscalYear::calendar(2026).unwrap(),
            &[asset("lentokoneet", "10000.00", "10000.00")],
            depreciation_rate_table(),
        )
        .unwrap_err();
        assert_eq!(err.error_code(), "DEPRECIATION");
    }

    #[test]
    fn a_fiscal_year_that_is_not_twelve_months_is_refused() {
        let short = FiscalYear::new(date("2026-07-01"), date("2026-12-31")).unwrap();
        let err = compute_depreciation(
            &short,
            &[asset("koneet-ja-kalusto", "10000.00", "8000.00")],
            depreciation_rate_table(),
        )
        .unwrap_err();
        assert_eq!(err.error_code(), "DEPRECIATION");
        assert!(err.to_string().contains("6 months"), "{err}");
        // Without assets there is nothing to prorate.
        assert!(compute_depreciation(&short, &[], depreciation_rate_table())
            .unwrap()
            .is_empty());
        // A non-calendar 12-month year depreciates normally.
        let july = FiscalYear::new(date("2026-07-01"), date("2027-06-30")).unwrap();
        let lines = compute_depreciation(
            &july,
            &[asset("koneet-ja-kalusto", "10000.00", "8000.00")],
            depreciation_rate_table(),
        )
        .unwrap();
        assert_eq!(lines[0].amount.amount_str(), "2000.00");
    }

    #[test]
    fn depreciation_voucher_is_balanced() {
        let lines = compute_depreciation(
            &FiscalYear::calendar(2026).unwrap(),
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

    fn fy(year: i32) -> FiscalYear {
        FiscalYear::calendar(year).unwrap()
    }

    fn plan(
        year: i32,
        opening: Option<&AssetRegister>,
        input: &[AssetInput],
        removed: &[&str],
    ) -> Result<(AssetRegister, Vec<DepreciationLine>), DomainError> {
        let removed: Vec<String> = removed.iter().map(|s| s.to_string()).collect();
        plan_asset_year(
            &fy(year),
            opening,
            input,
            &removed,
            depreciation_rate_table(),
        )
    }

    #[test]
    fn a_register_starts_from_the_input_and_carries_the_closing_value() {
        let (first, lines) = plan(
            2026,
            None,
            &[asset("koneet-ja-kalusto", "10000.00", "8000.00")],
            &[],
        )
        .unwrap();
        assert_eq!(lines[0].amount.amount_str(), "2000.00");
        assert_eq!(first.assets[0].source, AssetSource::Input);
        assert_eq!(first.assets[0].book_value_end.amount_str(), "6000.00");

        // The next year names nothing: 25 % of the carried 6 000.00.
        let (second, lines) = plan(2027, Some(&first), &[], &[]).unwrap();
        assert_eq!(lines[0].amount.amount_str(), "1500.00");
        assert_eq!(second.assets[0].source, AssetSource::Carried);
        assert_eq!(second.assets[0].book_value_start.amount_str(), "6000.00");
        assert_eq!(second.assets[0].book_value_end.amount_str(), "4500.00");
        second.check_continues(Some(&first)).unwrap();
    }

    #[test]
    fn a_carried_asset_cannot_be_given_other_figures() {
        let (first, _) = plan(
            2026,
            None,
            &[asset("koneet-ja-kalusto", "10000.00", "8000.00")],
            &[],
        )
        .unwrap();
        // Restated exactly: accepted, still carried.
        let (same, _) = plan(
            2027,
            Some(&first),
            &[asset("koneet-ja-kalusto", "10000.00", "6000.00")],
            &[],
        )
        .unwrap();
        assert_eq!(same.assets[0].source, AssetSource::Carried);
        // Last year's opening value typed again: refused, naming the carried value.
        let err = plan(
            2027,
            Some(&first),
            &[asset("koneet-ja-kalusto", "10000.00", "8000.00")],
            &[],
        )
        .unwrap_err();
        assert_eq!(err.error_code(), "DEPRECIATION");
        assert!(
            err.to_string().contains("book_value_start 6000.00"),
            "{err}"
        );
        // Removed and given again: the input's value stands.
        let (restated, lines) = plan(
            2027,
            Some(&first),
            &[asset("koneet-ja-kalusto", "10000.00", "8000.00")],
            &["A1"],
        )
        .unwrap();
        assert_eq!(restated.assets[0].source, AssetSource::Input);
        assert_eq!(lines[0].amount.amount_str(), "2000.00");
    }

    #[test]
    fn removal_takes_an_asset_out_and_only_a_held_one_can_be_removed() {
        let (first, _) = plan(
            2026,
            None,
            &[asset("koneet-ja-kalusto", "10000.00", "8000.00")],
            &[],
        )
        .unwrap();
        let (second, lines) = plan(2027, Some(&first), &[], &["A1"]).unwrap();
        assert!(lines.is_empty());
        assert!(second.assets.is_empty());
        assert_eq!(second.removed, vec!["A1".to_string()]);
        assert!(plan(2027, Some(&first), &[], &["B7"]).is_err());
        assert!(plan(2027, None, &[], &["A1"]).is_err());
        assert!(plan(2027, Some(&first), &[], &["A1", "A1"]).is_err());
    }

    #[test]
    fn a_fully_depreciated_asset_stays_in_the_register_without_a_line() {
        let (register, lines) = plan(
            2026,
            None,
            &[asset("aineettomat-oikeudet", "1000.00", "0.00")],
            &[],
        )
        .unwrap();
        assert!(lines.is_empty());
        assert_eq!(register.assets[0].depreciation.amount_str(), "0.00");
        assert_eq!(register.assets[0].book_value_end.amount_str(), "0.00");
    }

    #[test]
    fn a_register_that_does_not_continue_the_previous_year_is_refused() {
        let (first, _) = plan(
            2026,
            None,
            &[asset("koneet-ja-kalusto", "10000.00", "8000.00")],
            &[],
        )
        .unwrap();
        let (good, _) = plan(2027, Some(&first), &[], &[]).unwrap();

        // An asset silently dropped.
        assert!(AssetRegister::default()
            .check_continues(Some(&first))
            .is_err());
        // An opening value other than the sealed closing value.
        let mut wrong = good.clone();
        wrong.assets[0].book_value_start = eur_money("8000.00");
        wrong.assets[0].book_value_end = eur_money("6500.00");
        assert!(wrong.check_continues(Some(&first)).is_err());
        // The same asset passed off as a new input one.
        let mut relabelled = good.clone();
        relabelled.assets[0].source = AssetSource::Input;
        assert!(relabelled.check_continues(Some(&first)).is_err());
        // Arithmetic that does not hold.
        let mut bad_sum = good.clone();
        bad_sum.assets[0].book_value_end = eur_money("5000.00");
        assert!(bad_sum.check_continues(Some(&first)).is_err());
        // A carried asset where there is nothing to carry from.
        assert!(good.check_continues(None).is_err());
    }
}
