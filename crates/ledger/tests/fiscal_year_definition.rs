//! Fiscal years that are not calendar years (issue `non-calendar-fiscal-year`):
//! defining them, placing sealed vouchers in them, closing them month by month,
//! year-to-date reports and opening balances, and refusing definitions that would
//! re-attribute recorded bookkeeping. Esimerkki Oy and all figures are synthetic.

use aggountant_domain::PolicySet;
use aggountant_ledger::{
    FiscalYearState, Ledger, LedgerError, MonthlyReportOptions, PeriodState, SealRequest,
};
use aggountant_model::{AccountId, Currency, Date, Money, PeriodId, Posting, Side, VoucherDraft};
use rust_decimal::Decimal;

fn init_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    home
}

fn d(s: &str) -> Date {
    Date::parse(s).unwrap()
}

fn draft(date: &str, debit: &str, credit: &str, amount: &str) -> VoucherDraft {
    let eur = Currency::new("EUR").unwrap();
    let posting = |account: &str, side| Posting {
        account: AccountId::new(account).unwrap(),
        amount: Money::parse(amount, eur.clone()).unwrap(),
        side,
        vat_code: None,
    };
    VoucherDraft::new(
        d(date),
        d(date),
        "Esimerkki Oy test entry".into(),
        vec![posting(debit, Side::Debit), posting(credit, Side::Credit)],
        None,
    )
    .unwrap()
}

