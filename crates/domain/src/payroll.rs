//! Deterministic payroll law (F4, scenario C1 / RF4). Pure, I/O-free functions the
//! `run-payroll` Run (RF4) calls: resolve the dated statutory rates for a pay-run,
//! compute one employee's payslip **deterministically** from a tax card + gross,
//! assemble the balanced payroll voucher, and model the two Tulorekisteri report
//! kinds. The Domain "never guesses" (`candidate.md §2`): the agent supplies a
//! `PayrollIntent` (period, `payment_date`, per-employee gross + tulolaji) and Rust
//! computes every statutory amount — **no AI-computed withholding/TyEL** (the
//! AI/Rust boundary, must-spec #6).
//!
//! ## Dated rate tables as data (must-spec #1)
//! [`RateTable`] is loaded from a bundled JSON data file (`data/payroll-rates.json`),
//! the same dated-data-file pattern as the chart of accounts — statutory rates change
//! annually, so they are reviewable data, not compiled code. [`resolve_rates`] picks
//! the edition whose `[valid_from, valid_to]` contains the `payment_date`.
//!
//! ## Rounding policy (must-spec #7)
//! Every percentage deduction is rounded to the EUR cent with
//! **half-away-from-zero** (`RoundingStrategy::MidpointAwayFromZero`) at 2 dp, then
//! `net = gross − Σ(employee deductions)` is an exact subtraction of 2-dp amounts.
//! The rounding is applied once per line, deterministically; see [`round_eur`].

use std::sync::OnceLock;

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use aggountant_model::{AccountId, Currency, Date, Money, Posting, Side, VoucherDraft};

use crate::DomainError;

// ---------------------------------------------------------------------------
// Chart accounts the payroll voucher footprint uses (must-spec #3). Kept as
// named constants so the footprint reads like the double-entry it is.
// ---------------------------------------------------------------------------

/// Debit — Palkat ja palkkiot (gross pay expense).
const ACC_GROSS_EXPENSE: &str = "5000";
/// Debit — Eläkekulut (employer TyEL expense).
const ACC_EMPLOYER_TYEL_EXPENSE: &str = "5400";
/// Debit — Muut henkilösivukulut (employer sava + employer unemployment expense).
const ACC_EMPLOYER_SIDE_EXPENSE: &str = "5500";
/// Credit — Palkkavelat (net pay owed to the employee).
const ACC_NET_PAY_LIABILITY: &str = "2790";
/// Credit — Ennakonpidätysvelka (withholding tax withheld).
const ACC_WITHHOLDING_LIABILITY: &str = "2770";
/// Credit — Sosiaaliturvamaksuvelka (employer sava payable).
const ACC_SAVA_LIABILITY: &str = "2780";
/// Credit — Muut lyhytaikaiset velat (TyEL + unemployment payable to the providers).
const ACC_PENSION_UNEMP_LIABILITY: &str = "2740";

/// The single-currency assumption of the slice (EUR minor unit is the cent).
fn eur() -> Currency {
    Currency::new("EUR").expect("EUR is a valid currency code")
}

/// The **rounding policy** for percentage deductions (must-spec #7): round to the
/// EUR cent, half **away from zero**, and pin the scale to 2 dp so `amount_str()`
/// is canonical (`"12.30"`, never `"12.3"`). Applied once per computed line.
fn round_eur(mut d: Decimal) -> Decimal {
    d = d.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
    d.rescale(2);
    d
}

/// `round_eur(base × pct / 100)` — one statutory line.
fn pct_of(base: Decimal, pct: Decimal) -> Decimal {
    round_eur(base * pct / Decimal::from(100))
}

// ---------------------------------------------------------------------------
// Statutory rate table (dated editions, loaded from data)
// ---------------------------------------------------------------------------

/// One dated edition of the statutory rates, valid over `[valid_from, valid_to]`
/// (`valid_to = None` = current). Percentages are canonical decimal strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateEdition {
    pub valid_from: Date,
    pub valid_to: Option<Date>,
    pub tyel_employee_pct: String,
    pub tyel_employer_pct: String,
    pub sava_employer_pct: String,
    pub unemployment_employee_pct: String,
    pub unemployment_employer_pct: String,
}

