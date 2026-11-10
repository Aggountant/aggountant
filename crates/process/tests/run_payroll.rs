//! RF4 `run-payroll` Run tests — the **M4 gate**: partial-failure ergonomics at
//! scale. Many employees run through one Run; one employee's payment times out while
//! the rest succeed, the Run parks `NeedsAttention` on exactly that employee, and a
//! deliberate `handle_payment_failure` recovers it (no double payment). Also covers
//! the HETU-preflight → `resolve_errors` path and the PII-isolation invariant (no
//! HETU/IBAN in the process store).

use serde_json::{json, Value};

use aggountant_broker::{
    Broker, PankkiMaksuAdapter, RawResponse, Registry, ScriptedTransport, Transport,
    TransportError, TulorekisteriAdapter, Vault, SVC_PANKKI_MAKSU, SVC_TULOREKISTERI,
};
use aggountant_domain::{PayrollIntent, PolicySet, TaxCard};
use aggountant_ledger::{EmployeeState, Ledger};
use aggountant_process::{EffectStatus, ProcessEngine, RunStatus};

type Steps = Vec<Result<RawResponse, TransportError>>;

fn ok(status: u16, body: Value) -> Result<RawResponse, TransportError> {
    Ok(RawResponse::new(status, body))
}

/// A valid test HETU (mod-31 checksum ok).
const HETU_OK: &str = "131052-308T";

fn employee(id: &str) -> EmployeeState {
    EmployeeState {
        employee_id: id.into(),
        name: format!("Test {id}"),
        hetu: HETU_OK.into(),
        iban: "FI2112345600000785".into(),
        tax_card: TaxCard {
            withholding_pct: "20.00".into(),
        },
    }
}

/// Init a home with `n` employees (emp-1..emp-n), all with valid master data.
fn home_with_employees(n: usize) -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    for i in 1..=n {
        ledger
            .upsert_employee(&employee(&format!("emp-{i}")))
            .unwrap();
    }
    home
}

fn broker_with(home: &std::path::Path, pankki: Steps, tulorekisteri: Steps) -> Broker {
    let mut vault = Vault::new();
    for s in [SVC_PANKKI_MAKSU, SVC_TULOREKISTERI] {
        vault.set(s, "secret");
    }
    let mut reg = Registry::new();
    reg.register(Box::new(PankkiMaksuAdapter::new(
        Box::new(ScriptedTransport::new(pankki)) as Box<dyn Transport>,
    )));
    reg.register(Box::new(TulorekisteriAdapter::new(
        Box::new(ScriptedTransport::new(tulorekisteri)) as Box<dyn Transport>,
    )));
    Broker::new(home, vault, reg)
}

fn intent(ids: &[&str]) -> PayrollIntent {
    intent_dated(ids, "2026-02-27")
}

fn intent_dated(ids: &[&str], payment_date: &str) -> PayrollIntent {
    let employees: Vec<Value> = ids
        .iter()
        .map(|id| json!({"employee_id": id, "gross": {"amount": "3000.00", "currency": "EUR"}, "tulolaji": "201"}))
        .collect();
    serde_json::from_value(json!({
        "payment_date": payment_date,
        "employees": employees,
    }))
    .unwrap()
}

fn effect(run: &aggountant_process::RunView, key: &str) -> EffectStatus {
    run.effects
        .iter()
        .find(|e| e.key == key)
        .unwrap_or_else(|| panic!("no effect {key}"))
        .status
}

// --- Happy path at scale ----------------------------------------------------

#[test]
fn happy_path_pays_reports_and_files_the_monthly_report() {
    let home = home_with_employees(3);
    let engine = ProcessEngine::open(home.path());
    // 3 payments + 3 palkkatieto + 1 erillisilmoitus, all 200.
    let broker = broker_with(
        home.path(),
        vec![
            ok(200, json!({"receipt": "P1"})),
            ok(200, json!({"receipt": "P2"})),
            ok(200, json!({"receipt": "P3"})),
        ],
        vec![
            ok(200, json!({"receipt": "R1"})),
            ok(200, json!({"receipt": "R2"})),
            ok(200, json!({"receipt": "R3"})),
            ok(200, json!({"receipt": "ERIL"})),
        ],
    );

    let start = engine
        .start_run_payroll(&intent(&["emp-1", "emp-2", "emp-3"]), &PolicySet::default())
        .unwrap();
    assert_eq!(start.status, RunStatus::AwaitingGate);
    assert_eq!(start.gates[0].gate_id, "ApproveRun");
    let run_id = start.run_id.clone();

    let done = engine
        .answer_run_payroll(&run_id, "approve", &broker)
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    assert_eq!(effect(&done, "erillisilmoitus"), EffectStatus::Succeeded);

    // Three payroll vouchers sealed, the chain verifies, YTD accumulated.
    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(ledger.verify().unwrap().voucher_count, 3);
    // YTD is year-scoped to the payment date's tax year (2026 here).
    assert_eq!(
        ledger.ytd_gross("emp-1", 2026).unwrap().amount_str(),
        "3000.00"
    );
}

