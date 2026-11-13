//! S2 process-engine tests: the derived-Run-state bet, the gate, resume, the
//! crash-injection reconcile gate (invariant 4), and workflow versioning.

use std::fs;

use aggountant_domain::PolicySet;
use aggountant_ledger::{Ledger, PeriodState, SealRequest};
use aggountant_model::{AccountId, Currency, Date, Money, PeriodId, Posting, Side, VoucherDraft};
use aggountant_process::{current_workflow_version, EffectStatus, ProcessEngine, RunStatus, Seam};

fn q1() -> PeriodId {
    PeriodId::parse("2026-Q1").unwrap()
}

/// Init a fiscal home and return its path (owned by the tempdir).
fn init_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    home
}

fn effect_status(view: &aggountant_process::RunView, key: &str) -> EffectStatus {
    view.effects.iter().find(|e| e.key == key).unwrap().status
}

/// Seal one balanced voucher dated `date` through the ordinary (authority-less)
/// path, as `capture-receipt record` would.
fn seal_voucher(
    ledger: &Ledger,
    date: &str,
    key: &str,
) -> Result<(), aggountant_ledger::LedgerError> {
    let eur = Currency::new("EUR").unwrap();
    let posting = |acc: &str, side| Posting {
        account: AccountId::new(acc).unwrap(),
        amount: Money::parse("12.40", eur.clone()).unwrap(),
        side,
        vat_code: None,
    };
    let d = Date::parse(date).unwrap();
    let draft = VoucherDraft::new(
        d,
        d,
        "Esimerkki Oy hosting".into(),
        vec![posting("7690", Side::Debit), posting("1910", Side::Credit)],
        None,
    )
    .unwrap();
    let req = SealRequest {
        natural_key: key.into(),
        actor: "test".into(),
        policy_set: PolicySet::default(),
    };
    ledger.seal(&draft, &req).map(|_| ())
}

#[test]
fn happy_path_open_closing_gate_sealed() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());

    let started = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();
    // Parked on the gate; transition Succeeded, seal still Planned.
    assert_eq!(started.status, RunStatus::AwaitingGate);
    assert_eq!(
        effect_status(&started, "transition"),
        EffectStatus::Succeeded
    );
    assert_eq!(effect_status(&started, "seal"), EffectStatus::Planned);
    assert_eq!(started.gates.len(), 1);
    assert_eq!(started.gates[0].gate_id, "ApproveClose");
    assert!(started.gates[0].answered.is_none());

    // Fiscal is authoritative: the period is already Closing.
    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.period_view(&q1()).unwrap().state,
        PeriodState::Closing
    );

    let done = engine
        .answer(&started.run_id, "ApproveClose", "approve")
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    assert_eq!(effect_status(&done, "seal"), EffectStatus::Succeeded);
    assert_eq!(
        ledger.period_view(&q1()).unwrap().state,
        PeriodState::Sealed
    );
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn run_status_is_derived_not_stored() {
    // `show` must reconstruct status by folding effects — there is no status
    // field in the journal to read. We assert the derivation transitions purely
    // as a function of the recorded effect states.
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();

    let shown = engine.show(&started.run_id).unwrap();
    assert_eq!(shown.status, RunStatus::AwaitingGate);
    // Sanity: the derived status is a pure function of effects+gates.
    assert_eq!(shown.effects.len(), 2);
    assert!(shown.gates[0].answered.is_none());

    engine
        .answer(&started.run_id, "ApproveClose", "approve")
        .unwrap();
    assert_eq!(
        engine.show(&started.run_id).unwrap().status,
        RunStatus::Completed
    );
}

#[test]
fn reject_gate_returns_the_period_to_open_without_sealing() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();

    let after = engine
        .answer(&started.run_id, "ApproveClose", "reject")
        .unwrap();
    assert_eq!(after.status, RunStatus::Rejected);
    assert_eq!(effect_status(&after, "seal"), EffectStatus::Planned);
    assert_eq!(
        effect_status(&after, "reject-close"),
        EffectStatus::Succeeded
    );
    // The rejection took Closing→Open under this run's authority.
    let ledger = Ledger::open(home.path()).unwrap();
    let view = ledger.period_view(&q1()).unwrap();
    assert_eq!(view.state, PeriodState::Open);
    assert_eq!(view.owning_run_id.as_deref(), Some(started.run_id.as_str()));
    // The answered gate cannot be answered again.
    let err = engine
        .answer(&started.run_id, "ApproveClose", "approve")
        .unwrap_err();
    assert_eq!(err.error_code(), "GATE_NOT_OPEN");
}

