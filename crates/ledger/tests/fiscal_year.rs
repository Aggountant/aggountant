//! S6 fiscal-year (`SealYear`) tests at the Ledger (fiscal) level. The point of S6
//! is *these* — the irreversible year lock, its preconditions, the double-seal CAS,
//! the concurrency guard, and the `fold(fiscal_year_transition) == fiscal_year`
//! verify check — not the happy path.

use std::sync::{Arc, Barrier};
use std::thread;

use aggountant_domain::PolicySet;
use aggountant_ledger::{FiscalYearState, Ledger, LedgerError, PeriodState, SealRequest};
use aggountant_model::{AccountId, Currency, Date, Money, PeriodId, Posting, Side, VoucherDraft};

fn init_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    home
}

/// A well-formed 64-char lowercase hex statements_hash for tests (the seal validates
/// the format).
fn hx(seed: u32) -> String {
    format!("{seed:064x}")
}

/// Seal all four quarters of `year` (empty periods: begin_close → seal_period).
fn seal_all_quarters(ledger: &Ledger, year: i32) {
    for q in 1..=4 {
        let p = PeriodId::new(year, q).unwrap();
        let run = format!("close-period-{year}-Q{q}");
        let auth = ledger.begin_close(&p, &run, "cli").unwrap();
        ledger.seal_period(&p, &auth, "cli").unwrap();
        assert_eq!(ledger.period_view(&p).unwrap().state, PeriodState::Sealed);
    }
}

#[test]
fn seal_year_locks_the_year_when_all_subperiods_are_sealed() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_all_quarters(&ledger, 2026);

    assert_eq!(
        ledger.fiscal_year_view(2026).unwrap().state,
        FiscalYearState::Open
    );
    let view = ledger
        .seal_year(2026, &hx(0xabc), &Default::default(), "cy-run-1", "cli")
        .unwrap();
    assert_eq!(view.state, FiscalYearState::Sealed);
    assert_eq!(view.statements_hash.as_deref(), Some(hx(0xabc).as_str()));
    assert_eq!(view.sealed_by_run_id.as_deref(), Some("cy-run-1"));

    // The verify walk covers the fiscal-year chain.
    let report = ledger.verify().unwrap();
    assert!(report.ok);
    assert_eq!(report.fiscal_year_count, 1);
}

#[test]
fn seal_year_refuses_to_depend_on_an_unsealed_prior_year() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    // Mid-year activity: 2025 is a year kept in aggountant, not an opening cut-over.
    let date = Date::parse("2025-06-30").unwrap();
    let currency = Currency::new("EUR").unwrap();
    let draft = VoucherDraft::new(
        date,
        date,
        "prior activity".into(),
        vec![
            Posting {
                account: AccountId::new("1910").unwrap(),
                amount: Money::parse("1.00", currency.clone()).unwrap(),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("2000").unwrap(),
                amount: Money::parse("1.00", currency).unwrap(),
                side: Side::Credit,
                vat_code: None,
            },
        ],
        None,
    )
    .unwrap();
    ledger
        .seal(
            &draft,
            &SealRequest {
                natural_key: "prior".into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
    seal_all_quarters(&ledger, 2026);

    let err = ledger
        .seal_year(2026, &hx(1), &Default::default(), "cy", "test")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED");
    assert!(err.to_string().contains("2025"));
}

#[test]
fn seal_year_refuses_an_unsealed_subperiod() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    // Seal only Q1..Q3; Q4 stays Open.
    for q in 1..=3 {
        let p = PeriodId::new(2026, q).unwrap();
        let auth = ledger.begin_close(&p, "r", "cli").unwrap();
        ledger.seal_period(&p, &auth, "cli").unwrap();
    }
    let err = ledger
        .seal_year(2026, &hx(1), &Default::default(), "cy-run", "cli")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED");
    assert!(
        err.to_string().contains("2026-Q4"),
        "names the blocker: {err}"
    );
    // The year is untouched (still Open).
    assert_eq!(
        ledger.fiscal_year_view(2026).unwrap().state,
        FiscalYearState::Open
    );
}