/// The bundled statutory rate table (all editions).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateTable {
    pub table: String,
    pub editions: Vec<RateEdition>,
}

/// The resolved, parsed rates for one pay-run (the edition selected by
/// `payment_date`), plus the content-hashed `rates_version` stamped into a payslip
/// so a run records which rate edition it computed against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRates {
    pub rates_version: String,
    pub valid_from: Date,
    pub valid_to: Option<Date>,
    pub tyel_employee_pct: Decimal,
    pub tyel_employer_pct: Decimal,
    pub sava_employer_pct: Decimal,
    pub unemployment_employee_pct: Decimal,
    pub unemployment_employer_pct: Decimal,
}

const RATES_JSON: &str = include_str!("../data/payroll-rates.json");
const TULOLAJIT_JSON: &str = include_str!("../data/tulolajit.json");

fn rate_table() -> &'static RateTable {
    static TABLE: OnceLock<RateTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str(RATES_JSON).expect("bundled payroll-rates.json parses")
    })
}

/// `"<table>@<64 hex>"` — a content hash over the rate table bytes (the chart's
/// `chart_version` convention), so a payslip records exactly which rate edition set
/// it was computed against.
fn rates_version() -> &'static str {
    static VER: OnceLock<String> = OnceLock::new();
    VER.get_or_init(|| {
        let mut h = Sha256::new();
        h.update(RATES_JSON.as_bytes());
        format!("{}@{}", rate_table().table, hex::encode(h.finalize()))
    })
}

fn parse_pct(s: &str, field: &str) -> Result<Decimal, DomainError> {
    let d: Decimal = s
        .parse()
        .map_err(|e| DomainError::PayrollRates(format!("{field} {s:?} is not a decimal: {e}")))?;
    if d < Decimal::ZERO {
        return Err(DomainError::PayrollRates(format!(
            "{field} must be non-negative, got {s}"
        )));
    }
    Ok(d)
}

/// Resolve the statutory rates in force on `payment_date` (must-spec #1). Fails with
/// `PAYROLL_RATES` if no edition covers the date — rates are never extrapolated.
pub fn resolve_rates(payment_date: Date) -> Result<ResolvedRates, DomainError> {
    let table = rate_table();
    let edition = table
        .editions
        .iter()
        .find(|e| payment_date >= e.valid_from && e.valid_to.is_none_or(|to| payment_date <= to))
        .ok_or_else(|| {
            DomainError::PayrollRates(format!(
                "no statutory rate edition covers payment_date {payment_date} \
                 (rates are dated data; add the {} edition)",
                payment_date
            ))
        })?;
    Ok(ResolvedRates {
        rates_version: rates_version().to_string(),
        valid_from: edition.valid_from,
        valid_to: edition.valid_to,
        tyel_employee_pct: parse_pct(&edition.tyel_employee_pct, "tyel_employee_pct")?,
        tyel_employer_pct: parse_pct(&edition.tyel_employer_pct, "tyel_employer_pct")?,
        sava_employer_pct: parse_pct(&edition.sava_employer_pct, "sava_employer_pct")?,
        unemployment_employee_pct: parse_pct(
            &edition.unemployment_employee_pct,
            "unemployment_employee_pct",
        )?,
        unemployment_employer_pct: parse_pct(
            &edition.unemployment_employer_pct,
            "unemployment_employer_pct",
        )?,
    })
}

// ---------------------------------------------------------------------------
// Tulolaji (income-type) catalog as data (must-spec #5)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TulolajiEntry {
    code: String,
    name: String,
    payroll_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TulolajiCatalog {
    catalog: String,
    catalog_date: String,
    source: String,
    codes: Vec<TulolajiEntry>,
}

fn tulolaji_catalog() -> &'static TulolajiCatalog {
    static CAT: OnceLock<TulolajiCatalog> = OnceLock::new();
    CAT.get_or_init(|| serde_json::from_str(TULOLAJIT_JSON).expect("bundled tulolajit.json parses"))
}

/// Resolve a tulolaji code to its name, or `TULOLAJI_UNKNOWN` for a code not in the
/// catalog (the code set is data, not free text — must-spec #5).
pub fn resolve_tulolaji(code: &str) -> Result<&'static str, DomainError> {
    tulolaji_catalog()
        .codes
        .iter()
        .find(|e| e.code == code)
        .map(|e| e.name.as_str())
        .ok_or_else(|| {
            DomainError::Tulolaji(format!(
                "unknown tulolaji code {code:?} (see tulolajit.json)"
            ))
        })
}

