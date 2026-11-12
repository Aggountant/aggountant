//! S6 `close-year` (RF6) process tests — the point of the slice is *these*: the
//! precondition + must-seal gating, the D7 pre-close reclassification invariant, the
//! long multi-gate resume, the `ApproveStatements` Preview tamper-refusal, the
//! irreversible `SealYear`, version-pin resume, and restart-from-fiscal.

use std::fs;

use aggountant_domain::{AccrualInput, AssetInput, PolicySet};
use aggountant_ledger::{FiscalYearState, Ledger, SealRequest};
use aggountant_model::{AccountId, Currency, Date, Money, PeriodId, Posting, Side, VoucherDraft};
use aggountant_process::{
    CloseYearInput, CorrectionInput, EffectStatus, ProcessEngine, RunStatus, RunView,
};

fn eur() -> Currency {
    Currency::new("EUR").unwrap()
}
fn money(s: &str) -> Money {
    Money::parse(s, eur()).unwrap()
}
fn acct(s: &str) -> AccountId {
    AccountId::new(s).unwrap()
}

fn init_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    home
}

fn posting(acc: &str, amount: &str, side: Side) -> Posting {
    Posting {
        account: acct(acc),
        amount: money(amount),
        side,
        vat_code: None,
    }
}

/// Seal a balanced voucher (`debit`/`credit` accounts) on `date` into its (Open) period.
fn seal_voucher(ledger: &Ledger, date: &str, debit: &str, credit: &str, amount: &str, nk: &str) {
    let d = Date::parse(date).unwrap();
    let draft = VoucherDraft::new(
        d,
        d,
        "test voucher".into(),
        vec![
            posting(debit, amount, Side::Debit),
            posting(credit, amount, Side::Credit),
        ],
        None,
    )
    .unwrap();
    ledger
        .seal(
            &draft,
            &SealRequest {
                natural_key: nk.to_string(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
}

/// Seed a year with some sealed activity, then seal all four quarters.
fn setup_sealed_year(home: &std::path::Path, year: i32) {
    let ledger = Ledger::open(home).unwrap();
    // Capital injection + a sale + an expense, spread across quarters.
    seal_voucher(
        &ledger,
        &format!("{year}-01-05"),
        "1910",
        "2000",
        "5000.00",
        "cap",
    );
    seal_voucher(
        &ledger,
        &format!("{year}-03-31"),
        "1910",
        "3000",
        "10000.00",
        "sale",
    );
    seal_voucher(
        &ledger,
        &format!("{year}-06-30"),
        "7690",
        "1910",
        "2000.00",
        "exp",
    );
    for q in 1..=4 {
        let p = PeriodId::new(year, q).unwrap();
        let auth = ledger
            .begin_close(&p, &format!("cp-{year}-{q}"), "test")
            .unwrap();
        ledger.seal_period(&p, &auth, "test").unwrap();
    }
}

fn asset() -> AssetInput {
    AssetInput {
        asset_id: "kone-1".into(),
        asset_class: "koneet-ja-kalusto".into(),
        expense_account: acct("6000"),
        accumulated_account: acct("1160"),
        acquisition_cost: money("10000.00"),
        book_value_start: money("8000.00"),
    }
}

fn base_input(year: i32) -> CloseYearInput {
    CloseYearInput {
        year,
        assets: vec![asset()],
        accruals: vec![AccrualInput {
            accrual_id: "sahko".into(),
            description: "sähkölaskun jaksotus".into(),
            debit_account: acct("7010"),
            credit_account: acct("2760"),
            amount: money("300.00"),
        }],
        corrections: vec![],
    }
}

fn eff(view: &RunView, key: &str) -> EffectStatus {
    view.effects.iter().find(|e| e.key == key).unwrap().status
}

fn approve(engine: &ProcessEngine, run_id: &str, gate: &str) -> RunView {
    engine.answer_close_year(run_id, gate, "approve").unwrap()
}

#[test]
fn happy_path_seals_the_year_and_verifies() {
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());

    let started = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap();
    assert_eq!(started.run.status, RunStatus::AwaitingGate);
    assert_eq!(started.run.gates[0].gate_id, "ApproveAdjustments");
    // Depreciation is a Domain output (25 % of 8000 = 2000.00), not agent input.
    assert_eq!(started.depreciation[0].amount.amount_str(), "2000.00");
    let run_id = started.run.run_id.clone();

    let v = approve(&engine, &run_id, "ApproveAdjustments");
    assert_eq!(eff(&v, "seal-adjustments"), EffectStatus::Succeeded);
    assert_eq!(eff(&v, "compute-statements"), EffectStatus::Succeeded);
    assert_eq!(
        v.gates.iter().last().unwrap().gate_id,
        "ApproveTaseErittelyt"
    );

    approve(&engine, &run_id, "ApproveTaseErittelyt");
    approve(&engine, &run_id, "ApproveLiitetiedot");
    let done = approve(&engine, &run_id, "ApproveStatements");
    assert_eq!(done.status, RunStatus::Completed);
    assert_eq!(eff(&done, "seal-year"), EffectStatus::Succeeded);

    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.fiscal_year_view(2026).unwrap().state,
        FiscalYearState::Sealed
    );
    // The bound statements_hash is present on the sealed year (what-approved ==
    // what-sealed).
    assert!(ledger
        .fiscal_year_view(2026)
        .unwrap()
        .statements_hash
        .is_some());
    assert!(ledger.verify().unwrap().ok);
    // The depreciation voucher was sealed into the next open period (2027-Q1).
    let next = ledger.year_vouchers(2027).unwrap();
    assert!(next.iter().any(|v| v
        .postings()
        .iter()
        .any(|p| p.account.as_str() == "6000" && p.amount.amount_str() == "2000.00")));
    // The fiscal-archive stub was written (Fork C option b).
    assert!(home.path().join("archive/2026/tilinpaatos.json").exists());
}

#[test]
fn refuses_to_start_with_an_unsealed_subperiod() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    // Seal only Q1..Q3.
    for q in 1..=3 {
        let p = PeriodId::new(2026, q).unwrap();
        let auth = ledger.begin_close(&p, "cp", "test").unwrap();
        ledger.seal_period(&p, &auth, "test").unwrap();
    }
    let engine = ProcessEngine::open(home.path());
    let err = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap_err();
    assert_eq!(err.error_code(), "CLOSE_YEAR_BLOCKED");
    assert!(err.to_string().contains("subperiod-not-sealed:2026-Q4"));
}

