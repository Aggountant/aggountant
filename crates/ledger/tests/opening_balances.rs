//! Cross-crate opening-balance tests: prior closing positions are first-class model
//! data, sealed vouchers remain sealed-voucher-only, statement sources are tagged, and
//! deriving the opening position cannot mutate voucher hash preimages.

use aggountant_domain::{
    compute_statements, kpa_policy, kpa_policy_version, tase_erittelyt, PolicySet,
    StatementProvenance, StatementSourceRef,
};
use aggountant_ledger::{FiscalYearState, Ledger, MonthlyReportOptions, SealRequest};
use aggountant_model::{
    AccountId, Currency, Date, Money, OpeningBalance, OpeningBalanceSource, PeriodId, Posting,
    Side, VoucherDraft,
};
use rust_decimal::Decimal;

fn init_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    home
}

fn eur() -> Currency {
    Currency::new("EUR").unwrap()
}

fn seal(ledger: &Ledger, date: &str, debit: &str, credit: &str, amount: &str, nk: &str) {
    let d = Date::parse(date).unwrap();
    let postings = vec![
        Posting {
            account: AccountId::new(debit).unwrap(),
            amount: Money::parse(amount, eur()).unwrap(),
            side: Side::Debit,
            vat_code: None,
        },
        Posting {
            account: AccountId::new(credit).unwrap(),
            amount: Money::parse(amount, eur()).unwrap(),
            side: Side::Credit,
            vat_code: None,
        },
    ];
    let draft = VoucherDraft::new(d, d, "test".into(), postings, None).unwrap();
    ledger
        .seal(
            &draft,
            &SealRequest {
                natural_key: nk.into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
}

type HashedVoucherRow = (u64, Vec<u8>, Vec<u8>, Vec<u8>);

fn hashed_voucher_rows(home: &std::path::Path) -> Vec<HashedVoucherRow> {
    let conn = rusqlite::Connection::open(home.join("fiscal.sqlite")).unwrap();
    let mut statement = conn
        .prepare("SELECT number, prev_hash, hash, canonical_bytes FROM vouchers ORDER BY number")
        .unwrap();
    statement
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn seal_fiscal_year(ledger: &Ledger, year: i32) {
    for quarter in 1..=4 {
        let period = PeriodId::new(year, quarter).unwrap();
        let run = format!("close-{period}");
        let authority = ledger.begin_close(&period, &run, "test").unwrap();
        ledger.seal_period(&period, &authority, "test").unwrap();
    }
    let view = ledger
        .seal_year(
            year,
            &format!("{year:064x}"),
            &format!("close-year-{year}"),
            "test",
        )
        .unwrap();
    assert_eq!(view.state, FiscalYearState::Sealed);
}

/// Signed opening balance for `account`: debit positive, credit negative.
fn opening_signed(opening: &OpeningBalance, account: &str) -> Decimal {
    opening
        .postings()
        .iter()
        .filter(|p| p.account.as_str() == account)
        .map(Posting::signed_amount)
        .sum()
}

/// Seal a first year (2025) with a capital injection, a sale and an expense —
/// result 8000, bank 13000.
fn seal_first_year(ledger: &Ledger) {
    seal(ledger, "2025-01-05", "1910", "2000", "5000.00", "cap");
    seal(ledger, "2025-03-31", "1910", "3000", "10000.00", "sale");
    seal(ledger, "2025-06-30", "7690", "1910", "2000.00", "exp");
}

#[test]
fn the_first_fiscal_year_has_no_opening_balance() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_first_year(&ledger);

    let data = ledger.fiscal_year_data(2025).unwrap();
    assert_eq!(data.vouchers.len(), 3);
    assert!(data.opening_balance.is_none());
}

#[test]
fn opening_balance_is_first_class_and_year_vouchers_remain_sealed_records() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_first_year(&ledger);

    seal_fiscal_year(&ledger, 2025);
    // No 2026 voucher exists: the carried position is a separate model value.
    let data = ledger.fiscal_year_data(2026).unwrap();
    assert!(data.vouchers.is_empty());
    assert!(ledger.year_vouchers(2026).unwrap().is_empty());
    let opening = data.opening_balance.as_ref().unwrap();
    assert_eq!(opening.fiscal_year(), 2026);

    // Bank carries its full closing balance forward (5000 + 10000 − 2000 = 13000).
    assert_eq!(opening_signed(opening, "1910"), Decimal::from(13000));
    // Share capital carries forward as an equity credit (−5000 signed).
    assert_eq!(opening_signed(opening, "2000"), Decimal::from(-5000));
    // The prior year's result (revenue 10000 − expense 2000 = 8000) is reset at the
    // year boundary and rolled into retained earnings 2250 as an equity credit.
    assert_eq!(opening_signed(opening, "2250"), Decimal::from(-8000));
    // Income/expense accounts do NOT carry forward — they reset at the boundary.
    assert_eq!(opening_signed(opening, "3000"), Decimal::ZERO);
    assert_eq!(opening_signed(opening, "7690"), Decimal::ZERO);
}

#[test]
fn statements_carry_prior_balances_and_stay_balanced() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_first_year(&ledger);
    seal_fiscal_year(&ledger, 2025);
    // A modest 2026 sale so the year also has its own movement.
    seal(&ledger, "2026-02-01", "1910", "3000", "1000.00", "sale26");

    let chart = ledger.chart_list(None).unwrap();
    let provenance = StatementProvenance {
        chart_version: ledger.chart_version().unwrap(),
        policy_set: "fi.core.2026-01@stub".into(),
        kpa_policy_version: kpa_policy_version().to_string(),
    };
    let data = ledger.fiscal_year_data(2026).unwrap();
    let s = compute_statements(
        &data.fiscal_year,
        data.opening_balance.as_ref(),
        &data.vouchers,
        &chart,
        kpa_policy(),
        provenance,
    )
    .unwrap();

    // 2026's own result is just the new sale (1000); prior years' result is in equity,
    // not in this year's income statement.
    assert_eq!(s.result, "1000.00");
    // Assets: bank 13000 (opening) + 1000 (2026 sale) = 14000.
    assert_eq!(s.assets_total, "14000.00");
    // Equity+liabilities: capital 5000 + retained 8000 + this year's result 1000 = 14000.
    assert_eq!(s.equity_liabilities_total, "14000.00");
    assert!(
        s.balanced,
        "the balance-sheet identity holds with opening balances"
    );
}