// ---------------------------------------------------------------------------
// Tax card (non-PII computation params) — the F7 EmployeeState stores one of these
// ---------------------------------------------------------------------------

/// The tax-card parameters a payslip is computed from (must-spec #2, #6). Non-PII on
/// its own (a percentage), so it lives in the Domain as a computation input; the
/// ledger's `EmployeeState` embeds it alongside the identity that IS PII. Simplified
/// for the slice to a single effective `withholding_pct` (a real tax card carries
/// perusprosentti + lisäprosentti + tuloraja — a later refinement).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaxCard {
    /// Effective ennakonpidätys percentage (`0..=100`), a canonical decimal string.
    pub withholding_pct: String,
}

impl TaxCard {
    fn withholding(&self) -> Result<Decimal, DomainError> {
        let d = parse_pct(&self.withholding_pct, "withholding_pct")?;
        if d > Decimal::from(100) {
            return Err(DomainError::Payroll(format!(
                "withholding_pct must be 0..=100, got {}",
                self.withholding_pct
            )));
        }
        Ok(d)
    }
}

// ---------------------------------------------------------------------------
// Payroll intent (AI-produced) → payslip (Rust-computed)
// ---------------------------------------------------------------------------

/// One employee's line in a `PayrollIntent`: the agent supplies only the **gross**
/// and the income-type; every statutory amount is computed here (must-spec #6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmployeeIntent {
    pub employee_id: String,
    pub gross: Money,
    pub tulolaji: String,
}

/// The AI-produced input to a pay run (must-spec #6). Carries no computed statutory
/// figures — only the period, the mandatory `payment_date` (must-spec #4: it drives
/// the 5-day Tulorekisteri deadline and the rate edition), and the per-employee gross.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayrollIntent {
    pub payment_date: Date,
    pub employees: Vec<EmployeeIntent>,
}

/// One employee's deterministically computed payslip. All amounts are 2-dp EUR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payslip {
    pub employee_id: String,
    pub tulolaji: String,
    pub tulolaji_name: String,
    pub rates_version: String,
    pub gross: Money,
    /// Employee-side deductions (withheld from gross, reduce net).
    pub withholding: Money,
    pub employee_tyel: Money,
    pub employee_unemployment: Money,
    pub net: Money,
    /// Employer-side side-costs (added expense + liability, not deducted from pay).
    pub employer_tyel: Money,
    pub employer_sava: Money,
    pub employer_unemployment: Money,
}

/// Compute one employee's payslip deterministically from the gross, the tax card and
/// the resolved statutory rates. Total over valid input; `PAYROLL` on a non-positive
/// or non-EUR gross, `TULOLAJI` on an unknown income-type.
pub fn compute_payslip(
    intent: &EmployeeIntent,
    tax_card: &TaxCard,
    rates: &ResolvedRates,
) -> Result<Payslip, DomainError> {
    if intent.gross.currency() != &eur() {
        return Err(DomainError::Payroll(format!(
            "payroll is EUR-only in this slice, got {}",
            intent.gross.currency()
        )));
    }
    let gross = intent.gross.amount();
    if gross <= Decimal::ZERO {
        return Err(DomainError::Payroll(format!(
            "gross must be > 0, got {}",
            intent.gross.amount_str()
        )));
    }
    // Reject sub-cent gross: `Money` preserves scale, so a 3-dp gross would let `net`
    // (computed from raw gross) and the stored 2-dp `gross` diverge and unbalance the
    // voucher. Payroll input is whole cents.
    if gross.scale() > 2 {
        return Err(DomainError::Payroll(format!(
            "gross must have at most 2 decimal places (EUR cents), got {}",
            intent.gross.amount_str()
        )));
    }
    let tulolaji_name = resolve_tulolaji(&intent.tulolaji)?.to_string();
    if !tulolaji_catalog()
        .codes
        .iter()
        .any(|e| e.code == intent.tulolaji && e.payroll_supported)
    {
        return Err(DomainError::Payroll(
            "travel reimbursement codes belong to file-wage, not the salary calculation".into(),
        ));
    }
    let withholding_pct = tax_card.withholding()?;

    let withholding = pct_of(gross, withholding_pct);
    let employee_tyel = pct_of(gross, rates.tyel_employee_pct);
    let employee_unemployment = pct_of(gross, rates.unemployment_employee_pct);
    let mut net = gross - withholding - employee_tyel - employee_unemployment;
    net.rescale(2);
    if net < Decimal::ZERO {
        return Err(DomainError::Payroll(format!(
            "employee {} deductions exceed gross (net would be {net})",
            intent.employee_id
        )));
    }

    let employer_tyel = pct_of(gross, rates.tyel_employer_pct);
    let employer_sava = pct_of(gross, rates.sava_employer_pct);
    let employer_unemployment = pct_of(gross, rates.unemployment_employer_pct);

    let m = |d: Decimal| Money::new(d, eur());
    Ok(Payslip {
        employee_id: intent.employee_id.clone(),
        tulolaji: intent.tulolaji.clone(),
        tulolaji_name,
        rates_version: rates.rates_version.clone(),
        gross: m(round_eur(gross)),
        withholding: m(withholding),
        employee_tyel: m(employee_tyel),
        employee_unemployment: m(employee_unemployment),
        net: m(net),
        employer_tyel: m(employer_tyel),
        employer_sava: m(employer_sava),
        employer_unemployment: m(employer_unemployment),
    })
}