#[test]
fn must_seal_blocks_on_an_open_obligation() {
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let obl = engine
        .obligations()
        .create(
            &PeriodId::new(2026, 2).unwrap(),
            "ajopaivakirja-missing",
            "ajopäiväkirja puuttuu",
            "2027-01-31",
            "some-run",
        )
        .unwrap();
    let err = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap_err();
    assert_eq!(err.error_code(), "CLOSE_YEAR_BLOCKED");
    assert!(err
        .to_string()
        .contains(&format!("open-obligation:{}", obl.id)));
}

#[test]
fn a_d7_correction_must_be_sealed_before_approve_statements() {
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let mut input = base_input(2026);
    input.corrections = vec![CorrectionInput {
        correction_id: "reclass-1".into(),
        description: "uudelleenluokittelu".into(),
        debit_account: acct("7780"),
        credit_account: acct("7690"),
        amount: money("150.00"),
    }];
    let started = engine
        .start_close_year(&input, &PolicySet::default())
        .unwrap();
    let run_id = started.run.run_id.clone();

    approve(&engine, &run_id, "ApproveAdjustments");
    approve(&engine, &run_id, "ApproveTaseErittelyt");
    // ApproveLiitetiedot approve is blocked while the D7 correction is unsealed.
    let err = engine
        .answer_close_year(&run_id, "ApproveLiitetiedot", "approve")
        .unwrap_err();
    assert_eq!(err.error_code(), "CLOSE_YEAR_BLOCKED");
    assert!(err.to_string().contains("correction:reclass-1"));

    // Seal the correction on its own approval, then the gate is reachable.
    let v = engine
        .seal_close_year_correction(&run_id, "reclass-1")
        .unwrap();
    assert_eq!(eff(&v, "correction:reclass-1"), EffectStatus::Succeeded);
    let v2 = approve(&engine, &run_id, "ApproveLiitetiedot");
    assert_eq!(v2.gates.iter().last().unwrap().gate_id, "ApproveStatements");
    let done = approve(&engine, &run_id, "ApproveStatements");
    assert_eq!(done.status, RunStatus::Completed);
}