#[test]
fn a_second_seal_year_fails_the_cas_irreversibly() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_all_quarters(&ledger, 2026);
    ledger
        .seal_year(2026, &hx(1), &Default::default(), "run-a", "cli")
        .unwrap();

    let err = ledger
        .seal_year(2026, &hx(2), &Default::default(), "run-b", "cli")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_TRANSITION_CONFLICT");
    // The bound statements_hash is unchanged — the first seal stands.
    assert_eq!(
        ledger
            .fiscal_year_view(2026)
            .unwrap()
            .statements_hash
            .as_deref(),
        Some(hx(1).as_str())
    );
}

#[test]
fn concurrent_seal_year_lets_exactly_one_win() {
    // Mirrors the S2 8-sealer contention test at the year granularity: many racing
    // seal_year calls, exactly one commits (CAS), the rest lose cleanly.
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_all_quarters(&ledger, 2026);
    drop(ledger);

    let n = 8;
    let barrier = Arc::new(Barrier::new(n));
    let path = home.path().to_path_buf();
    let handles: Vec<_> = (0..n)
        .map(|i| {
            let barrier = Arc::clone(&barrier);
            let path = path.clone();
            thread::spawn(move || {
                let ledger = Ledger::open(&path).unwrap();
                barrier.wait();
                ledger.seal_year(
                    2026,
                    &hx(i as u32),
                    &Default::default(),
                    &format!("run-{i}"),
                    "cli",
                )
            })
        })
        .collect();
    let results: Vec<Result<_, LedgerError>> =
        handles.into_iter().map(|h| h.join().unwrap()).collect();
    let wins = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(wins, 1, "exactly one seal_year must win the CAS");

    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.fiscal_year_view(2026).unwrap().state,
        FiscalYearState::Sealed
    );
    // The chain still verifies after the contention.
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn a_subperiod_cannot_be_reopened_after_the_year_is_sealed() {
    // The year lock must bite the subperiods: once SealYear binds a statements_hash,
    // reopening a quarter would let the ledger diverge from what was sealed.
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_all_quarters(&ledger, 2026);
    ledger
        .seal_year(2026, &hx(7), &Default::default(), "cy", "cli")
        .unwrap();

    let q4 = PeriodId::new(2026, 4).unwrap();
    // The close-period run that sealed Q4 can re-acquire its authority…
    let auth = ledger
        .acquire_authority(&q4, "close-period-2026-Q4")
        .unwrap();
    // …but reopening it is now refused because the fiscal year is Sealed.
    let err = ledger
        .reopen_period(&q4, &auth, PeriodState::Closing, "cli")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_TRANSITION_CONFLICT");
    assert!(
        err.to_string().contains("fiscal year 2026 is Sealed"),
        "{err}"
    );
    // Q4 is untouched.
    assert_eq!(ledger.period_view(&q4).unwrap().state, PeriodState::Sealed);
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn an_unsealed_year_reads_back_open() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let view = ledger.fiscal_year_view(2099).unwrap();
    assert_eq!(view.state, FiscalYearState::Open);
    assert_eq!(view.statements_hash, None);
    assert_eq!(view.head_seq, 0);
}

fn seal_all_months(ledger: &Ledger, year: i32) {
    for month in 1..=12 {
        let period = PeriodId::new_month(year, month).unwrap();
        let run = format!("close-period-{year}-{month:02}");
        let authority = ledger.begin_close(&period, &run, "cli").unwrap();
        ledger.seal_period(&period, &authority, "cli").unwrap();
    }
}