// ---------------------------------------------------------------------------
// Payroll voucher footprint (must-spec #3): full double-entry, Σdebit = Σcredit
// ---------------------------------------------------------------------------

/// Assemble the balanced payroll voucher for one payslip (must-spec #3). The
/// `VoucherDraft` constructor enforces `Σdebit = Σcredit`, ≥2 postings and single
/// currency; a zero-valued leg is omitted (postings must be > 0). See the module
/// docs for the account mapping.
pub fn build_payroll_voucher(
    payslip: &Payslip,
    document_date: Date,
    posting_date: Date,
    description: String,
) -> Result<VoucherDraft, DomainError> {
    let mut postings = Vec::new();
    let mut push = |account: &str, amount: &Money, side: Side| -> Result<(), DomainError> {
        if amount.amount() <= Decimal::ZERO {
            return Ok(()); // omit zero legs — postings must be strictly positive
        }
        postings.push(Posting {
            account: AccountId::new(account)?,
            amount: amount.clone(),
            side,
            vat_code: None,
        });
        Ok(())
    };

    // Debits: gross expense + employer side-costs.
    push(ACC_GROSS_EXPENSE, &payslip.gross, Side::Debit)?;
    push(
        ACC_EMPLOYER_TYEL_EXPENSE,
        &payslip.employer_tyel,
        Side::Debit,
    )?;
    let employer_side = add(&payslip.employer_sava, &payslip.employer_unemployment);
    push(ACC_EMPLOYER_SIDE_EXPENSE, &employer_side, Side::Debit)?;

    // Credits: net pay + withheld + employer/employee statutory liabilities.
    push(ACC_NET_PAY_LIABILITY, &payslip.net, Side::Credit)?;
    push(
        ACC_WITHHOLDING_LIABILITY,
        &payslip.withholding,
        Side::Credit,
    )?;
    push(ACC_SAVA_LIABILITY, &payslip.employer_sava, Side::Credit)?;
    let pension_unemp = add(
        &add(&payslip.employee_tyel, &payslip.employer_tyel),
        &add(
            &payslip.employee_unemployment,
            &payslip.employer_unemployment,
        ),
    );
    push(ACC_PENSION_UNEMP_LIABILITY, &pension_unemp, Side::Credit)?;

    Ok(VoucherDraft::new(
        document_date,
        posting_date,
        description,
        postings,
        None,
    )?)
}

/// Sum two EUR amounts at 2-dp scale.
fn add(a: &Money, b: &Money) -> Money {
    let mut d = a.amount() + b.amount();
    d.rescale(2);
    Money::new(d, eur())
}

// ---------------------------------------------------------------------------
// The two distinct Tulorekisteri reports (must-spec #4) — never conflated
// ---------------------------------------------------------------------------