#[test]
fn crash_across_gate_three_resumes_to_gate_three() {
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap();
    let run_id = started.run.run_id.clone();
    approve(&engine, &run_id, "ApproveAdjustments");
    approve(&engine, &run_id, "ApproveTaseErittelyt");
    // Parked at gate 3 (ApproveLiitetiedot). Simulate a process restart.
    drop(engine);
    let engine2 = ProcessEngine::open(home.path());
    let resumed = engine2.resume_close_year(&run_id).unwrap();
    assert_eq!(resumed.status, RunStatus::AwaitingGate);
    let open_gate = resumed.gates.iter().find(|g| g.answered.is_none()).unwrap();
    assert_eq!(open_gate.gate_id, "ApproveLiitetiedot");
    // And the run can still complete from there.
    approve(&engine2, &run_id, "ApproveLiitetiedot");
    let done = approve(&engine2, &run_id, "ApproveStatements");
    assert_eq!(done.status, RunStatus::Completed);
}

#[test]
fn approve_statements_refuses_tampered_statement_bytes() {
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap();
    let run_id = started.run.run_id.clone();
    approve(&engine, &run_id, "ApproveAdjustments");
    approve(&engine, &run_id, "ApproveTaseErittelyt");
    approve(&engine, &run_id, "ApproveLiitetiedot");

    // Tamper the persisted statements (change the sealed result), then approve.
    let state_path = home
        .path()
        .join("process/runs")
        .join(&run_id)
        .join("close_year_state.json");
    let raw = fs::read_to_string(&state_path).unwrap();
    let tampered = raw.replace("\"result\": \"8000.00\"", "\"result\": \"9999.00\"");
    assert_ne!(raw, tampered, "the tamper must actually change the bytes");
    fs::write(&state_path, tampered).unwrap();

    let err = engine
        .answer_close_year(&run_id, "ApproveStatements", "approve")
        .unwrap_err();
    assert_eq!(err.error_code(), "JOURNAL_CORRUPT");
    // The year is not sealed — the tamper was refused.
    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.fiscal_year_view(2026).unwrap().state,
        FiscalYearState::Open
    );
}

#[test]
fn a_sealed_year_refuses_a_second_close_year() {
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap();
    let run_id = started.run.run_id.clone();
    approve(&engine, &run_id, "ApproveAdjustments");
    approve(&engine, &run_id, "ApproveTaseErittelyt");
    approve(&engine, &run_id, "ApproveLiitetiedot");
    approve(&engine, &run_id, "ApproveStatements");

    let err = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap_err();
    assert!(err.to_string().contains("already sealed"));
}

#[test]
fn version_pin_resume_holds_for_its_own_version() {
    // A close-year Run started under the engine's version resumes under it without
    // tripping WORKFLOW_VERSION_MISMATCH (M1 version-pin, `workflow-migration §4`).
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap();
    let run_id = started.run.run_id.clone();
    // A fresh engine of the same build resumes cleanly (no migrate needed).
    let engine2 = ProcessEngine::open(home.path());
    let resumed = engine2.resume_close_year(&run_id).unwrap();
    assert_eq!(resumed.status, RunStatus::AwaitingGate);
}