#[test]
fn reject_record_close_seal_then_the_home_verifies() {
    let month = PeriodId::parse("2026-05").unwrap();
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let ledger = Ledger::open(home.path()).unwrap();
    seal_voucher(&ledger, "2026-05-04", "k:before").unwrap();

    let first = engine
        .start_close_period(&month, &PolicySet::default())
        .unwrap();
    let err = seal_voucher(&ledger, "2026-05-20", "k:missing").unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_CLOSING");
    engine
        .answer(&first.run_id, "ApproveClose", "reject")
        .unwrap();

    // The month takes the missing voucher, and a new close seals it.
    seal_voucher(&ledger, "2026-05-20", "k:missing").unwrap();
    let second = engine
        .start_close_period(&month, &PolicySet::default())
        .unwrap();
    assert_ne!(second.run_id, first.run_id);
    let done = engine
        .answer(&second.run_id, "ApproveClose", "approve")
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    let view = ledger.period_view(&month).unwrap();
    assert_eq!(view.state, PeriodState::Sealed);
    assert_eq!(view.owning_run_id.as_deref(), Some(second.run_id.as_str()));
    assert_eq!(ledger.period_vouchers(&month).unwrap().len(), 2);
    let report = ledger.verify().unwrap();
    assert!(report.ok, "{report:?}");
    // The rejected run stays rejected and does not touch the new close.
    let resumed = engine.resume(&first.run_id).unwrap();
    assert_eq!(resumed.status, RunStatus::Rejected);
    assert_eq!(
        ledger.period_view(&month).unwrap().state,
        PeriodState::Sealed
    );
}

#[test]
fn resume_returns_the_period_after_a_crash_between_reject_and_reopen() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();
    let run_id = started.run_id;

    // Crash after GateAnswered(reject), before RejectClose ran. The journal now
    // looks like a Run an older binary rejected and left Closing.
    let crashed = engine
        .answer_seam(
            &run_id,
            "ApproveClose",
            "reject",
            Seam::CrashAfterGateAnswer,
        )
        .unwrap();
    assert_eq!(crashed.status, RunStatus::Rejected);
    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.period_view(&q1()).unwrap().state,
        PeriodState::Closing
    );

    let resumed = engine.resume(&run_id).unwrap();
    assert_eq!(resumed.status, RunStatus::Rejected);
    assert_eq!(
        effect_status(&resumed, "reject-close"),
        EffectStatus::Succeeded
    );
    assert_eq!(ledger.period_view(&q1()).unwrap().state, PeriodState::Open);
    // A second resume changes nothing.
    let events = engine.journal(&run_id).unwrap().len();
    let seq = ledger.period_view(&q1()).unwrap().head_transition_seq;
    engine.resume(&run_id).unwrap();
    assert_eq!(engine.journal(&run_id).unwrap().len(), events);
    assert_eq!(ledger.period_view(&q1()).unwrap().head_transition_seq, seq);
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn resume_reconciles_a_reopen_that_committed_before_the_crash() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();
    let run_id = started.run_id;
    engine
        .answer_seam(
            &run_id,
            "ApproveClose",
            "reject",
            Seam::CrashAfterGateAnswer,
        )
        .unwrap();
    // The fiscal Closing→Open COMMIT landed, the journal never recorded it.
    let ledger = Ledger::open(home.path()).unwrap();
    let auth = ledger.acquire_authority(&q1(), &run_id).unwrap();
    ledger.reject_close(&q1(), &auth, "test").unwrap();
    let seq = ledger.period_view(&q1()).unwrap().head_transition_seq;

    // Resume adopts fiscal instead of taking the edge a second time.
    let resumed = engine.resume(&run_id).unwrap();
    assert_eq!(resumed.status, RunStatus::Rejected);
    assert_eq!(
        effect_status(&resumed, "reject-close"),
        EffectStatus::Succeeded
    );
    assert!(engine
        .journal(&run_id)
        .unwrap()
        .iter()
        .any(|e| e.event_type == "effect_reconciled"));
    let view = ledger.period_view(&q1()).unwrap();
    assert_eq!(view.state, PeriodState::Open);
    assert_eq!(view.head_transition_seq, seq);
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn invalid_gate_answer_is_rejected() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();
    let err = engine
        .answer(&started.run_id, "ApproveClose", "maybe")
        .unwrap_err();
    assert_eq!(err.error_code(), "INVALID_ANSWER");
}