fn try_seal(
    ledger: &Ledger,
    date: &str,
    debit: &str,
    credit: &str,
    amount: &str,
    key: &str,
) -> Result<u64, LedgerError> {
    ledger
        .seal(
            &draft(date, debit, credit, amount),
            &SealRequest {
                natural_key: key.into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .map(|sealed| sealed.number)
}

fn seal(ledger: &Ledger, date: &str, debit: &str, credit: &str, amount: &str, key: &str) -> u64 {
    try_seal(ledger, date, debit, credit, amount, key).unwrap()
}

fn close(ledger: &Ledger, period: &PeriodId) {
    let run = format!("close-{period}");
    let authority = ledger.begin_close(period, &run, "test").unwrap();
    ledger.seal_period(period, &authority, "test").unwrap();
}

fn seal_year(ledger: &Ledger, year: i32) {
    for period in ledger.year_close_periods(year).unwrap() {
        if ledger.period_view(&period).unwrap().state != PeriodState::Sealed {
            close(ledger, &period);
        }
    }
    let view = ledger
        .seal_year(year, &format!("{year:064x}"), &format!("cy-{year}"), "test")
        .unwrap();
    assert_eq!(view.state, FiscalYearState::Sealed);
}

fn july_company() -> (tempfile::TempDir, Ledger) {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let defined = ledger
        .define_fiscal_year(d("2026-07-01"), d("2027-06-30"), "test")
        .unwrap();
    assert_eq!(defined.year, 2027);
    assert_eq!(defined.months, 12);
    assert_eq!(defined.next_start, d("2027-07-01"));
    assert!(!defined.replayed);
    (home, ledger)
}

#[test]
fn a_july_to_june_company_books_closes_and_reports_its_own_year() {
    let (_home, ledger) = july_company();

    // Share capital on the first day, a sale in the autumn, costs over the turn of
    // the calendar year, and one entry in the next fiscal year.
    seal(&ledger, "2026-07-01", "1910", "2000", "2500.00", "capital");
    seal(&ledger, "2026-09-15", "1910", "3000", "1000.00", "sale");
    seal(&ledger, "2027-01-10", "7690", "1910", "100.00", "cloud");
    seal(&ledger, "2027-06-30", "7690", "1910", "50.00", "cloud-june");
    seal(&ledger, "2027-07-01", "1910", "3000", "999.00", "next-year");

    let view = ledger.fiscal_year_view(2027).unwrap();
    assert_eq!(
        (view.start, view.end, view.months),
        (d("2026-07-01"), d("2027-06-30"), 12)
    );
    assert_eq!(ledger.fiscal_year_of(d("2026-12-31")).unwrap().year(), 2027);
    assert_eq!(ledger.fiscal_year(2028).unwrap().start(), d("2027-07-01"));

    let data = ledger.fiscal_year_data(2027).unwrap();
    assert_eq!(data.vouchers.len(), 4);
    assert!(data.opening_balance.is_none());
    assert_eq!(ledger.year_vouchers(2027).unwrap().len(), 4);

    // The year closes by its own twelve months, July to June.
    let periods: Vec<String> = ledger
        .year_close_periods(2027)
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(periods.first().unwrap(), "2026-07");
    assert_eq!(periods.last().unwrap(), "2027-06");
    assert_eq!(periods.len(), 12);

    // Close July–January month by month; the January report counts the year to date
    // from July.
    for period in &ledger.year_close_periods(2027).unwrap()[..7] {
        close(&ledger, period);
    }
    let january = PeriodId::parse("2027-01").unwrap();
    let report = ledger
        .monthly_statements_report(&january, "Esimerkki Oy", MonthlyReportOptions::default())
        .unwrap();
    assert_eq!(report.status.presentation, "closed");
    assert_eq!(report.sections.month_income.result, "-100.00");
    assert_eq!(report.sections.year_to_date_income.result, "900.00");
    assert_eq!(
        report.sections.year_to_date_income.title,
        "Tuloslaskelma, tilikauden alusta"
    );

    // Statements state the year's dates.
    let hash = ledger
        .close_package_hash_for_year(2027, &PolicySet::default().id, &"0".repeat(64))
        .unwrap();
    assert_eq!(hash.len(), 64);
}

#[test]
fn a_date_before_the_first_fiscal_year_is_refused_and_audited() {
    let (_home, ledger) = july_company();
    let before = ledger.rejection_count().unwrap();
    let err = try_seal(&ledger, "2026-06-30", "1910", "2000", "1.00", "early").unwrap_err();
    assert_eq!(err.error_code(), "FISCAL_YEAR_UNDEFINED");
    assert_eq!(ledger.rejection_count().unwrap(), before + 1);
    assert_eq!(
        ledger.fiscal_year(2026).unwrap_err().error_code(),
        "FISCAL_YEAR_UNDEFINED"
    );
}

#[test]
fn the_next_year_opens_from_the_sealed_july_year() {
    let (_home, ledger) = july_company();
    seal(&ledger, "2026-07-01", "1910", "2000", "2500.00", "capital");
    seal(&ledger, "2026-09-15", "1910", "3000", "1000.00", "sale");
    seal(&ledger, "2027-07-05", "7690", "1910", "40.00", "next");

    // Before 2027 is sealed, the next year cannot open from it.
    let err = ledger.fiscal_year_data(2028).unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED");
    assert!(err.to_string().contains("2027"), "{err}");

    seal_year(&ledger, 2027);
    let data = ledger.fiscal_year_data(2028).unwrap();
    let opening = data.opening_balance.expect("an opening balance from 2027");
    assert_eq!(opening.fiscal_year(), 2028);
    assert_eq!(opening.source().prior_year(), 2027);
    let bank: Decimal = opening
        .postings()
        .iter()
        .filter(|p| p.account.as_str() == "1910")
        .map(Posting::signed_amount)
        .sum();
    assert_eq!(bank, Decimal::new(350000, 2));
    assert_eq!(data.vouchers.len(), 1);

    // A sealed fiscal year blocks reopening its months even across the calendar year.
    let september = PeriodId::parse("2026-09").unwrap();
    let authority = ledger
        .acquire_authority(&september, "close-2026-09")
        .unwrap();
    let err = ledger
        .reopen_period(&september, &authority, PeriodState::Open, "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_TRANSITION_CONFLICT");
    assert!(
        err.to_string().contains("fiscal year 2027 is Sealed"),
        "{err}"
    );
}

#[test]
fn an_eighteen_month_first_year_is_accepted_and_a_longer_one_refused() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let err = ledger
        .define_fiscal_year(d("2026-07-01"), d("2028-01-31"), "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "INVALID_FISCAL_YEAR");
    assert!(err.to_string().contains("kirjanpitolaki 1:4"), "{err}");
    assert!(ledger.fiscal_calendar().unwrap().defined().is_empty());

    let defined = ledger
        .define_fiscal_year(d("2026-07-01"), d("2027-12-31"), "test")
        .unwrap();
    assert_eq!((defined.year, defined.months), (2027, 18));
    assert_eq!(defined.next_start, d("2028-01-01"));

    seal(&ledger, "2026-07-01", "1910", "2000", "2500.00", "capital");
    seal(&ledger, "2027-12-31", "7690", "1910", "10.00", "late");
    seal(&ledger, "2028-01-02", "7690", "1910", "20.00", "next");
    assert_eq!(ledger.year_close_periods(2027).unwrap().len(), 18);
    assert_eq!(ledger.fiscal_year_data(2027).unwrap().vouchers.len(), 2);

    // No fiscal year ends in 2026; the year after the long one opens from it.
    assert_eq!(
        ledger.fiscal_year(2026).unwrap_err().error_code(),
        "FISCAL_YEAR_UNDEFINED"
    );
    seal_year(&ledger, 2027);
    let opening = ledger
        .fiscal_year_data(2028)
        .unwrap()
        .opening_balance
        .unwrap();
    assert_eq!(opening.source().prior_year(), 2027);
}

#[test]
fn adjustments_attribute_to_a_july_year() {
    let (_home, ledger) = july_company();
    seal(&ledger, "2026-08-01", "1910", "3000", "500.00", "sale");
    let adjustment = seal(&ledger, "2027-07-15", "7690", "1910", "30.00", "accrual");

    // Posted in fiscal year 2028; cannot be attributed to a later year.
    let err = ledger
        .attribute_adjustment(adjustment, 2029, "cy-2027", "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_TRANSITION_CONFLICT");
    ledger
        .attribute_adjustment(adjustment, 2027, "cy-2027", "test")
        .unwrap();
    let data = ledger.fiscal_year_data(2027).unwrap();
    let redated = data
        .vouchers
        .iter()
        .find(|v| v.number() == adjustment)
        .unwrap();
    assert_eq!(redated.posting_date(), d("2027-06-30"));
}

#[test]
fn a_calendar_company_changes_to_july_with_a_short_year() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal(&ledger, "2026-03-01", "1910", "2000", "2500.00", "capital");
    seal(
        &ledger,
        "2027-02-01",
        "1910",
        "3000",
        "100.00",
        "early-2027",
    );

    // The 2027 voucher stays in fiscal year 2027, which shrinks to six months.
    ledger
        .define_fiscal_year(d("2027-01-01"), d("2027-06-30"), "test")
        .unwrap();
    let short = ledger.fiscal_year(2027).unwrap();
    assert_eq!(short.months(), 6);
    assert_eq!(ledger.fiscal_year(2026).unwrap().months(), 12);
    assert_eq!(ledger.fiscal_year(2028).unwrap().start(), d("2027-07-01"));
    assert_eq!(ledger.year_close_periods(2027).unwrap().len(), 6);
}

#[test]
fn definitions_that_would_move_recorded_bookkeeping_are_refused() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal(&ledger, "2026-08-01", "1910", "2000", "2500.00", "capital");

    // A calendar home with an August 2026 voucher: making 1.7.2026 the start would
    // move it from fiscal year 2026 to 2027.
    let before = ledger.rejection_count().unwrap();
    let err = ledger
        .define_fiscal_year(d("2026-07-01"), d("2027-06-30"), "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "FISCAL_YEAR_CONFLICT");
    assert!(err.to_string().contains("sealed voucher 1"), "{err}");
    assert_eq!(ledger.rejection_count().unwrap(), before + 1);

    // Starting after the voucher's year is fine, and replays idempotently.
    let first = ledger
        .define_fiscal_year(d("2027-01-01"), d("2027-06-30"), "test")
        .unwrap();
    let again = ledger
        .define_fiscal_year(d("2027-01-01"), d("2027-06-30"), "test")
        .unwrap();
    assert!(!first.replayed && again.replayed);
    assert_eq!(ledger.fiscal_calendar().unwrap().defined().len(), 1);

    // Append-only: nothing can be inserted before or over an existing definition.
    let err = ledger
        .define_fiscal_year(d("2027-01-01"), d("2027-12-31"), "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "INVALID_FISCAL_YEAR");

    // Two fiscal years ending in one calendar year cannot be told apart.
    let err = ledger
        .define_fiscal_year(d("2027-07-01"), d("2027-12-31"), "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "INVALID_FISCAL_YEAR");
    assert!(err.to_string().contains("both end in 2027"), "{err}");
}

#[test]
fn a_sealed_year_keeps_its_dates() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_year(&ledger, 2026);
    // An empty, sealed calendar year 2026: a first definition starting mid-2026 would
    // remove it.
    let err = ledger
        .define_fiscal_year(d("2026-07-01"), d("2027-06-30"), "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "FISCAL_YEAR_CONFLICT");
    assert!(err.to_string().contains("fiscal year 2026"), "{err}");
}

#[test]
fn quarters_cannot_be_closed_inside_a_non_calendar_year() {
    // Review (verifier) finding: closing and VAT-filing a quarter inside a
    // non-calendar year would leave that year impossible to seal.
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    ledger
        .define_fiscal_year(d("2026-07-01"), d("2027-06-30"), "test")
        .unwrap();
    let before = ledger.rejection_count().unwrap();
    for quarter in ["2026-Q3", "2027-Q1", "2027-Q4"] {
        let err = ledger
            .begin_close(&PeriodId::parse(quarter).unwrap(), "close-q", "test")
            .unwrap_err();
        assert_eq!(err.error_code(), "PERIOD_TRANSITION_CONFLICT", "{quarter}");
        assert!(err.to_string().contains("closed by month"), "{err}");
    }
    assert_eq!(ledger.rejection_count().unwrap(), before + 3);
    // Months close normally, and a calendar company keeps its quarters.
    close(&ledger, &PeriodId::parse("2026-07").unwrap());
    let calendar_home = init_home();
    let calendar = Ledger::open(calendar_home.path()).unwrap();
    close(&calendar, &PeriodId::parse("2026-Q3").unwrap());
}

#[test]
fn a_definition_may_not_remove_or_redate_an_attributed_year() {
    // Review finding 1: an attribution names a fiscal year by label; a definition
    // that removes that label (an 18-month year swallowing it) would turn the
    // voucher into prior-year activity of the next year.
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_year(&ledger, 2026);
    let adjustment = seal(&ledger, "2028-01-01", "7690", "1910", "100.00", "adj");
    ledger
        .attribute_adjustment(adjustment, 2027, "cy-2027", "test")
        .unwrap();
    let err = ledger
        .define_fiscal_year(d("2027-01-01"), d("2028-06-30"), "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "FISCAL_YEAR_CONFLICT");
    assert!(err.to_string().contains("attributed"), "{err}");
    // Shortening 2027 to six months would also change what the attribution means.
    let err = ledger
        .define_fiscal_year(d("2027-01-01"), d("2027-06-30"), "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "FISCAL_YEAR_CONFLICT");
}

#[test]
fn an_attribution_must_name_an_existing_fiscal_year() {
    // Review finding 2.
    let (_home, ledger) = july_company();
    let adjustment = seal(&ledger, "2027-07-15", "7690", "1910", "30.00", "accrual");
    let before = ledger.rejection_count().unwrap();
    let err = ledger
        .attribute_adjustment(adjustment, 2026, "cy-2026", "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "FISCAL_YEAR_UNDEFINED");
    assert_eq!(ledger.rejection_count().unwrap(), before + 1);
    assert!(ledger.adjustment_attribution(adjustment).unwrap().is_none());
}

#[test]
fn a_definition_may_not_strand_active_quarter_coverage() {
    // Review finding 3: a non-calendar fiscal year closes by month, and a calendar
    // year cannot mix active months and quarters, so a definition that puts an
    // active (here: VAT-filed) quarter's calendar year into a non-calendar fiscal
    // year would leave the year impossible to close.
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal(&ledger, "2027-02-01", "1910", "3000", "100.00", "sale");
    close(&ledger, &PeriodId::parse("2027-Q1").unwrap());
    let err = ledger
        .define_fiscal_year(d("2027-01-01"), d("2027-06-30"), "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "FISCAL_YEAR_CONFLICT");
    assert!(err.to_string().contains("2027-Q1"), "{err}");

    // A quarter in a calendar year the definition leaves alone is fine.
    ledger
        .define_fiscal_year(d("2028-01-01"), d("2028-06-30"), "test")
        .unwrap();
}

#[test]
fn a_definition_starting_mid_year_may_not_orphan_an_active_quarter() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    close(&ledger, &PeriodId::parse("2026-Q1").unwrap());
    let err = ledger
        .define_fiscal_year(d("2026-07-01"), d("2027-06-30"), "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "FISCAL_YEAR_CONFLICT");
}