#[test]
fn a_prior_loss_carries_a_debit_to_retained_earnings() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    // 2025: capital 5000, a small sale 1000, a large expense 4000 → result −3000 (loss).
    seal(&ledger, "2025-01-05", "1910", "2000", "5000.00", "cap");
    seal(&ledger, "2025-03-31", "1910", "3000", "1000.00", "sale");
    seal(&ledger, "2025-06-30", "7690", "1910", "4000.00", "exp");
    seal_fiscal_year(&ledger, 2025);

    let data = ledger.fiscal_year_data(2026).unwrap();
    let opening = data.opening_balance.as_ref().unwrap();
    // Bank: 5000 + 1000 − 4000 = 2000 (debit).
    assert_eq!(opening_signed(opening, "1910"), Decimal::from(2000));
    // The loss reduces equity: retained earnings 2250 carries a +3000 DEBIT.
    assert_eq!(opening_signed(opening, "2250"), Decimal::from(3000));
    assert_eq!(opening_signed(opening, "2000"), Decimal::from(-5000));
}

#[test]
fn opening_balances_accumulate_across_multiple_prior_years() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    // 2025: capital 5000, sale 10000, expense 2000 → result 8000.
    seal_first_year(&ledger);
    seal_fiscal_year(&ledger, 2025);
    // 2026: another sale 3000, expense 1000 → result 2000.
    seal(&ledger, "2026-04-01", "1910", "3000", "3000.00", "sale26");
    seal(&ledger, "2026-07-01", "7690", "1910", "1000.00", "exp26");
    seal_fiscal_year(&ledger, 2026);

    // Closing 2027 must fold BOTH prior years: retained earnings = 8000 + 2000 = 10000,
    // bank = 13000 (2025) + 2000 (2026) = 15000.
    let data = ledger.fiscal_year_data(2027).unwrap();
    let opening = data.opening_balance.as_ref().unwrap();
    assert_eq!(opening_signed(opening, "1910"), Decimal::from(15000));
    assert_eq!(opening_signed(opening, "2250"), Decimal::from(-10000));
    assert_eq!(opening_signed(opening, "2000"), Decimal::from(-5000));
    // Income/expense accounts never carry forward.
    assert_eq!(opening_signed(opening, "3000"), Decimal::ZERO);
    assert_eq!(opening_signed(opening, "7690"), Decimal::ZERO);
}