// --- Invariant 4: resume reads authoritative fiscal PeriodState (fiscal wins) --

#[test]
fn resume_reconciles_crash_between_fiscal_commit_and_effectstate() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());

    // Crash the run right after the fiscal COMMIT of TransitionPeriod, before the
    // process EffectState (and the gate) are written.
    let crashed = engine
        .start_close_period_seam(&q1(), &PolicySet::default(), Seam::CrashAfterFiscalCommit)
        .unwrap();
    let run_id = crashed.run_id.clone();

    // Process side: the transition is stuck Executing, no gate opened.
    assert_eq!(
        effect_status(&crashed, "transition"),
        EffectStatus::Executing
    );
    assert!(crashed.gates.is_empty());
    // Fiscal side (authoritative): the period IS Closing, owned by the run.
    let ledger = Ledger::open(home.path()).unwrap();
    let fiscal = ledger.period_view(&q1()).unwrap();
    assert_eq!(fiscal.state, PeriodState::Closing);
    assert_eq!(fiscal.owning_run_id.as_deref(), Some(run_id.as_str()));

    // Resume (fresh engine = fresh process) reconciles the process EffectState to
    // fiscal — never the reverse — and converges to the parked-at-gate state.
    let resumed = ProcessEngine::open(home.path()).resume(&run_id).unwrap();
    assert_eq!(
        effect_status(&resumed, "transition"),
        EffectStatus::Succeeded
    );
    assert_eq!(resumed.status, RunStatus::AwaitingGate);
    assert_eq!(resumed.gates.len(), 1);

    // And the reconciled run can be driven to a clean seal.
    let done = ProcessEngine::open(home.path())
        .answer(&run_id, "ApproveClose", "approve")
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    assert_eq!(
        ledger.period_view(&q1()).unwrap().state,
        PeriodState::Sealed
    );
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn resume_completes_an_approved_but_crashed_seal() {
    // The wedge the review found: approve the gate, crash before SealPeriod. The
    // gate is answered, so the "open the gate" converge branch is skipped — resume
    // must instead complete the seal, or the period is stuck Closing forever.
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();
    let run_id = started.run_id;

    let crashed = engine
        .answer_seam(
            &run_id,
            "ApproveClose",
            "approve",
            Seam::CrashAfterGateAnswer,
        )
        .unwrap();
    assert_eq!(effect_status(&crashed, "seal"), EffectStatus::Planned);
    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.period_view(&q1()).unwrap().state,
        PeriodState::Closing
    );

    // Resume completes the approved-but-unsealed close.
    let resumed = ProcessEngine::open(home.path()).resume(&run_id).unwrap();
    assert_eq!(resumed.status, RunStatus::Completed);
    assert_eq!(effect_status(&resumed, "seal"), EffectStatus::Succeeded);
    assert_eq!(
        ledger.period_view(&q1()).unwrap().state,
        PeriodState::Sealed
    );
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn resume_is_idempotent_after_convergence() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let crashed = engine
        .start_close_period_seam(&q1(), &PolicySet::default(), Seam::CrashAfterFiscalCommit)
        .unwrap();
    let run_id = crashed.run_id;

    let first = engine.resume(&run_id).unwrap();
    let second = engine.resume(&run_id).unwrap();
    // A second resume adds no new reconciliation and keeps the same derived state.
    assert_eq!(first.status, second.status);
    assert_eq!(second.status, RunStatus::AwaitingGate);
    assert_eq!(second.gates.len(), 1, "no duplicate gate opened");
    // Exactly one reconcile event for the transition.
    let reconciles = engine
        .journal(&run_id)
        .unwrap()
        .iter()
        .filter(|e| e.event_type == "effect_reconciled")
        .count();
    assert_eq!(reconciles, 1);
}