// --- Year boundary: YTD lands in the payment date's tax year ----------------

#[test]
fn ytd_is_attributed_to_the_payment_dates_tax_year() {
    // A run paid on the last day of 2026 must accumulate into tax year 2026 — not
    // 2027 (the following year) and not any prior year. This locks in the process-layer
    // `PeriodId::containing(payment_date).year()` derivation across a year boundary.
    let home = home_with_employees(1);
    let engine = ProcessEngine::open(home.path());
    let broker = broker_with(
        home.path(),
        vec![ok(200, json!({"receipt": "P1"}))],
        vec![
            ok(200, json!({"receipt": "R1"})),
            ok(200, json!({"receipt": "ERIL"})),
        ],
    );
    let run_id = engine
        .start_run_payroll(
            &intent_dated(&["emp-1"], "2026-12-31"),
            &PolicySet::default(),
        )
        .unwrap()
        .run_id;
    let done = engine
        .answer_run_payroll(&run_id, "approve", &broker)
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);

    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.ytd_gross("emp-1", 2026).unwrap().amount_str(),
        "3000.00"
    );
    assert_eq!(
        ledger.ytd_gross("emp-1", 2027).unwrap().amount_str(),
        "0.00"
    );
    assert_eq!(
        ledger.ytd_gross("emp-1", 2025).unwrap().amount_str(),
        "0.00"
    );
}

// --- M4: one employee's payment times out; the rest succeed -----------------

#[test]
fn partial_payment_failure_at_scale_parks_one_and_recovers() {
    let home = home_with_employees(3);
    let engine = ProcessEngine::open(home.path());
    // emp-2's payment times out (Unknown); emp-1/emp-3 pay + report fine. On the
    // deliberate retry, emp-2's payment meets a 409 the adapter recovers (no double
    // payment), then its report + the monthly report complete.
    let broker = broker_with(
        home.path(),
        vec![
            ok(200, json!({"receipt": "P1"})), // emp-1 pay
            Err(TransportError::Timeout),      // emp-2 pay → Unknown
            ok(200, json!({"receipt": "P3"})), // emp-3 pay
            ok(
                409,
                json!({"receipt": "P2-DUP", "idempotency_key": "pay:emp-2:2026-02-27"}),
            ), // emp-2 retry → recovered
        ],
        vec![
            ok(200, json!({"receipt": "R1"})),   // emp-1 report
            ok(200, json!({"receipt": "R3"})),   // emp-3 report
            ok(200, json!({"receipt": "R2"})),   // emp-2 report (after retry)
            ok(200, json!({"receipt": "ERIL"})), // erillisilmoitus
        ],
    );

    let run_id = engine
        .start_run_payroll(&intent(&["emp-1", "emp-2", "emp-3"]), &PolicySet::default())
        .unwrap()
        .run_id;

    let after = engine
        .answer_run_payroll(&run_id, "approve", &broker)
        .unwrap();
    // The Run parks NeedsAttention on emp-2 only; emp-1 and emp-3 are fully done.
    assert_eq!(after.status, RunStatus::NeedsAttention);
    assert_eq!(effect(&after, "pay:emp-2"), EffectStatus::Rejected);
    assert_eq!(effect(&after, "pay:emp-1"), EffectStatus::Succeeded);
    assert_eq!(effect(&after, "report:emp-1"), EffectStatus::Succeeded);
    assert_eq!(effect(&after, "pay:emp-3"), EffectStatus::Succeeded);
    assert_eq!(effect(&after, "report:emp-3"), EffectStatus::Succeeded);
    // emp-2's report never fired (its payment never landed).
    assert_eq!(effect(&after, "report:emp-2"), EffectStatus::Planned);
    // The monthly report waits for the whole batch.
    assert_eq!(effect(&after, "erillisilmoitus"), EffectStatus::Planned);

    // Deliberate, per-employee recovery.
    let recovered = engine
        .handle_payment_failure(&run_id, "emp-2", &broker)
        .unwrap();
    assert_eq!(recovered.status, RunStatus::Completed);
    assert_eq!(effect(&recovered, "pay:emp-2"), EffectStatus::Succeeded);
    assert_eq!(
        effect(&recovered, "erillisilmoitus"),
        EffectStatus::Succeeded
    );

    // Exactly three vouchers — emp-2 was not double-sealed by the retry.
    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(ledger.verify().unwrap().voucher_count, 3);
    // YTD counted once per employee despite the retry.
    assert_eq!(
        ledger.ytd_gross("emp-2", 2026).unwrap().amount_str(),
        "3000.00"
    );
}