/// The two Tulorekisteri report kinds. They are **distinct** obligations
/// (must-spec #4): a per-payment palkkatietoilmoitus due within 5 days of the
/// payment, and a monthly työnantajan erillisilmoitus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TulorekisteriReport {
    /// Per-payment earnings report (palkkatietoilmoitus). Due `deadline` = 5 days
    /// after `payment_date`.
    Palkkatietoilmoitus { payment_date: Date, deadline: Date },
    /// Monthly employer summary (työnantajan erillisilmoitus), for `month` (`YYYY-MM`).
    TyonantajanErillisilmoitus { month: String },
}

/// The palkkatietoilmoitus filing deadline: 5 calendar days after the payment
/// (must-spec #4). Deterministic date math.
pub fn palkkatieto_deadline(payment_date: Date) -> Date {
    let d = payment_date.inner() + chrono::Duration::days(5);
    // `payment_date` is already in the valid calendar-year range and +5 days cannot
    // leave it, so this constructor cannot fail.
    Date::from_ymd(
        chrono::Datelike::year(&d),
        chrono::Datelike::month(&d),
        chrono::Datelike::day(&d),
    )
    .expect("payment_date + 5 days is a valid date")
}

/// Build the per-payment report descriptor for a payment made on `payment_date`.
pub fn palkkatietoilmoitus(payment_date: Date) -> TulorekisteriReport {
    TulorekisteriReport::Palkkatietoilmoitus {
        payment_date,
        deadline: palkkatieto_deadline(payment_date),
    }
}

/// Build the monthly report descriptor for the month containing `payment_date`
/// (`YYYY-MM`).
pub fn erillisilmoitus(payment_date: Date) -> TulorekisteriReport {
    let d = payment_date.inner();
    TulorekisteriReport::TyonantajanErillisilmoitus {
        month: format!(
            "{:04}-{:02}",
            chrono::Datelike::year(&d),
            chrono::Datelike::month(&d)
        ),
    }
}

// ---------------------------------------------------------------------------
// HETU (Finnish personal identity code) validation — the Run's HETU-preflight
// ---------------------------------------------------------------------------

/// The mod-31 checksum alphabet for a HETU's control character.
const HETU_CHECK: &[u8; 31] = b"0123456789ABCDEFHJKLMNPRSTUVWXY";

/// Validate a Finnish personal identity code (HETU): `ddmmyy` + century mark +
/// 3-digit individual number + mod-31 checksum character. Used for the RF4
/// HETU-preflight (a bad HETU parks that employee `NeedsAttention` for
/// `ResolveErrors`, rather than reaching the authorities). Rejects with
/// `INVALID_HETU`.
pub fn validate_hetu(hetu: &str) -> Result<(), DomainError> {
    let bad = |why: &str| DomainError::Hetu(format!("{why}: {hetu:?}"));
    let b = hetu.as_bytes();
    if b.len() != 11 {
        return Err(bad("HETU must be 11 characters (ddmmyyCzzzQ)"));
    }
    if !b[0..6].iter().all(u8::is_ascii_digit) {
        return Err(bad("the ddmmyy part must be 6 digits"));
    }
    if !matches!(
        b[6],
        b'+' | b'-' | b'A' | b'B' | b'C' | b'D' | b'E' | b'F' | b'X' | b'Y' | b'W' | b'V' | b'U'
    ) {
        return Err(bad("invalid century marker"));
    }
    if !b[7..10].iter().all(u8::is_ascii_digit) {
        return Err(bad("the individual number must be 3 digits"));
    }
    let day: u32 = hetu[0..2].parse().map_err(|_| bad("bad day"))?;
    let month: u32 = hetu[2..4].parse().map_err(|_| bad("bad month"))?;
    let yy: i32 = hetu[4..6].parse().map_err(|_| bad("bad year"))?;
    // Build the real birth date from the century marker and reject impossible dates
    // (31 Feb, 29 Feb in a non-leap year, …) — a range check alone is not enough.
    let century = match b[6] {
        b'+' => 1800,
        b'-' | b'Y' | b'X' | b'W' | b'V' | b'U' => 1900,
        _ => 2000, // A..F (validated above)
    };
    if chrono::NaiveDate::from_ymd_opt(century + yy, month, day).is_none() {
        return Err(bad("impossible birth date"));
    }
    // The checksum is over the 9-digit number ddmmyyzzz.
    let ddmmyyzzz: u64 = format!("{}{}", &hetu[0..6], &hetu[7..10])
        .parse()
        .map_err(|_| bad("checksum preimage not numeric"))?;
    let expected = HETU_CHECK[(ddmmyyzzz % 31) as usize];
    if b[10] != expected {
        return Err(bad(&format!(
            "checksum character is {:?}, expected {:?}",
            b[10] as char, expected as char
        )));
    }
    Ok(())
}