#[test]
fn a_new_close_after_a_rejection_resumes_a_crash_before_its_fiscal_commit() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let first = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();
    engine
        .answer(&first.run_id, "ApproveClose", "reject")
        .unwrap();

    // The second close crashes before Open→Closing commits. The Open row still
    // names the rejecting run, which must not stop the retry.
    let crashed = engine
        .start_close_period_seam(&q1(), &PolicySet::default(), Seam::CrashBeforeFiscalCommit)
        .unwrap();
    let ledger = Ledger::open(home.path()).unwrap();
    assert_eq!(
        ledger.period_view(&q1()).unwrap().owning_run_id.as_deref(),
        Some(first.run_id.as_str())
    );

    // The retry is a start, so an open obligation blocks it like a fresh start.
    let obligation = engine
        .obligations()
        .create(&q1(), "missing", "missing receipt", "2026-04-30", "capture")
        .unwrap();
    let err = engine.resume(&crashed.run_id).unwrap_err();
    assert_eq!(err.error_code(), "OPEN_OBLIGATION");
    assert_eq!(ledger.period_view(&q1()).unwrap().state, PeriodState::Open);
    engine.obligations().resolve(&obligation.id).unwrap();

    let resumed = engine.resume(&crashed.run_id).unwrap();
    assert_eq!(resumed.status, RunStatus::AwaitingGate);
    let view = ledger.period_view(&q1()).unwrap();
    assert_eq!(view.state, PeriodState::Closing);
    assert_eq!(view.owning_run_id.as_deref(), Some(crashed.run_id.as_str()));
    let done = engine
        .answer(&crashed.run_id, "ApproveClose", "approve")
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    assert!(ledger.verify().unwrap().ok);
}

// --- Workflow versioning ---------------------------------------------------

#[test]
fn resume_refuses_a_version_mismatch_until_migrated() {
    let home = init_home();
    // Start under the real version.
    let v1 = current_workflow_version();
    let engine_v1 = ProcessEngine::open_with_version(home.path(), v1);
    let started = engine_v1
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();
    let run_id = started.run_id;

    // A newer engine version refuses to resume.
    let engine_v2 = ProcessEngine::open_with_version(home.path(), "wf-deadbeefdeadbeef");
    let err = engine_v2.resume(&run_id).unwrap_err();
    assert_eq!(err.error_code(), "WORKFLOW_VERSION_MISMATCH");

    // Migrate, then the newer engine resumes cleanly.
    engine_v2.migrate(&run_id).unwrap();
    let resumed = engine_v2.resume(&run_id).unwrap();
    assert_eq!(resumed.workflow_version, "wf-deadbeefdeadbeef");
    assert_eq!(resumed.status, RunStatus::AwaitingGate);
}

// --- Lease + journal integrity + not-found ---------------------------------

#[test]
fn a_held_lease_blocks_another_driver() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();
    let run_id = started.run_id;

    // Plant a fresh (non-expired) lease as if another driver held it.
    let lease = home
        .path()
        .join("process")
        .join("runs")
        .join(&run_id)
        .join("lease");
    let future = (chrono::Utc::now() + chrono::Duration::seconds(120)).to_rfc3339();
    fs::write(&lease, format!("{future}\n")).unwrap();

    let err = engine.resume(&run_id).unwrap_err();
    assert_eq!(err.error_code(), "RUN_LEASED");

    // A read (`show`) needs no lease and still works.
    assert!(engine.show(&run_id).is_ok());
}

#[test]
fn a_stale_lease_is_stealable() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();
    let run_id = started.run_id;

    // A lease from a crashed driver, already expired.
    let lease = home
        .path()
        .join("process")
        .join("runs")
        .join(&run_id)
        .join("lease");
    let past = (chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339();
    fs::write(&lease, format!("{past}\n")).unwrap();

    // resume steals it and proceeds.
    assert!(engine.resume(&run_id).is_ok());
}