#[test]
fn direct_prior_postings_to_retained_earnings_combine_with_the_rolled_result() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_first_year(&ledger); // result 8000
                              // A prior dividend distribution debits retained earnings 2250 directly by 1000.
    seal(&ledger, "2025-12-15", "2250", "1910", "1000.00", "div");
    seal_fiscal_year(&ledger, 2025);

    let data = ledger.fiscal_year_data(2026).unwrap();
    let opening = data.opening_balance.as_ref().unwrap();
    // 2250: rolled result −8000 (credit) + direct debit +1000 = −7000 net (credit 7000).
    assert_eq!(opening_signed(opening, "2250"), Decimal::from(-7000));
    // Bank: 13000 − 1000 dividend = 12000.
    assert_eq!(opening_signed(opening, "1910"), Decimal::from(12000));
}

#[test]
fn tase_erittelyt_reflects_carried_opening_balances_in_closing_balances() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_first_year(&ledger); // 2025 closing bank = 13000
    seal_fiscal_year(&ledger, 2025);

    let chart = ledger.chart_list(None).unwrap();
    let provenance = StatementProvenance {
        chart_version: ledger.chart_version().unwrap(),
        policy_set: "fi.core.2026-01@stub".into(),
        kpa_policy_version: kpa_policy_version().to_string(),
    };
    // No 2026 activity of its own: the erittely closing balance is purely the opening.
    let data = ledger.fiscal_year_data(2026).unwrap();
    let te = tase_erittelyt(
        &data.fiscal_year,
        data.opening_balance.as_ref(),
        &data.vouchers,
        &chart,
        provenance,
    )
    .unwrap();

    // The balance specification carries the prior year's closing position forward:
    // bank 13000, share capital 5000, retained earnings 8000.
    let bank = te.lines.iter().find(|l| l.account == "1910").unwrap();
    assert_eq!(bank.closing_balance, "13000.00");
    let capital = te.lines.iter().find(|l| l.account == "2000").unwrap();
    assert_eq!(capital.closing_balance, "5000.00");
    let retained = te.lines.iter().find(|l| l.account == "2250").unwrap();
    assert_eq!(retained.closing_balance, "8000.00");
    assert_eq!(
        bank.source_refs,
        vec![StatementSourceRef::OpeningBalance {
            source: data.opening_balance.as_ref().unwrap().source().clone(),
        }]
    );
    // Income/expense accounts are not balance-sheet lines and never appear.
    assert!(te
        .lines
        .iter()
        .all(|l| l.account != "3000" && l.account != "7690"));
}

#[test]
fn deriving_opening_data_does_not_change_voucher_hash_preimages_or_chain() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_first_year(&ledger);
    seal_fiscal_year(&ledger, 2025);
    let before_report = ledger.verify().unwrap();
    let before_rows = hashed_voucher_rows(home.path());

    let data = ledger.fiscal_year_data(2026).unwrap();
    assert!(data.opening_balance.is_some());
    let after_report = ledger.verify().unwrap();
    let after_rows = hashed_voucher_rows(home.path());

    assert_eq!(
        after_rows, before_rows,
        "opening redesign changed hashed voucher bytes"
    );
    assert_eq!(after_report.head_hash, before_report.head_hash);
    assert_eq!(after_report.voucher_count, before_report.voucher_count);
}

#[test]
fn opening_balance_requires_the_immediately_prior_year_to_be_sealed() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_first_year(&ledger);
    let err = ledger.fiscal_year_data(2026).unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED");
    assert!(err.to_string().contains("2025"));
}

#[test]
fn net_zero_prior_activity_does_not_bypass_the_seal_precondition() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal(&ledger, "2025-01-01", "1910", "2000", "10.00", "in");
    seal(&ledger, "2025-01-02", "2000", "1910", "10.00", "out");

    let err = ledger.fiscal_year_data(2026).unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED");
    assert!(err.to_string().contains("2025"));
}

/// Books that start at the turn of the fiscal year: the old system's closing position
/// is one opening voucher dated its last day, 31.12.2025.
fn seal_boundary_opening(ledger: &Ledger) {
    seal(
        ledger,
        "2025-12-31",
        "1910",
        "2000",
        "2500.00",
        "open-capital",
    );
    seal(ledger, "2025-12-31", "1910", "2400", "400.00", "open-loan");
}