/// Convenience accessor for the chart accounts the payroll footprint touches, so a
/// caller (e.g. `aggountant init` sanity checks) can assert they exist. Not used by the
/// compute path.
pub fn payroll_accounts() -> [&'static str; 7] {
    [
        ACC_GROSS_EXPENSE,
        ACC_EMPLOYER_TYEL_EXPENSE,
        ACC_EMPLOYER_SIDE_EXPENSE,
        ACC_NET_PAY_LIABILITY,
        ACC_WITHHOLDING_LIABILITY,
        ACC_SAVA_LIABILITY,
        ACC_PENSION_UNEMP_LIABILITY,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn eur_money(s: &str) -> Money {
        Money::parse(s, eur()).unwrap()
    }
    fn d(s: &str) -> Date {
        Date::parse(s).unwrap()
    }

    fn rates_2026() -> ResolvedRates {
        resolve_rates(d("2026-02-15")).unwrap()
    }

    #[test]
    fn resolve_rates_picks_the_edition_covering_the_date() {
        let r25 = resolve_rates(d("2025-06-01")).unwrap();
        assert_eq!(
            r25.sava_employer_pct,
            Decimal::from_str_exact("1.87").unwrap()
        );
        let r26 = resolve_rates(d("2026-06-01")).unwrap();
        assert_eq!(
            r26.sava_employer_pct,
            Decimal::from_str_exact("1.53").unwrap()
        );
        // Same table content → same version regardless of the date.
        assert_eq!(r25.rates_version, r26.rates_version);
        assert!(r26.rates_version.starts_with("fi-payroll-rates@"));
    }

    #[test]
    fn resolve_rates_refuses_an_uncovered_date() {
        let err = resolve_rates(d("2019-01-01")).unwrap_err();
        assert_eq!(err.error_code(), "PAYROLL_RATES");
    }

    #[test]
    fn compute_is_deterministic_and_rounds_per_line() {
        let intent = EmployeeIntent {
            employee_id: "emp-1".into(),
            gross: eur_money("3000.00"),
            tulolaji: "201".into(),
        };
        let card = TaxCard {
            withholding_pct: "20.00".into(),
        };
        let slip = compute_payslip(&intent, &card, &rates_2026()).unwrap();
        // 20% of 3000 = 600.00; TyEL 7.15% = 214.50; unemployment 0.59% = 17.70.
        assert_eq!(slip.withholding.amount_str(), "600.00");
        assert_eq!(slip.employee_tyel.amount_str(), "214.50");
        assert_eq!(slip.employee_unemployment.amount_str(), "17.70");
        // net = 3000 - 600 - 214.50 - 17.70 = 2167.80
        assert_eq!(slip.net.amount_str(), "2167.80");
        assert_eq!(slip.employer_tyel.amount_str(), "521.40");
        assert_eq!(slip.employer_sava.amount_str(), "45.90");
        assert_eq!(slip.tulolaji_name, "Aikapalkka");
        // Deterministic: recomputing yields an equal payslip.
        assert_eq!(
            slip,
            compute_payslip(&intent, &card, &rates_2026()).unwrap()
        );
    }

    #[test]
    fn rounding_is_half_away_from_zero() {
        // 0.125 rounds to 0.13 (away from zero), not 0.12.
        assert_eq!(
            round_eur(Decimal::from_str_exact("0.125").unwrap()).to_string(),
            "0.13"
        );
        assert_eq!(
            pct_of(Decimal::from(50), Decimal::from_str_exact("0.25").unwrap()).to_string(),
            "0.13"
        );
    }

    #[test]
    fn voucher_footprint_balances() {
        let intent = EmployeeIntent {
            employee_id: "emp-1".into(),
            gross: eur_money("3000.00"),
            tulolaji: "201".into(),
        };
        let card = TaxCard {
            withholding_pct: "20.00".into(),
        };
        let slip = compute_payslip(&intent, &card, &rates_2026()).unwrap();
        let v = build_payroll_voucher(
            &slip,
            d("2026-02-28"),
            d("2026-02-28"),
            "Palkka emp-1".into(),
        )
        .unwrap();
        // VoucherDraft::new already enforced Σ=0; assert we produced a real footprint.
        assert!(v.postings().len() >= 4);
        let sum: Decimal = v.postings().iter().map(Posting::signed_amount).sum();
        assert!(sum.is_zero());
    }

    #[test]
    fn travel_catalog_entries_never_enter_salary_deductions() {
        for code in ["311", "331"] {
            let intent = EmployeeIntent {
                employee_id: "test".into(),
                gross: eur_money("54.00"),
                tulolaji: code.into(),
            };
            assert!(compute_payslip(
                &intent,
                &TaxCard {
                    withholding_pct: "20".into()
                },
                &rates_2026()
            )
            .is_err());
        }
    }

    #[test]
    fn unknown_tulolaji_is_rejected() {
        let intent = EmployeeIntent {
            employee_id: "emp-1".into(),
            gross: eur_money("3000.00"),
            tulolaji: "999".into(),
        };
        let card = TaxCard {
            withholding_pct: "20.00".into(),
        };
        let err = compute_payslip(&intent, &card, &rates_2026()).unwrap_err();
        assert_eq!(err.error_code(), "TULOLAJI");
    }

    #[test]
    fn palkkatieto_deadline_is_five_days() {
        assert_eq!(palkkatieto_deadline(d("2026-02-28")), d("2026-03-05"));
        match palkkatietoilmoitus(d("2026-02-28")) {
            TulorekisteriReport::Palkkatietoilmoitus { deadline, .. } => {
                assert_eq!(deadline, d("2026-03-05"))
            }
            _ => panic!("wrong kind"),
        }
    }

    #[test]
    fn erillisilmoitus_is_monthly() {
        match erillisilmoitus(d("2026-02-28")) {
            TulorekisteriReport::TyonantajanErillisilmoitus { month } => {
                assert_eq!(month, "2026-02")
            }
            _ => panic!("wrong kind"),
        }
    }

    #[test]
    fn hetu_validation_accepts_valid_and_rejects_bad_checksum() {
        // A known-valid test HETU.
        assert!(validate_hetu("131052-308T").is_ok());
        assert!(validate_hetu("010101A123N").is_ok());
        // Wrong checksum char.
        assert_eq!(
            validate_hetu("131052-308U").unwrap_err().error_code(),
            "INVALID_HETU"
        );
        // Wrong length / shape.
        assert!(validate_hetu("13105-308T").is_err());
        assert!(validate_hetu("131352-308T").is_err()); // month 13
    }

    #[test]
    fn hetu_rejects_impossible_calendar_dates() {
        // 31 Feb / 29 Feb in a non-leap year must be rejected (real-date construction).
        assert!(validate_hetu("310252-999X").is_err());
        assert!(validate_hetu("290299-999X").is_err()); // 1999 is not a leap year
    }

    #[test]
    fn gross_must_be_whole_cents() {
        let intent = EmployeeIntent {
            employee_id: "emp-1".into(),
            gross: Money::new(Decimal::from_str_exact("3000.005").unwrap(), eur()),
            tulolaji: "201".into(),
        };
        let card = TaxCard {
            withholding_pct: "20.00".into(),
        };
        let err = compute_payslip(&intent, &card, &rates_2026()).unwrap_err();
        assert_eq!(err.error_code(), "PAYROLL");
    }

    #[test]
    fn rate_editions_are_ordered_and_non_overlapping() {
        // A future edit that overlaps two editions (or lists them out of order) would
        // make first-match resolution silently ambiguous — catch it here.
        let eds = &rate_table().editions;
        for w in eds.windows(2) {
            let prev_to = w[0]
                .valid_to
                .expect("only the last edition may be open-ended");
            assert!(
                prev_to < w[1].valid_from,
                "editions overlap or are unordered: {:?} then {:?}",
                w[0].valid_to,
                w[1].valid_from
            );
        }
    }
}