#[test]
fn restart_from_fiscal_reconstructs_from_sealed_facts() {
    // M3 restart-from-fiscal: a fresh close-year Run seeded from the sealed
    // subperiods re-plans and completes; the adjustment seals are idempotent
    // (ledger natural_key), so nothing double-books.
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());

    // Drive the first Run partway (through sealing the adjustments), then abandon it.
    let first = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap();
    approve(&engine, &first.run.run_id, "ApproveAdjustments");

    // A brand-new Run (the process Run is disposable; fiscal facts are the truth).
    let second = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap();
    let run_id = second.run.run_id.clone();
    approve(&engine, &run_id, "ApproveAdjustments");
    approve(&engine, &run_id, "ApproveTaseErittelyt");
    approve(&engine, &run_id, "ApproveLiitetiedot");
    let done = approve(&engine, &run_id, "ApproveStatements");
    assert_eq!(done.status, RunStatus::Completed);

    // The depreciation voucher was booked exactly once despite two Runs sealing it.
    let ledger = Ledger::open(home.path()).unwrap();
    let dep_count = ledger
        .year_vouchers(2027)
        .unwrap()
        .iter()
        .filter(|v| v.postings().iter().any(|p| p.account.as_str() == "6000"))
        .count();
    assert_eq!(
        dep_count, 1,
        "the adjustment seal is idempotent across Runs"
    );
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn duplicate_asset_ids_are_refused() {
    // Two assets sharing an id would collide their natural key `closeyear:Y:dep:id`
    // and silently drop the second — reject up front.
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let mut input = base_input(2026);
    input.assets = vec![asset(), asset()]; // same asset_id "kone-1"
    let err = engine
        .start_close_year(&input, &PolicySet::default())
        .unwrap_err();
    assert_eq!(err.error_code(), "INVALID_ANSWER");
    assert!(err.to_string().contains("duplicate asset_id"));
}

#[test]
fn rejecting_a_gate_declines_the_close() {
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap();
    let run_id = started.run.run_id.clone();
    let v = engine
        .answer_close_year(&run_id, "ApproveAdjustments", "reject")
        .unwrap();
    assert_eq!(v.status, RunStatus::Rejected);
    // The year stays Open — a declined close seals nothing.
    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.fiscal_year_view(2026).unwrap().state,
        FiscalYearState::Open
    );
}