#[test]
fn twelve_months_seal_a_year_but_four_months_do_not() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    for month in 1..=4 {
        let period = PeriodId::new_month(2026, month).unwrap();
        let authority = ledger.begin_close(&period, "monthly", "cli").unwrap();
        ledger.seal_period(&period, &authority, "cli").unwrap();
    }
    let error = ledger
        .seal_year(2026, &hx(8), &Default::default(), "cy", "cli")
        .unwrap_err();
    assert_eq!(error.error_code(), "PERIOD_NOT_SEALED");
    assert!(error.to_string().contains("2026-05"));

    for month in 5..=12 {
        let period = PeriodId::new_month(2026, month).unwrap();
        let authority = ledger.begin_close(&period, "monthly", "cli").unwrap();
        ledger.seal_period(&period, &authority, "cli").unwrap();
    }
    ledger
        .seal_year(2026, &hx(9), &Default::default(), "cy", "cli")
        .unwrap();
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn reopened_month_remnant_allows_legacy_quarter_year() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let january = PeriodId::parse("2026-01").unwrap();
    let authority = ledger.begin_close(&january, "jan", "cli").unwrap();
    ledger.seal_period(&january, &authority, "cli").unwrap();
    ledger
        .reopen_period(&january, &authority, PeriodState::Open, "cli")
        .unwrap();
    seal_all_quarters(&ledger, 2026);
    assert_eq!(
        ledger.year_close_periods(2026).unwrap()[0].to_string(),
        "2026-Q1"
    );
    ledger
        .seal_year(2026, &hx(10), &Default::default(), "cy", "cli")
        .unwrap();
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn reopened_quarter_remnant_allows_monthly_year() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let q1 = PeriodId::parse("2026-Q1").unwrap();
    let authority = ledger.begin_close(&q1, "q1-remnant", "cli").unwrap();
    ledger.seal_period(&q1, &authority, "cli").unwrap();
    ledger
        .reopen_period(&q1, &authority, PeriodState::Open, "cli")
        .unwrap();
    seal_all_months(&ledger, 2026);
    assert_eq!(
        ledger.year_close_periods(2026).unwrap()[0].to_string(),
        "2026-01"
    );
    ledger
        .seal_year(2026, &hx(11), &Default::default(), "cy", "cli")
        .unwrap();
    assert!(ledger.verify().unwrap().ok);
}

// --- the fixed-asset register stored with the year seal -----------------------

fn register(
    year: i32,
    opening: Option<&aggountant_domain::AssetRegister>,
) -> aggountant_domain::AssetRegister {
    use aggountant_domain::{depreciation_rate_table, plan_asset_year, AssetInput};
    let eur = Currency::new("EUR").unwrap();
    let input = if opening.is_some() {
        vec![]
    } else {
        vec![AssetInput {
            asset_id: "machine-1".into(),
            asset_class: "koneet-ja-kalusto".into(),
            expense_account: AccountId::new("6000").unwrap(),
            accumulated_account: AccountId::new("1160").unwrap(),
            acquisition_cost: Money::parse("10000.00", eur.clone()).unwrap(),
            book_value_start: Money::parse("8000.00", eur).unwrap(),
        }]
    };
    plan_asset_year(
        &aggountant_model::FiscalYear::calendar(year).unwrap(),
        opening,
        &input,
        &[],
        depreciation_rate_table(),
    )
    .unwrap()
    .0
}

#[test]
fn seal_year_stores_the_asset_register_and_the_next_year_must_continue_it() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    seal_all_quarters(&ledger, 2026);
    seal_all_quarters(&ledger, 2027);
    assert!(ledger.asset_register(2026).unwrap().is_none());
    assert!(ledger.opening_asset_register(2027).unwrap().is_none());

    let first = register(2026, None);
    ledger
        .seal_year(2026, &hx(1), &first, "cy-2026", "cli")
        .unwrap();
    assert_eq!(ledger.asset_register(2026).unwrap().as_ref(), Some(&first));
    assert_eq!(
        ledger.opening_asset_register(2027).unwrap().as_ref(),
        Some(&first)
    );

    // 2027 sealed with a register that drops the machine: refused, year stays open.
    let err = ledger
        .seal_year(2027, &hx(2), &Default::default(), "cy-2027", "cli")
        .unwrap_err();
    assert_eq!(err.error_code(), "DEPRECIATION");
    assert_eq!(
        ledger.fiscal_year_view(2027).unwrap().state,
        FiscalYearState::Open
    );

    let second = register(2027, Some(&first));
    ledger
        .seal_year(2027, &hx(2), &second, "cy-2027", "cli")
        .unwrap();
    let stored = ledger.asset_register(2027).unwrap().unwrap();
    assert_eq!(stored.assets[0].book_value_start.amount_str(), "6000.00");
    assert_eq!(stored.assets[0].book_value_end.amount_str(), "4500.00");
    assert!(ledger.verify().unwrap().ok);
}