// --- Recovery: resume re-drives a parked employee + monthly report ----------

#[test]
fn resume_redrives_a_parked_employee_and_completes() {
    let home = home_with_employees(2);
    let engine = ProcessEngine::open(home.path());
    let approve_broker = broker_with(
        home.path(),
        vec![
            ok(200, json!({"receipt": "P1"})),
            Err(TransportError::Timeout), // emp-2 pay → parked
        ],
        vec![ok(200, json!({"receipt": "R1"}))],
    );
    let run_id = engine
        .start_run_payroll(&intent(&["emp-1", "emp-2"]), &PolicySet::default())
        .unwrap()
        .run_id;
    let after = engine
        .answer_run_payroll(&run_id, "approve", &approve_broker)
        .unwrap();
    assert_eq!(after.status, RunStatus::NeedsAttention);

    // A fresh broker (as after a restart) with the recovery responses; `resume`
    // reconciles the whole run without the operator naming the employee.
    let resume_broker = broker_with(
        home.path(),
        vec![ok(
            409,
            json!({"receipt": "P2-DUP", "idempotency_key": "pay:emp-2:2026-02-27"}),
        )],
        vec![
            ok(200, json!({"receipt": "R2"})),
            ok(200, json!({"receipt": "ERIL"})),
        ],
    );
    let resumed = engine.resume_run_payroll(&run_id, &resume_broker).unwrap();
    assert_eq!(resumed.status, RunStatus::Completed);
    assert_eq!(effect(&resumed, "pay:emp-2"), EffectStatus::Succeeded);
    assert_eq!(effect(&resumed, "erillisilmoitus"), EffectStatus::Succeeded);
}

#[test]
fn duplicate_employee_id_is_rejected() {
    let home = home_with_employees(1);
    let engine = ProcessEngine::open(home.path());
    let err = engine
        .start_run_payroll(&intent(&["emp-1", "emp-1"]), &PolicySet::default())
        .unwrap_err();
    assert_eq!(err.error_code(), "INVALID_ANSWER");
}

// --- M4: HETU-preflight failure parks before the gate; resolve_errors fixes --

#[test]
fn bad_hetu_parks_before_the_gate_then_resolves() {
    let home = home_with_employees(2);
    let ledger = Ledger::open(home.path()).unwrap();
    // Corrupt emp-2's HETU so the preflight rejects it.
    let mut bad = employee("emp-2");
    bad.hetu = "131052-308U".into(); // wrong checksum
    ledger.upsert_employee(&bad).unwrap();

    let engine = ProcessEngine::open(home.path());
    let start = engine
        .start_run_payroll(&intent(&["emp-1", "emp-2"]), &PolicySet::default())
        .unwrap();
    // emp-2's compute is parked; no gate opens while a compute is unresolved.
    assert_eq!(start.status, RunStatus::NeedsAttention);
    assert_eq!(effect(&start, "compute:emp-2"), EffectStatus::Rejected);
    assert!(start.gates.is_empty(), "no gate while a compute is parked");
    let run_id = start.run_id.clone();

    // Fix the master data, then resolve.
    ledger.upsert_employee(&employee("emp-2")).unwrap();
    let resolved = engine.resolve_errors(&run_id, "emp-2").unwrap();
    assert_eq!(effect(&resolved, "compute:emp-2"), EffectStatus::Succeeded);
    assert_eq!(resolved.status, RunStatus::AwaitingGate);
    assert_eq!(resolved.gates[0].gate_id, "ApproveRun");
}

// --- PII isolation: no HETU/IBAN in the process store -----------------------

#[test]
fn process_store_holds_no_pii() {
    let home = home_with_employees(2);
    let engine = ProcessEngine::open(home.path());
    let run_id = engine
        .start_run_payroll(&intent(&["emp-1", "emp-2"]), &PolicySet::default())
        .unwrap()
        .run_id;

    // Walk every file under the run directory; none may contain the HETU or IBAN.
    let run_dir = home.path().join("process").join("runs").join(&run_id);
    let mut checked = 0;
    for entry in walk(&run_dir) {
        let bytes = std::fs::read(&entry).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains(HETU_OK),
            "HETU leaked into the process store at {}",
            entry.display()
        );
        assert!(
            !text.contains("FI2112345600000785"),
            "IBAN leaked into the process store at {}",
            entry.display()
        );
        checked += 1;
    }
    assert!(checked > 0, "expected some run files to inspect");
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}
