//! S6 `close-year` (RF6) process tests — the point of the slice is *these*: the
//! precondition + must-seal gating, the D7 pre-close reclassification invariant, the
//! long multi-gate resume, the `ApproveStatements` Preview tamper-refusal, the
//! irreversible `SealYear`, version-pin resume, and restart-from-fiscal.

use std::fs;

use aggountant_domain::{depreciation_voucher, AccrualInput, AssetInput, PolicySet};
use aggountant_ledger::{FiscalYearState, Ledger, MonthlyReportOptions, SealRequest};
use aggountant_model::{AccountId, Currency, Date, Money, PeriodId, Posting, Side, VoucherDraft};
use aggountant_process::{
    CloseYearInput, CloseYearShow, CorrectionInput, EffectStatus, GateContent, ProcessEngine,
    RunStatus, RunView,
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
    // The depreciation and accrual vouchers were sealed into the next open period
    // (2027-Q1) and attributed to 2026, the year whose statements they belong to.
    let q1 = ledger
        .period_vouchers(&PeriodId::new(2027, 1).unwrap())
        .unwrap();
    let depreciation = q1
        .iter()
        .find(|v| {
            v.postings()
                .iter()
                .any(|p| p.account.as_str() == "6000" && p.amount.amount_str() == "2000.00")
        })
        .unwrap();
    assert_eq!(depreciation.posting_date().to_string(), "2027-01-01");
    assert_eq!(q1.len(), 2);
    for voucher in &q1 {
        assert_eq!(
            ledger
                .adjustment_attribution(voucher.number())
                .unwrap()
                .unwrap()
                .effective_year,
            2026
        );
    }
    assert!(ledger.year_vouchers(2027).unwrap().is_empty());
    // The sealed result includes them: 10 000 sales - 2 000 expense - 2 000
    // depreciation - 300 accrual.
    let archive: serde_json::Value = serde_json::from_slice(
        &fs::read(home.path().join("archive/2026/tilinpaatos.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(archive["statements"]["result"], "5700.00");
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
fn a_d7_correction_must_be_sealed_before_approve_adjustments() {
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let mut input = base_input(2026);
    input.corrections = vec![CorrectionInput {
        correction_id: "reclass-1".into(),
        description: "uudelleenluokittelu".into(),
        debit_account: acct("6000"),
        credit_account: acct("7690"),
        amount: money("150.00"),
    }];
    let started = engine
        .start_close_year(&input, &PolicySet::default())
        .unwrap();
    let run_id = started.run.run_id.clone();

    // The statements are computed when ApproveAdjustments is approved, so the
    // approval is blocked while the D7 correction is unsealed.
    let err = engine
        .answer_close_year(&run_id, "ApproveAdjustments", "approve")
        .unwrap_err();
    assert_eq!(err.error_code(), "CLOSE_YEAR_BLOCKED");
    let msg = err.to_string();
    assert!(msg.contains("correction:reclass-1"));
    assert!(
        msg.contains("ApproveAdjustments is blocked"),
        "names the blocked gate: {msg}"
    );
    assert!(!msg.contains("ApproveStatements"), "{msg}");
    assert!(
        msg.contains(&format!(
            "close-year seal-correction --run-id {run_id} --correction <id>"
        )),
        "tells how to seal: {msg}"
    );

    // Seal the correction on its own approval, then the gate is reachable.
    let v = engine
        .seal_close_year_correction(&run_id, "reclass-1")
        .unwrap();
    assert_eq!(eff(&v, "correction:reclass-1"), EffectStatus::Succeeded);
    approve(&engine, &run_id, "ApproveAdjustments");
    approve(&engine, &run_id, "ApproveTaseErittelyt");
    let v2 = approve(&engine, &run_id, "ApproveLiitetiedot");
    assert_eq!(v2.gates.iter().last().unwrap().gate_id, "ApproveStatements");
    let done = approve(&engine, &run_id, "ApproveStatements");
    assert_eq!(done.status, RunStatus::Completed);

    // The correction is attributed to 2026 like the adjustments, and the sealed
    // statements carry it: 150.00 moved from other expenses (7690) to depreciation.
    let ledger = Ledger::open(home.path()).unwrap();
    let correction = ledger
        .year_vouchers(2026)
        .unwrap()
        .into_iter()
        .find(|v| v.description().starts_with("Oikaisu (D7)"))
        .unwrap();
    assert_eq!(correction.posting_date().to_string(), "2027-01-01");
    assert_eq!(
        ledger.adjustment_attributions_for_year(2026).unwrap().len(),
        3
    );
    let archive: serde_json::Value = serde_json::from_slice(
        &fs::read(home.path().join("archive/2026/tilinpaatos.json")).unwrap(),
    )
    .unwrap();
    let statements = &archive["statements"];
    assert_eq!(statements["result"], "5700.00");
    let row = |id: &str| {
        statements["income"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["row_id"] == id)
            .unwrap()["amount"]
            .clone()
    };
    assert_eq!(row("IS-POISTOT"), "2150.00");
    assert_eq!(row("IS-MUUT-KULUT"), "2150.00");
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
    let tampered = raw.replace("\"result\": \"5700.00\"", "\"result\": \"9999.00\"");
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

    // The depreciation voucher was booked exactly once despite two Runs sealing it,
    // and both Runs' attributions agree on it.
    let ledger = Ledger::open(home.path()).unwrap();
    let dep_count = ledger
        .year_vouchers(2026)
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
        "a current close-year run must not migrate to close-period's version"
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
        .period_vouchers(&PeriodId::new_month(2027, 7).unwrap())
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
    assert_eq!(archive["statements"]["result"], "5700.00");
    assert_eq!(
        ledger
            .adjustment_attribution(depreciation.number())
            .unwrap()
            .unwrap()
            .effective_year,
        2027
    );
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

    // The way out the refusal names: the year's depreciation, calculated by hand,
    // given as a correction. It is attributed to the short year like any adjustment.
    let input = CloseYearInput {
        year: 2026,
        assets: vec![],
        accruals: vec![],
        corrections: vec![CorrectionInput {
            correction_id: "poisto-kone-1".into(),
            description: "lyhyen tilikauden poisto".into(),
            debit_account: acct("6000"),
            credit_account: acct("1160"),
            amount: money("1000.00"),
        }],
    };
    let started = engine
        .start_close_year(&input, &PolicySet::default())
        .unwrap();
    let run_id = started.run.run_id;
    engine
        .seal_close_year_correction(&run_id, "poisto-kone-1")
        .unwrap();
    assert_eq!(approve_all(&engine, &run_id).status, RunStatus::Completed);
    let archive: serde_json::Value = serde_json::from_slice(
        &fs::read(home.path().join("archive/2026/tilinpaatos.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(archive["statements"]["result"], "-1000.00");
}

/// Seal all twelve months of `year`.
fn seal_months(ledger: &Ledger, year: i32) {
    for month in 1..=12 {
        let period = PeriodId::new_month(year, month).unwrap();
        let authority = ledger
            .begin_close(&period, &format!("month-{year}-{month:02}"), "test")
            .unwrap();
        ledger.seal_period(&period, &authority, "test").unwrap();
    }
}

fn approve_all(engine: &ProcessEngine, run_id: &str) -> RunView {
    approve(engine, run_id, "ApproveAdjustments");
    approve(engine, run_id, "ApproveTaseErittelyt");
    approve(engine, run_id, "ApproveLiitetiedot");
    approve(engine, run_id, "ApproveStatements")
}

#[test]
fn the_sealed_result_includes_depreciation_and_the_next_year_does_not_count_it_again() {
    // One purchase (98.80 expense, 25.20 input VAT), twelve sealed months, one
    // machine of 2 000.00: the 500.00 depreciation sealed on 2027-01-01 belongs to
    // the result of 2026.
    let home = init_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let d = Date::parse("2026-03-10").unwrap();
    let purchase = VoucherDraft::new(
        d,
        d,
        "purchase".into(),
        vec![
            posting("7690", "98.80", Side::Debit),
            posting("1763", "25.20", Side::Debit),
            posting("1910", "124.00", Side::Credit),
        ],
        None,
    )
    .unwrap();
    ledger
        .seal(
            &purchase,
            &SealRequest {
                natural_key: "purchase".into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
    seal_months(&ledger, 2026);

    let engine = ProcessEngine::open(home.path());
    let input = CloseYearInput {
        year: 2026,
        assets: vec![AssetInput {
            acquisition_cost: money("2000.00"),
            book_value_start: money("2000.00"),
            ..asset()
        }],
        accruals: vec![],
        corrections: vec![],
    };
    let started = engine
        .start_close_year(&input, &PolicySet::default())
        .unwrap();
    assert_eq!(started.depreciation[0].amount.amount_str(), "500.00");
    let done = approve_all(&engine, &started.run.run_id);
    assert_eq!(done.status, RunStatus::Completed);

    let archive: serde_json::Value = serde_json::from_slice(
        &fs::read(home.path().join("archive/2026/tilinpaatos.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(archive["statements"]["result"], "-598.80");
    assert_eq!(archive["statements"]["balanced"], true);

    // The next year opens from the closing year including the depreciation: the
    // accumulated depreciation is in the opening balance, and no 2027 voucher counts
    // it in 2027's result.
    let next = ledger.fiscal_year_data(2027).unwrap();
    assert!(next.vouchers.is_empty());
    let opening = next.opening_balance.unwrap();
    let accumulated = opening
        .postings()
        .iter()
        .find(|p| p.account.as_str() == "1160")
        .unwrap();
    assert_eq!(accumulated.side, Side::Credit);
    assert_eq!(accumulated.amount.amount_str(), "500.00");

    let report = ledger
        .monthly_statements_report(
            &PeriodId::new_month(2027, 1).unwrap(),
            "Esimerkki Oy",
            MonthlyReportOptions::default(),
        )
        .unwrap();
    assert_eq!(report.sections.month_income.result, "0.00");
    assert_eq!(report.sections.year_to_date_income.result, "0.00");
    assert!(report.sources.is_empty());
    assert!(report.sections.balance.balanced);
    // 25.20 VAT receivable - 124.00 cash - 500.00 accumulated depreciation.
    assert_eq!(report.sections.balance.assets_total, "-598.80");
    // The voucher itself stays in the period it was sealed into, which is what the
    // period close and the VAT return read.
    assert_eq!(
        ledger
            .period_vouchers(&PeriodId::new_month(2027, 1).unwrap())
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn an_adjustment_sealed_but_not_attributed_is_attributed_when_the_run_continues() {
    // An interruption between sealing an adjustment and attributing it leaves the
    // voucher in the books without an attribution. Continuing the Run replays the
    // seal and records the attribution before the statements are computed.
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap();
    let ledger = Ledger::open(home.path()).unwrap();
    let draft = depreciation_voucher(
        &started.depreciation[0],
        Date::parse("2026-12-31").unwrap(),
        Date::parse("2027-01-01").unwrap(),
    )
    .unwrap();
    let sealed = ledger
        .seal(
            &draft,
            &SealRequest {
                natural_key: "closeyear:2026:dep:kone-1".into(),
                actor: "process:close-year".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
    assert!(ledger
        .adjustment_attribution(sealed.number)
        .unwrap()
        .is_none());

    let done = approve_all(&engine, &started.run.run_id);
    assert_eq!(done.status, RunStatus::Completed);
    assert_eq!(
        ledger
            .adjustment_attribution(sealed.number)
            .unwrap()
            .unwrap()
            .effective_year,
        2026
    );
    assert_eq!(
        ledger.fiscal_year_view(2026).unwrap().state,
        FiscalYearState::Sealed
    );
    let archive: serde_json::Value = serde_json::from_slice(
        &fs::read(home.path().join("archive/2026/tilinpaatos.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(archive["statements"]["result"], "5700.00");
}

#[test]
fn statements_are_not_bound_while_a_run_voucher_is_unattributed() {
    // A Run whose plan names a voucher that is not attributed to the closing year
    // (here: an accrual added to the persisted plan after the statements were
    // computed) must not bind or seal those statements.
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

    let state_path = home
        .path()
        .join("process/runs")
        .join(&run_id)
        .join("close_year_state.json");
    let mut state: serde_json::Value =
        serde_json::from_slice(&fs::read(&state_path).unwrap()).unwrap();
    let mut late = state["input"]["accruals"][0].clone();
    late["accrual_id"] = "myohainen".into();
    state["input"]["accruals"]
        .as_array_mut()
        .unwrap()
        .push(late);
    fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();

    let err = engine
        .answer_close_year(&run_id, "ApproveStatements", "approve")
        .unwrap_err();
    assert_eq!(err.error_code(), "CLOSE_YEAR_BLOCKED");
    assert!(
        err.to_string().contains("closeyear:2026:acc:myohainen"),
        "{err}"
    );
    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.fiscal_year_view(2026).unwrap().state,
        FiscalYearState::Open
    );
}

#[test]
fn a_resumed_run_does_not_seal_statements_another_run_has_made_stale() {
    // Run A's statements are approved but SealYear fails (a subperiod was reopened).
    // Meanwhile Run B for the same year seals and attributes a correction, so the
    // year's figures no longer match A's approved statements. Resuming A must not
    // seal them.
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let a = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap()
        .run
        .run_id;
    let mut b_input = base_input(2026);
    b_input.corrections = vec![CorrectionInput {
        correction_id: "myohainen".into(),
        description: "myöhäinen oikaisu".into(),
        debit_account: acct("7690"),
        credit_account: acct("1910"),
        amount: money("100.00"),
    }];
    let b = engine
        .start_close_year(&b_input, &PolicySet::default())
        .unwrap()
        .run
        .run_id;
    approve(&engine, &a, "ApproveAdjustments");
    approve(&engine, &a, "ApproveTaseErittelyt");
    approve(&engine, &a, "ApproveLiitetiedot");

    let ledger = Ledger::open(home.path()).unwrap();
    let q4 = PeriodId::new(2026, 4).unwrap();
    let authority = ledger.acquire_authority(&q4, "cp-2026-4").unwrap();
    ledger
        .reopen_period(
            &q4,
            &authority,
            aggountant_ledger::PeriodState::Closing,
            "test",
        )
        .unwrap();
    engine
        .answer_close_year(&a, "ApproveStatements", "approve")
        .unwrap_err();
    ledger.seal_period(&q4, &authority, "test").unwrap();

    engine.seal_close_year_correction(&b, "myohainen").unwrap();
    let err = engine.resume_close_year(&a).unwrap_err();
    assert_eq!(err.error_code(), "JOURNAL_CORRUPT", "{err}");
    assert_eq!(
        ledger.fiscal_year_view(2026).unwrap().state,
        FiscalYearState::Open
    );

    // Run B, whose statements include the correction, completes the close.
    assert_eq!(approve_all(&engine, &b).status, RunStatus::Completed);
    let archive: serde_json::Value = serde_json::from_slice(
        &fs::read(home.path().join("archive/2026/tilinpaatos.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(archive["statements"]["result"], "5600.00");
}

/// The gate's snapshot hash recomputed here, independently of the engine, from the
/// shown content: sha256 over the depreciation JSON then the accrual JSON for the
/// adjustments, and the type's `statements_hash()` for the rest.
fn independent_hash(content: &GateContent) -> String {
    use sha2::{Digest, Sha256};
    match content {
        GateContent::Adjustments(a) => {
            let mut h = Sha256::new();
            h.update(serde_json::to_vec(&a.depreciation).unwrap());
            h.update(serde_json::to_vec(&a.accruals).unwrap());
            hex::encode(h.finalize())
        }
        GateContent::TaseErittelyt(te) => te.statements_hash(),
        GateContent::Liitetiedot(lt) => lt.statements_hash(),
        GateContent::Statements(st) => st.statements_hash(),
    }
}

/// The open gate's id, after checking that the shown content hashes to the gate's
/// `snapshot_hash` as recorded in the Run.
fn assert_shown_content_is_bound(shown: &CloseYearShow) -> String {
    let gate = shown.open_gate.as_ref().expect("a gate is open");
    let recorded = shown
        .run
        .gates
        .iter()
        .find(|g| g.answered.is_none())
        .unwrap();
    assert_eq!(gate.gate_id, recorded.gate_id);
    assert_eq!(gate.snapshot_hash, recorded.snapshot_hash);
    assert_eq!(independent_hash(&gate.content), recorded.snapshot_hash);
    assert_eq!(gate.content.snapshot_hash(), recorded.snapshot_hash);
    gate.gate_id.clone()
}

fn state_path(home: &std::path::Path, run_id: &str) -> std::path::PathBuf {
    home.join("process/runs")
        .join(run_id)
        .join("close_year_state.json")
}

#[test]
fn every_gate_shows_the_content_its_hash_binds_also_after_a_restart() {
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let mut input = base_input(2026);
    input.corrections = vec![CorrectionInput {
        correction_id: "reclass-1".into(),
        description: "uudelleenluokittelu".into(),
        debit_account: acct("6000"),
        credit_account: acct("7690"),
        amount: money("150.00"),
    }];
    let run_id = {
        let engine = ProcessEngine::open(home.path());
        engine
            .start_close_year(&input, &PolicySet::default())
            .unwrap()
            .run
            .run_id
    };

    // Each gate is read by a fresh engine after the previous process has gone, and
    // after a resume, as a user returning in a later session would.
    let gates = [
        "ApproveAdjustments",
        "ApproveTaseErittelyt",
        "ApproveLiitetiedot",
        "ApproveStatements",
    ];
    for expected in gates {
        let engine = ProcessEngine::open(home.path());
        engine.resume_close_year(&run_id).unwrap();
        let shown = engine.show_close_year(&run_id).unwrap();
        assert_eq!(assert_shown_content_is_bound(&shown), expected);
        let gate = shown.open_gate.as_ref().unwrap();
        match (&gate.content, expected) {
            (GateContent::Adjustments(a), "ApproveAdjustments") => {
                assert_eq!(a.depreciation[0].amount.amount_str(), "2000.00");
                assert_eq!(a.accruals[0].accrual_id, "sahko");
                // The correction is listed, outside the hash, with its seal state.
                assert_eq!(gate.corrections.len(), 1);
                assert!(!gate.corrections[0].sealed);
                engine
                    .seal_close_year_correction(&run_id, "reclass-1")
                    .unwrap();
                let again = engine.show_close_year(&run_id).unwrap();
                assert_shown_content_is_bound(&again);
                assert!(again.open_gate.unwrap().corrections[0].sealed);
            }
            (GateContent::TaseErittelyt(te), "ApproveTaseErittelyt") => {
                assert_eq!(te.fiscal_year, 2026);
                assert!(!te.lines.is_empty());
                assert!(gate.corrections.is_empty());
            }
            (GateContent::Liitetiedot(lt), "ApproveLiitetiedot") => {
                assert_eq!(lt.depreciation_notes[0].asset_class, "koneet-ja-kalusto");
            }
            (GateContent::Statements(st), "ApproveStatements") => {
                assert_eq!(st.result, "5700.00");
            }
            (content, gate) => panic!("{gate} showed {content:?}"),
        }
        approve(&engine, &run_id, expected);
    }

    let engine = ProcessEngine::open(home.path());
    let done = engine.show_close_year(&run_id).unwrap();
    assert_eq!(done.run.status, RunStatus::Completed);
    assert!(done.open_gate.is_none());

    // The archive keeps what was approved: the persisted tase-erittelyt and
    // liitetiedot, byte for byte.
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(state_path(home.path(), &run_id)).unwrap()).unwrap();
    let archive: serde_json::Value = serde_json::from_slice(
        &fs::read(home.path().join("archive/2026/tilinpaatos.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(archive["tase_erittelyt"], state["tase_erittelyt"]);
    assert_eq!(archive["liitetiedot"], state["liitetiedot"]);
}

#[test]
fn shown_content_has_a_stable_json_shape() {
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let run_id = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap()
        .run
        .run_id;
    let value = serde_json::to_value(engine.show_close_year(&run_id).unwrap()).unwrap();
    assert_eq!(value["run_id"], run_id.as_str());
    let gate = &value["open_gate"];
    assert_eq!(gate["gate_id"], "ApproveAdjustments");
    assert_eq!(gate["kind"], "adjustments");
    assert_eq!(
        gate["content"]["depreciation"][0]["amount"]["amount"],
        "2000.00"
    );
    assert_eq!(gate["content"]["accruals"][0]["accrual_id"], "sahko");
    assert!(gate.get("corrections").is_none());

    approve(&engine, &run_id, "ApproveAdjustments");
    let value = serde_json::to_value(engine.show_close_year(&run_id).unwrap()).unwrap();
    assert_eq!(value["open_gate"]["kind"], "tase_erittelyt");
    assert_eq!(value["open_gate"]["content"]["fiscal_year"], 2026);
}

#[test]
fn changed_gate_content_is_neither_shown_nor_approved() {
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let run_id = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap()
        .run
        .run_id;
    approve(&engine, &run_id, "ApproveAdjustments");

    // Change the persisted tase-erittelyt behind the open gate.
    let path = state_path(home.path(), &run_id);
    let mut state: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    state["tase_erittelyt"]["fiscal_year"] = serde_json::json!(2025);
    fs::write(&path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();

    let err = engine.show_close_year(&run_id).unwrap_err();
    assert_eq!(err.error_code(), "JOURNAL_CORRUPT");
    let err = engine
        .answer_close_year(&run_id, "ApproveTaseErittelyt", "approve")
        .unwrap_err();
    assert_eq!(err.error_code(), "JOURNAL_CORRUPT");
    // The gate stays open and can still be rejected.
    let v = engine
        .answer_close_year(&run_id, "ApproveTaseErittelyt", "reject")
        .unwrap();
    assert_eq!(v.status, RunStatus::Rejected);
}

#[test]
fn a_run_started_before_content_was_persisted_shows_a_checked_rebuild() {
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let run_id = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap()
        .run
        .run_id;
    approve(&engine, &run_id, "ApproveAdjustments");

    // A state file written by an earlier version has none of the persisted content.
    let path = state_path(home.path(), &run_id);
    let strip = |path: &std::path::Path| {
        let mut state: serde_json::Value =
            serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let map = state.as_object_mut().unwrap();
        map.remove("depreciation");
        map.remove("tase_erittelyt");
        map.remove("liitetiedot");
        fs::write(path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
    };
    strip(&path);
    let shown = engine.show_close_year(&run_id).unwrap();
    assert_eq!(
        assert_shown_content_is_bound(&shown),
        "ApproveTaseErittelyt"
    );

    // A rebuild that no longer matches the bound hash is refused. (Here the run's
    // policy set changes, which the rebuilt provenance carries; in use it would be
    // the books changing after the gate opened.)
    let mut state: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    state["policy_set"] = serde_json::json!("other-policy");
    fs::write(&path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();
    let err = engine.show_close_year(&run_id).unwrap_err();
    assert_eq!(err.error_code(), "CLOSE_YEAR_BLOCKED");
}

#[test]
fn approved_breakdowns_made_stale_by_another_run_are_not_sealed() {
    // Run A has approved its tase-erittelyt. Run B for the same year then seals a
    // correction between two cash accounts on the same balance-sheet row, so A's
    // statements are unchanged but its approved breakdown no longer matches the
    // books. A must not seal the year with that breakdown.
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let a = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap()
        .run
        .run_id;
    let mut b_input = base_input(2026);
    b_input.corrections = vec![CorrectionInput {
        correction_id: "siirto".into(),
        description: "siirto pankkitilien välillä".into(),
        debit_account: acct("1920"),
        credit_account: acct("1910"),
        amount: money("100.00"),
    }];
    let b = engine
        .start_close_year(&b_input, &PolicySet::default())
        .unwrap()
        .run
        .run_id;
    approve(&engine, &a, "ApproveAdjustments");
    approve(&engine, &a, "ApproveTaseErittelyt");
    approve(&engine, &a, "ApproveLiitetiedot");

    engine.seal_close_year_correction(&b, "siirto").unwrap();
    let err = engine
        .answer_close_year(&a, "ApproveStatements", "approve")
        .unwrap_err();
    assert_eq!(err.error_code(), "JOURNAL_CORRUPT", "{err}");
    assert!(err.to_string().contains("ApproveTaseErittelyt"), "{err}");
    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.fiscal_year_view(2026).unwrap().state,
        FiscalYearState::Open
    );
}

#[test]
fn approved_breakdowns_changed_in_the_run_state_are_not_sealed() {
    // The archive writes the persisted tase-erittelyt, so a state file changed after
    // the approval must not reach it even though the books still match.
    let home = init_home();
    setup_sealed_year(home.path(), 2026);
    let engine = ProcessEngine::open(home.path());
    let run_id = engine
        .start_close_year(&base_input(2026), &PolicySet::default())
        .unwrap()
        .run
        .run_id;
    approve(&engine, &run_id, "ApproveAdjustments");
    approve(&engine, &run_id, "ApproveTaseErittelyt");
    approve(&engine, &run_id, "ApproveLiitetiedot");

    let path = state_path(home.path(), &run_id);
    let mut state: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    state["tase_erittelyt"]["fiscal_year"] = serde_json::json!(2025);
    fs::write(&path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();

    let err = engine
        .answer_close_year(&run_id, "ApproveStatements", "approve")
        .unwrap_err();
    assert_eq!(err.error_code(), "JOURNAL_CORRUPT", "{err}");
    assert!(err.to_string().contains("persisted"), "{err}");
    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.fiscal_year_view(2026).unwrap().state,
        FiscalYearState::Open
    );
    assert!(!home.path().join("archive/2026/tilinpaatos.json").exists());
}