#[test]
fn monthly_year_uses_twelve_months_and_next_month_for_adjustments() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    for month in 1..=12 {
        let period = PeriodId::new_month(2026, month).unwrap();
        let authority = ledger
            .begin_close(&period, &format!("month-{month:02}"), "test")
            .unwrap();
        ledger.seal_period(&period, &authority, "test").unwrap();
    }
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap();
    assert_eq!(started.run.period, "2026-12");
    let state: serde_json::Value = serde_json::from_slice(
        &fs::read(
            home.path()
                .join("process/runs")
                .join(&started.run.run_id)
                .join("close_year_state.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(state["subperiods"].as_array().unwrap().len(), 12);
    assert_eq!(state["next_open_period"], "2027-01");
    assert_eq!(
        engine
            .migrate(&started.run.run_id)
            .unwrap_err()
            .error_code(),
        "INVALID_ANSWER",
        "a current v2 close-year run must not migrate to close-period's version"
    );

    let run_id = started.run.run_id;
    approve(&engine, &run_id, "ApproveAdjustments");
    approve(&engine, &run_id, "ApproveTaseErittelyt");
    approve(&engine, &run_id, "ApproveLiitetiedot");
    let done = approve(&engine, &run_id, "ApproveStatements");
    assert_eq!(done.status, RunStatus::Completed);
    assert_eq!(
        ledger.fiscal_year_view(2026).unwrap().state,
        FiscalYearState::Sealed
    );
    assert!(!ledger
        .period_vouchers(&PeriodId::parse("2027-01").unwrap())
        .unwrap()
        .is_empty());
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn monthly_close_year_uses_the_target_years_active_quarter() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    for month in 1..=12 {
        let period = PeriodId::new_month(2026, month).unwrap();
        let authority = ledger.begin_close(&period, "monthly", "test").unwrap();
        ledger.seal_period(&period, &authority, "test").unwrap();
    }
    let q1_2027 = PeriodId::parse("2027-Q1").unwrap();
    let authority = ledger.begin_close(&q1_2027, "q1-2027", "test").unwrap();
    ledger.seal_period(&q1_2027, &authority, "test").unwrap();

    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap();
    let state: serde_json::Value = serde_json::from_slice(
        &fs::read(
            home.path()
                .join("process/runs")
                .join(&started.run.run_id)
                .join("close_year_state.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(state["next_open_period"], "2027-Q2");
    assert_eq!(state["posting_date"], "2027-04-01");

    let run_id = started.run.run_id;
    approve(&engine, &run_id, "ApproveAdjustments");
    approve(&engine, &run_id, "ApproveTaseErittelyt");
    approve(&engine, &run_id, "ApproveLiitetiedot");
    assert_eq!(
        approve(&engine, &run_id, "ApproveStatements").status,
        RunStatus::Completed
    );
    assert!(ledger.verify().unwrap().ok);
}

/// A July–June company (issue `non-calendar-fiscal-year`): 1.7.2026–30.6.2027 is
/// fiscal year 2027, closed by its twelve months.
fn setup_sealed_july_year(home: &std::path::Path) {
    let ledger = Ledger::open(home).unwrap();
    ledger
        .define_fiscal_year(
            Date::parse("2026-07-01").unwrap(),
            Date::parse("2027-06-30").unwrap(),
            "test",
        )
        .unwrap();
    seal_voucher(&ledger, "2026-07-01", "1910", "2000", "5000.00", "cap");
    seal_voucher(&ledger, "2026-11-30", "1910", "3000", "10000.00", "sale");
    seal_voucher(&ledger, "2027-06-30", "7690", "1910", "2000.00", "exp");
    for period in ledger.year_close_periods(2027).unwrap() {
        let auth = ledger
            .begin_close(&period, &format!("cp-{period}"), "test")
            .unwrap();
        ledger.seal_period(&period, &auth, "test").unwrap();
    }
}

#[test]
fn a_july_to_june_year_closes_into_july() {
    let home = init_home();
    setup_sealed_july_year(home.path());
    let engine = ProcessEngine::open(home.path());

    let started = engine
        .start_close_year(&base_input(2027), &PolicySet::default())
        .unwrap();
    assert_eq!(started.depreciation[0].amount.amount_str(), "2000.00");
    let run_id = started.run.run_id.clone();
    approve(&engine, &run_id, "ApproveAdjustments");
    approve(&engine, &run_id, "ApproveTaseErittelyt");
    approve(&engine, &run_id, "ApproveLiitetiedot");
    let done = approve(&engine, &run_id, "ApproveStatements");
    assert_eq!(done.status, RunStatus::Completed);

    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.fiscal_year_view(2027).unwrap().state,
        FiscalYearState::Sealed
    );
    // Adjustments are dated the fiscal year's last day and posted to the first open
    // month of the next fiscal year.
    let depreciation = ledger
        .year_vouchers(2028)
        .unwrap()
        .into_iter()
        .find(|v| v.postings().iter().any(|p| p.account.as_str() == "6000"))
        .unwrap();
    assert_eq!(depreciation.document_date().to_string(), "2027-06-30");
    assert_eq!(depreciation.posting_date().to_string(), "2027-07-01");

    let archive: serde_json::Value = serde_json::from_slice(
        &fs::read(home.path().join("archive/2027/tilinpaatos.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(archive["statements"]["fiscal_year"], 2027);
    assert_eq!(archive["statements"]["period"]["start"], "2026-07-01");
    assert_eq!(archive["statements"]["period"]["end"], "2027-06-30");
    assert_eq!(archive["statements"]["result"], "8000.00");
}

#[test]
fn depreciation_in_a_short_fiscal_year_is_refused() {
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    ledger
        .define_fiscal_year(
            Date::parse("2026-07-01").unwrap(),
            Date::parse("2026-12-31").unwrap(),
            "test",
        )
        .unwrap();
    for period in ledger.year_close_periods(2026).unwrap() {
        let auth = ledger
            .begin_close(&period, &format!("cp-{period}"), "test")
            .unwrap();
        ledger.seal_period(&period, &auth, "test").unwrap();
    }
    let engine = ProcessEngine::open(home.path());
    let err = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap_err();
    assert_eq!(err.error_code(), "CLOSE_YEAR_BLOCKED");
    assert!(err.to_string().contains("DEPRECIATION"), "{err}");
    assert!(err.to_string().contains("6 months"), "{err}");
}