#[test]
fn a_year_boundary_opening_voucher_opens_the_first_year_without_sealing_the_old_one() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_boundary_opening(&ledger);
    seal(&ledger, "2026-01-15", "1910", "3000", "100.00", "sale");

    let data = ledger.fiscal_year_data(2026).unwrap();
    assert_eq!(
        data.vouchers.len(),
        1,
        "the opening vouchers are not 2026 activity"
    );
    let opening = data.opening_balance.as_ref().expect("an opening balance");
    assert_eq!(opening_signed(opening, "1910"), Decimal::from(2900));
    assert_eq!(opening_signed(opening, "2000"), Decimal::from(-2500));
    assert_eq!(opening_signed(opening, "2400"), Decimal::from(-400));
    let OpeningBalanceSource::ExternalCutover {
        prior_year,
        source_head_hash,
        source_chart_version,
        target_chart_version,
        live_period,
        ..
    } = opening.source()
    else {
        panic!("expected an external cut-over, got {:?}", opening.source());
    };
    assert_eq!(*prior_year, 2025);
    assert_eq!(live_period, &PeriodId::parse("2026-01").unwrap());
    assert_eq!(source_chart_version, &ledger.chart_version().unwrap());
    assert_eq!(target_chart_version, &ledger.chart_version().unwrap());
    // Bound to the chain hash of the last opening voucher (number 2).
    assert_eq!(
        source_head_hash.to_hex(),
        ledger.voucher(2).unwrap().voucher_hash
    );

    // The issue's reproduction: the first month's report now produces statements.
    let january = PeriodId::parse("2026-01").unwrap();
    let report = ledger
        .monthly_statements_report(&january, "Esimerkki Oy", MonthlyReportOptions::default())
        .unwrap();
    assert!(report.sections.balance.balanced);
    assert_eq!(report.sections.balance.assets_total, "3000.00");
}

#[test]
fn a_boundary_started_year_closes_and_opens_the_next_from_the_sealed_year() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_boundary_opening(&ledger);
    seal(&ledger, "2026-01-15", "1910", "3000", "100.00", "sale");

    seal_fiscal_year(&ledger, 2026);
    let data = ledger.fiscal_year_data(2027).unwrap();
    let opening = data.opening_balance.as_ref().unwrap();
    assert!(matches!(
        opening.source(),
        OpeningBalanceSource::PriorFiscalYear {
            prior_year: 2026,
            ..
        }
    ));
    assert_eq!(opening_signed(opening, "1910"), Decimal::from(3000));
    assert_eq!(opening_signed(opening, "2250"), Decimal::from(-100));
}

#[test]
fn a_mid_year_opening_voucher_stays_in_its_own_year() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal(
        &ledger,
        "2026-05-31",
        "1910",
        "2000",
        "2500.00",
        "open-capital",
    );
    seal(&ledger, "2026-06-15", "1910", "3000", "100.00", "sale");

    let data = ledger.fiscal_year_data(2026).unwrap();
    assert!(data.opening_balance.is_none());
    assert_eq!(data.vouchers.len(), 2);
}

#[test]
fn an_unsealed_prior_year_with_a_result_still_refuses() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_boundary_opening(&ledger);
    seal(&ledger, "2025-12-31", "1910", "3000", "50.00", "late-sale");

    let err = ledger.fiscal_year_data(2026).unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED");
    assert!(err.to_string().contains("2025"), "{err}");
}

#[test]
fn an_unsealed_prior_year_with_activity_before_its_last_day_still_refuses() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_boundary_opening(&ledger);
    seal(&ledger, "2025-12-30", "1910", "2000", "10.00", "early");

    let err = ledger.fiscal_year_data(2026).unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED");
}

#[test]
fn activity_before_the_prior_year_still_refuses() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal(&ledger, "2024-12-31", "1910", "2000", "10.00", "older");
    seal_boundary_opening(&ledger);

    let err = ledger.fiscal_year_data(2026).unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED");
}

#[test]
fn a_prior_year_with_a_period_closed_in_aggountant_still_refuses() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_boundary_opening(&ledger);
    let q4 = PeriodId::new(2025, 4).unwrap();
    let authority = ledger.begin_close(&q4, "close-q4", "test").unwrap();
    ledger.seal_period(&q4, &authority, "test").unwrap();

    let err = ledger.fiscal_year_data(2026).unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED");
}