#[test]
fn unknown_run_is_not_found() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    assert_eq!(
        engine.show("cp-does-not-exist").unwrap_err().error_code(),
        "RUN_NOT_FOUND"
    );
    assert_eq!(
        engine.resume("cp-does-not-exist").unwrap_err().error_code(),
        "RUN_NOT_FOUND"
    );
}

#[test]
fn tampered_journal_is_detected() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let started = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();
    let run_id = started.run_id;

    // Corrupt the first event file's payload without fixing its hash.
    let ev = home
        .path()
        .join("process")
        .join("runs")
        .join(&run_id)
        .join("events")
        .join("000001.json");
    let body = fs::read_to_string(&ev).unwrap();
    let tampered = body.replace("close-period", "tampered-workflow");
    assert_ne!(
        body, tampered,
        "the substitution must actually change bytes"
    );
    fs::write(&ev, tampered).unwrap();

    let err = engine.show(&run_id).unwrap_err();
    assert_eq!(err.error_code(), "JOURNAL_CORRUPT");
}

#[test]
fn monthly_close_is_independent_and_overlap_obligations_block() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    let july = PeriodId::parse("2026-07").unwrap();
    let quarter = PeriodId::parse("2026-Q3").unwrap();
    let obligation = engine
        .obligations()
        .create(
            &quarter,
            "missing",
            "missing receipt",
            "2026-08-31",
            "capture",
        )
        .unwrap();
    let error = engine
        .start_close_period(&july, &PolicySet::default())
        .unwrap_err();
    assert_eq!(error.error_code(), "OPEN_OBLIGATION");
    engine.obligations().resolve(&obligation.id).unwrap();

    let started = engine
        .start_close_period(&july, &PolicySet::default())
        .unwrap();
    let done = engine
        .answer(&started.run_id, "ApproveClose", "approve")
        .unwrap();
    assert_eq!(done.status, RunStatus::Completed);
    assert_eq!(
        Ledger::open(home.path())
            .unwrap()
            .period_view(&july)
            .unwrap()
            .state,
        PeriodState::Sealed
    );
}

#[test]
fn monthly_obligation_blocks_legacy_quarter_close() {
    let home = init_home();
    let engine = ProcessEngine::open(home.path());
    engine
        .obligations()
        .create(
            &PeriodId::parse("2026-07").unwrap(),
            "missing",
            "missing receipt",
            "2026-08-31",
            "capture",
        )
        .unwrap();
    let error = engine
        .start_close_period(&PeriodId::parse("2026-Q3").unwrap(), &PolicySet::default())
        .unwrap_err();
    assert_eq!(error.error_code(), "OPEN_OBLIGATION");
}

#[test]
fn malformed_obligation_period_fails_visibly() {
    let home = init_home();
    let root = home.path().join("process/obligations");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("bad.json"),
        r#"{
          "id":"bad","period":"2026-7","kind":"missing",
          "description":"bad stored period","deadline":"2026-08-31",
          "created_by_run":"capture","status":"open"
        }"#,
    )
    .unwrap();
    let error = ProcessEngine::open(home.path())
        .start_close_period(&PeriodId::parse("2026-07").unwrap(), &PolicySet::default())
        .unwrap_err();
    assert_eq!(error.error_code(), "JOURNAL_CORRUPT");
    assert!(error.to_string().contains("invalid period"));
}

#[test]
fn malformed_resolved_obligation_does_not_block() {
    let home = init_home();
    let root = home.path().join("process/obligations");
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("old.json"),
        r#"{
          "id":"old","period":"2026-7","kind":"old",
          "description":"resolved legacy data","deadline":"2026-08-31",
          "created_by_run":"capture","status":"resolved"
        }"#,
    )
    .unwrap();
    assert!(ProcessEngine::open(home.path())
        .start_close_period(&PeriodId::parse("2026-07").unwrap(), &PolicySet::default())
        .is_ok());
}
