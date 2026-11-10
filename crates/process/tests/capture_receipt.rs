//! RF1 `capture-receipt` Run tests — the S5 consult slice + the LLM/law boundary (M5).
//! Each done-criterion is a distinct, non-happy-path test:
//! disclosure byte-binding (tamper refused), per-outbound-turn re-bind, the eligibility
//! boundary (claim not honored), `Advice` off the ledger, the durable `Effect::Wait`
//! resume, the F13 obligation surviving + blocking `close-period`, and the review-gated
//! knowledge write.

use serde_json::{json, Value};

use aggountant_broker::{
    Broker, ConsultLlmAdapter, RawResponse, Registry, ScriptedTransport, TransportError, Vault,
    SVC_CONSULT_LLM,
};
use aggountant_domain::PolicySet;
use aggountant_ledger::{Ledger, SealRequest};
use aggountant_model::PeriodId;
use aggountant_process::{
    CaptureReceiptInput, EventPayload, FixedClock, ProcessEngine, RunStatus, SystemClock,
};

fn q1() -> PeriodId {
    PeriodId::parse("2026-Q1").unwrap()
}

fn home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    home
}

type Steps = Vec<Result<RawResponse, TransportError>>;

fn ok(status: u16, body: Value) -> Result<RawResponse, TransportError> {
    Ok(RawResponse::new(status, body))
}

fn consult_broker(home: &std::path::Path, script: Steps) -> Broker {
    let mut vault = Vault::new();
    vault.set(SVC_CONSULT_LLM, "secret");
    let mut reg = Registry::new();
    reg.register_stream(Box::new(ConsultLlmAdapter::new(Box::new(
        ScriptedTransport::new(script),
    ))));
    Broker::new(home, vault, reg)
}

/// A passenger-car leasing receipt with a 70% business-use claim (the canonical boundary
/// case). The agent proposes a booking; the law clamps the deduction to 0.
fn input() -> CaptureReceiptInput {
    serde_json::from_value(json!({
        "period": "2026-Q1",
        "topic": "vat-deductibility",
        "question": "Is the VAT on this passenger-car lease deductible at 70%?",
        "facts": [
            {"key": "expense_nature", "value": "passenger-car leasing invoice"},
            {"key": "counterparty_type", "value": "car dealer"}
        ],
        "raw_source": "FICTIONAL RECEIPT FIXTURE amount=124.00",
        "nature": "passenger-car-mixed-use",
        "receipt_vat": {"amount": "124.00", "currency": "EUR"},
        "claimed_business_share": "0.70",
        "document_date": "2026-03-14",
        "obligation": {
            "kind": "ajopaivakirja-missing",
            "description": "ajopäiväkirja puuttuu",
            "deadline": "2026-04-30"
        },
        "knowledge_slug": "passenger-car-lease-vat",
        "draft": {
            "document_date": "2026-03-14",
            "posting_date": "2026-03-14",
            "description": "Auton leasing",
            "postings": [
                {"account": "7690", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "debit"},
                {"account": "1910", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "credit"}
            ]
        },
        "tool_results": []
    }))
    .unwrap()
}

fn explicit_vat_input(
    nature: &str,
    share: &str,
    receipt_vat: &str,
    expense: &str,
    deductible_vat: &str,
    gross: &str,
) -> CaptureReceiptInput {
    let mut value = serde_json::to_value(input()).unwrap();
    value["nature"] = json!(nature);
    value["claimed_business_share"] = json!(share);
    value["receipt_vat"] = json!({"amount": receipt_vat, "currency": "EUR"});
    value["draft"]["description"] = json!("Fictional classified purchase");
    value["draft"]["postings"] = json!([
        {"account": "7690", "amount": {"amount": expense, "currency": "EUR"}, "side": "debit"},
        {"account": "1763", "amount": {"amount": deductible_vat, "currency": "EUR"}, "side": "debit"},
        {"account": "1910", "amount": {"amount": gross, "currency": "EUR"}, "side": "credit"}
    ]);
    serde_json::from_value(value).unwrap()
}

fn no_obligation_input(as_null: bool) -> CaptureReceiptInput {
    let mut value = serde_json::to_value(input()).unwrap();
    if as_null {
        value["obligation"] = Value::Null;
    } else {
        value.as_object_mut().unwrap().remove("obligation");
    }
    serde_json::from_value(value).unwrap()
}

fn tagged_partial_input() -> CaptureReceiptInput {
    let mut value = serde_json::to_value(input()).unwrap();
    value["nature"] = json!("apportionable");
    value["claimed_business_share"] = json!("0.50");
    value["receipt_vat"] = json!({"amount": "25.50", "currency": "EUR"});
    value["draft"]["description"] = json!("Fictional shared-use telecom service");
    value["draft"]["postings"] = json!([
        {"account": "7690", "amount": {"amount": "100.00", "currency": "EUR"}, "side": "debit", "vat_code": "purchase-domestic-standard-deduct50"},
        {"account": "1910", "amount": {"amount": "100.00", "currency": "EUR"}, "side": "credit"}
    ]);
    serde_json::from_value(value).unwrap()
}

fn final_chunk() -> Value {
    json!({"type": "final", "advice": {"summary": "Use the deterministic eligibility assessment.", "citations": ["fictional policy fixture"]}})
}

/// Drive a fresh run to the ApproveVoucher gate (consult completed).
fn input_to_voucher_gate(
    engine: &ProcessEngine,
    broker: &Broker,
    input: &CaptureReceiptInput,
) -> String {
    let run = engine
        .start_capture_receipt(input, &PolicySet::default())
        .unwrap();
    let run_id = run.run_id.clone();
    engine
        .answer_capture_receipt(
            &run_id,
            "ApproveConsultDisclosure",
            "approve",
            broker,
            &SystemClock,
        )
        .unwrap();
    run_id
}

fn to_voucher_gate(engine: &ProcessEngine, broker: &Broker) -> String {
    input_to_voucher_gate(engine, broker, &input())
}

#[test]
fn happy_path_escalates_consults_seals_obligates_and_gates_knowledge() {
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = consult_broker(home.path(), vec![ok(200, final_chunk())]);

    let run_id = to_voucher_gate(&engine, &broker);
    // ApproveVoucher: seals the voucher + opens the obligation + opens the knowledge gate.
    let view = engine
        .answer_capture_receipt(&run_id, "ApproveVoucher", "approve", &broker, &SystemClock)
        .unwrap();
    assert_eq!(view.status, RunStatus::AwaitingGate);

    // An F13 obligation now exists for the period (survives this Run).
    let obligations = engine.obligations().open_for_period(&q1()).unwrap();
    assert_eq!(obligations.len(), 1);
    assert_eq!(obligations[0].kind, "ajopaivakirja-missing");

    // Knowledge is NOT written until the review gate is approved (no auto-cache).
    assert!(engine.knowledge().list().unwrap().is_empty());
    let view = engine
        .answer_capture_receipt(
            &run_id,
            "ApproveKnowledgePR",
            "approve",
            &broker,
            &SystemClock,
        )
        .unwrap();
    assert_eq!(view.status, RunStatus::Completed);
    assert_eq!(
        engine.knowledge().list().unwrap(),
        vec!["passenger-car-lease-vat"]
    );
}

#[test]
fn missing_or_null_obligation_seals_without_f13_and_period_can_close() {
    for as_null in [false, true] {
        let home = home();
        let engine = ProcessEngine::open(home.path());
        let broker = consult_broker(home.path(), vec![ok(200, final_chunk())]);
        let input = no_obligation_input(as_null);
        assert!(input.obligation.is_none());

        let run_id = input_to_voucher_gate(&engine, &broker, &input);
        engine
            .answer_capture_receipt(&run_id, "ApproveVoucher", "approve", &broker, &SystemClock)
            .unwrap();

        assert_eq!(
            Ledger::open(home.path())
                .unwrap()
                .verify()
                .unwrap()
                .voucher_count,
            1,
            "voucher must seal when obligation is {}",
            if as_null { "null" } else { "missing" }
        );
        assert!(engine.obligations().list().unwrap().is_empty());
        let events = engine.journal(&run_id).unwrap();
        assert!(events.iter().any(|event| matches!(
            &event.payload,
            EventPayload::EffectSucceeded { effect_key, note }
                if effect_key == "obligation" && note.contains("no follow-up obligation")
        )));
        engine
            .start_close_period(&q1(), &PolicySet::default())
            .expect("absence of an F13 obligation must not block period close");
    }
}

#[test]
fn malformed_obligation_object_is_not_treated_as_absent() {
    for malformed in [
        json!({}),
        json!({"kind": "missing-description", "deadline": "2026-04-30"}),
        json!({
            "kind": "bad-deadline",
            "description": "fictional follow-up",
            "deadline": "soon"
        }),
        json!({
            "kind": "extra-field",
            "description": "fictional follow-up",
            "deadline": "2026-04-30",
            "unexpected": true
        }),
    ] {
        let mut value = serde_json::to_value(input()).unwrap();
        value["obligation"] = malformed;
        assert!(serde_json::from_value::<CaptureReceiptInput>(value).is_err());
    }
}

#[test]
fn eligibility_boundary_claim_of_70pct_is_not_honored_deductible_is_zero() {
    // The M5 boundary: the user's 70% claim does NOT override codified law. The Domain
    // computes deductible_vat = 0 for a passenger car in private use.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = consult_broker(home.path(), vec![ok(200, final_chunk())]);
    let run_id = to_voucher_gate(&engine, &broker);

    let events = engine.journal(&run_id).unwrap();
    let assess_note = events
        .iter()
        .find_map(|e| match &e.payload {
            EventPayload::EffectSucceeded { effect_key, note } if effect_key == "assess" => {
                Some(note.clone())
            }
            _ => None,
        })
        .expect("assess effect succeeded");
    assert!(
        assess_note.contains("deductible_vat 0.00") && assess_note.contains("honored=false"),
        "law must clamp the claim to 0: {assess_note}"
    );
}

#[test]
fn contradictory_explicit_vat_is_refused_without_changing_sealed_history() {
    // Regression from the controlled real-data-derived probe, using fictional source
    // facts: eligibility is 0.00 while the approved draft debits input VAT by 65.33.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let ledger = Ledger::open(home.path()).unwrap();

    // Existing immutable history remains readable and unchanged across the refusal.
    let existing = aggountant_domain::build_draft(input().draft).unwrap();
    ledger
        .seal(
            &existing,
            &SealRequest {
                natural_key: "existing-history".into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();

    let broker = consult_broker(home.path(), vec![ok(200, final_chunk())]);
    let contradictory = explicit_vat_input(
        "passenger-car-mixed-use",
        "0.70",
        "65.33",
        "416.88",
        "65.33",
        "482.21",
    );
    let run_id = input_to_voucher_gate(&engine, &broker, &contradictory);
    let err = engine
        .answer_capture_receipt(&run_id, "ApproveVoucher", "approve", &broker, &SystemClock)
        .unwrap_err();
    assert_eq!(err.error_code(), "CAPTURE_VAT_MISMATCH");
    assert!(err.to_string().contains("65.33"));
    assert!(err.to_string().contains("0.00"));

    let verified = Ledger::open(home.path()).unwrap().verify().unwrap();
    assert_eq!(
        verified.voucher_count, 1,
        "the attempted approval sealed nothing"
    );

    // The answered-gate recovery path must apply the same reconciliation; resume cannot
    // bypass it after a crash or a caller retry.
    let retry = engine
        .resume_capture_receipt(&run_id, &broker, &SystemClock)
        .unwrap_err();
    assert_eq!(retry.error_code(), "CAPTURE_VAT_MISMATCH");
    assert_eq!(
        Ledger::open(home.path())
            .unwrap()
            .verify()
            .unwrap()
            .voucher_count,
        1
    );
}

#[test]
fn fully_deductible_explicit_vat_matching_assessment_seals() {
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = consult_broker(home.path(), vec![ok(200, final_chunk())]);
    let valid = explicit_vat_input(
        "fully-deductible",
        "1",
        "25.50",
        "100.00",
        "25.50",
        "125.50",
    );
    let run_id = input_to_voucher_gate(&engine, &broker, &valid);
    engine
        .answer_capture_receipt(&run_id, "ApproveVoucher", "approve", &broker, &SystemClock)
        .unwrap();
    assert_eq!(
        Ledger::open(home.path())
            .unwrap()
            .verify()
            .unwrap()
            .voucher_count,
        1
    );
}

#[test]
fn tagged_filing_projection_matching_assessment_seals_the_exact_bound_draft() {
    // Filing-projection consistency control only: under the supported legacy contract,
    // the tagged 100.00 posting is a VAT base and derives 100.00 × 25.5% × 50% =
    // 12.75. This does not certify the legacy two-leg shape as an economically complete
    // purchase booking; that wider limitation is tracked by
    // `adr-7-vat-a-import-completeness`. It proves this bounded guard cannot skip a
    // tagged deduction while comparing eligibility with what filing will read.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = consult_broker(home.path(), vec![ok(200, final_chunk())]);
    let valid = tagged_partial_input();
    assert_eq!(
        serde_json::to_value(&valid.draft).unwrap()["postings"],
        json!([
            {"account": "7690", "amount": {"amount": "100.00", "currency": "EUR"}, "side": "debit", "vat_code": "purchase-domestic-standard-deduct50"},
            {"account": "1910", "amount": {"amount": "100.00", "currency": "EUR"}, "side": "credit"}
        ]),
        "the gate must bind and seal this exact legacy draft; the control must not silently rewrite it"
    );
    let run_id = input_to_voucher_gate(&engine, &broker, &valid);
    engine
        .answer_capture_receipt(&run_id, "ApproveVoucher", "approve", &broker, &SystemClock)
        .unwrap();
    assert_eq!(
        Ledger::open(home.path())
            .unwrap()
            .verify()
            .unwrap()
            .voucher_count,
        1
    );
}

#[test]
fn disclosure_gate_binds_projected_bytes_a_tamper_is_refused() {
    // The ApproveConsultDisclosure preview gate binds the *projected* bytes (Ba). Tampering
    // the persisted projection after binding recomputes to a different hash → refused.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = consult_broker(home.path(), vec![ok(200, final_chunk())]);
    let run = engine
        .start_capture_receipt(&input(), &PolicySet::default())
        .unwrap();

    // Tamper the persisted disclosure body under the run.
    let state_path = home
        .path()
        .join("process/runs")
        .join(&run.run_id)
        .join("capture_state.json");
    let mut state: Value = serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
    state["disclosure_body"]["facts"][0]["value"] = json!("TAMPERED — leak the whole receipt");
    std::fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();

    let err = engine
        .answer_capture_receipt(
            &run.run_id,
            "ApproveConsultDisclosure",
            "approve",
            &broker,
            &SystemClock,
        )
        .unwrap_err();
    assert_eq!(err.error_code(), "JOURNAL_CORRUPT");
}

#[test]
fn every_outbound_turn_re_projects_and_re_binds() {
    // A multi-turn consult with a tool-call round trip: each outbound turn re-projects and
    // re-binds, recorded as a DisclosureBound event with a distinct hash.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = consult_broker(
        home.path(),
        vec![
            ok(
                200,
                json!({"type": "tool_call", "tool": "lookup-vat-code", "args": {}}),
            ),
            ok(200, final_chunk()),
        ],
    );
    let mut inp = input();
    inp.tool_results = vec![serde_json::from_value(json!({
        "tool": "lookup-vat-code",
        "result": {"code": "car-lease-no-deduction"}
    }))
    .unwrap()];

    let run = engine
        .start_capture_receipt(&inp, &PolicySet::default())
        .unwrap();
    engine
        .answer_capture_receipt(
            &run.run_id,
            "ApproveConsultDisclosure",
            "approve",
            &broker,
            &SystemClock,
        )
        .unwrap();

    let hashes: Vec<String> = engine
        .journal(&run.run_id)
        .unwrap()
        .iter()
        .filter_map(|e| match &e.payload {
            EventPayload::DisclosureBound { sent_hash, .. } => Some(sent_hash.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(hashes.len(), 2, "each outbound turn must re-bind");
    assert_ne!(
        hashes[0], hashes[1],
        "re-projected turn binds different bytes"
    );
}

#[test]
fn consult_rate_limit_parks_a_durable_wait_and_resume_recovers() {
    // Effect::Wait reliability at the RF1 level: a 429 parks the consult on a durable
    // wait; resume before the deadline stays Waiting; after the backoff it recovers. The
    // clock is injected — no wall-clock sleep — and the deadline is journal state, so a
    // fresh engine (a "crash") resumes correctly.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let clock = FixedClock::new(1_000_000);
    // First turn rate-limited (backoff 300s), second turn succeeds.
    let broker = consult_broker(
        home.path(),
        vec![
            ok(429, json!({"retry_after_secs": 300})),
            ok(200, final_chunk()),
        ],
    );

    let run = engine
        .start_capture_receipt(&input(), &PolicySet::default())
        .unwrap();
    let view = engine
        .answer_capture_receipt(
            &run.run_id,
            "ApproveConsultDisclosure",
            "approve",
            &broker,
            &clock,
        )
        .unwrap();
    assert_eq!(view.status, RunStatus::Waiting, "parked on a quota wait");

    // A "crash": a brand-new engine reads the durable deadline from the journal.
    let engine2 = ProcessEngine::open(home.path());
    // Still within the backoff → stays Waiting.
    let view = engine2
        .resume_capture_receipt(&run.run_id, &broker, &clock)
        .unwrap();
    assert_eq!(view.status, RunStatus::Waiting);

    // Advance past the deadline → the consult resumes and the run reaches ApproveVoucher.
    clock.advance(300);
    let view = engine2
        .resume_capture_receipt(&run.run_id, &broker, &clock)
        .unwrap();
    assert_eq!(view.status, RunStatus::AwaitingGate);
    assert!(view.gates.iter().any(|g| g.gate_id == "ApproveVoucher"));
}

#[test]
fn resume_opens_knowledge_gate_after_crash_following_obligation_decision() {
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = consult_broker(home.path(), vec![ok(200, final_chunk())]);
    let run_id = to_voucher_gate(&engine, &broker);
    engine
        .answer_capture_receipt(&run_id, "ApproveVoucher", "approve", &broker, &SystemClock)
        .unwrap();

    // Simulate a crash after both durable effects succeeded but before the next gate was
    // published: removing the final event leaves a valid hash-chain prefix.
    let events = engine.journal(&run_id).unwrap();
    let final_event = events.last().unwrap();
    assert!(matches!(
        &final_event.payload,
        EventPayload::GateOpened { gate_id, .. } if gate_id == "ApproveKnowledgePR"
    ));
    std::fs::remove_file(
        home.path()
            .join("process/runs")
            .join(&run_id)
            .join("events")
            .join(format!("{:06}.json", final_event.seq)),
    )
    .unwrap();

    let view = engine
        .resume_capture_receipt(&run_id, &broker, &SystemClock)
        .unwrap();
    assert!(view
        .gates
        .iter()
        .any(|gate| gate.gate_id == "ApproveKnowledgePR"));
    assert_eq!(engine.obligations().list().unwrap().len(), 1);
    for effect_key in ["seal-voucher", "obligation"] {
        assert_eq!(
            engine
                .journal(&run_id)
                .unwrap()
                .iter()
                .filter(|event| matches!(
                    &event.payload,
                    EventPayload::EffectSucceeded { effect_key: key, .. } if key == effect_key
                ))
                .count(),
            1
        );
    }
}

#[test]
fn obligation_survives_run_close_and_blocks_close_period() {
    // The F13 create → survive → block cycle, end to end.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = consult_broker(home.path(), vec![ok(200, final_chunk())]);

    let run_id = to_voucher_gate(&engine, &broker);
    engine
        .answer_capture_receipt(&run_id, "ApproveVoucher", "approve", &broker, &SystemClock)
        .unwrap();

    // close-period for Q1 is now blocked by the open obligation.
    let err = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap_err();
    assert_eq!(err.error_code(), "OPEN_OBLIGATION");

    // Resolve it → close-period proceeds.
    let obligations = engine.obligations().open_for_period(&q1()).unwrap();
    engine.obligations().resolve(&obligations[0].id).unwrap();
    let view = engine
        .start_close_period(&q1(), &PolicySet::default())
        .unwrap();
    assert!(matches!(
        view.status,
        RunStatus::AwaitingGate | RunStatus::Running
    ));
}

#[test]
fn a_rejected_disclosure_declines_the_run() {
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = consult_broker(home.path(), vec![ok(200, final_chunk())]);
    let run = engine
        .start_capture_receipt(&input(), &PolicySet::default())
        .unwrap();
    let view = engine
        .answer_capture_receipt(
            &run.run_id,
            "ApproveConsultDisclosure",
            "reject",
            &broker,
            &SystemClock,
        )
        .unwrap();
    assert_eq!(view.status, RunStatus::Rejected);
    // Nothing was consulted or sealed.
    assert_eq!(engine.ledger_voucher_count(home.path()), 0);
}

#[test]
fn knowledge_gate_binds_the_candidate_bytes_a_tamper_is_refused() {
    // F4: the ApproveKnowledgePR gate binds the candidate hash; mutating the persisted
    // advice after the gate opened recomputes to a different hash and is refused.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = consult_broker(home.path(), vec![ok(200, final_chunk())]);
    let run_id = to_voucher_gate(&engine, &broker);
    engine
        .answer_capture_receipt(&run_id, "ApproveVoucher", "approve", &broker, &SystemClock)
        .unwrap();

    // Tamper the persisted advice summary after the knowledge gate opened.
    let state_path = home
        .path()
        .join("process/runs")
        .join(&run_id)
        .join("capture_state.json");
    let mut state: Value = serde_json::from_slice(&std::fs::read(&state_path).unwrap()).unwrap();
    state["advice"]["summary"] = json!("TAMPERED knowledge the reviewer never saw");
    std::fs::write(&state_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();

    let err = engine
        .answer_capture_receipt(
            &run_id,
            "ApproveKnowledgePR",
            "approve",
            &broker,
            &SystemClock,
        )
        .unwrap_err();
    assert_eq!(err.error_code(), "JOURNAL_CORRUPT");
    assert!(engine.knowledge().list().unwrap().is_empty());
}

#[test]
fn historical_object_shaped_state_resumes_repeatedly_without_duplicate_effects() {
    // Historical capture state encoded `obligation` directly as an object. Option's
    // untagged serde representation reads those bytes as `Some`, preserving the
    // obligation across upgrade and every subsequent resume.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = consult_broker(home.path(), vec![ok(200, final_chunk())]);
    let run_id = to_voucher_gate(&engine, &broker);
    let state_path = home
        .path()
        .join("process/runs")
        .join(&run_id)
        .join("capture_state.json");
    let state: Value = serde_json::from_slice(&std::fs::read(state_path).unwrap()).unwrap();
    assert!(
        state["obligation"].is_object(),
        "historical wire shape changed"
    );

    engine
        .answer_capture_receipt(&run_id, "ApproveVoucher", "approve", &broker, &SystemClock)
        .unwrap();
    engine
        .answer_capture_receipt(
            &run_id,
            "ApproveKnowledgePR",
            "approve",
            &broker,
            &SystemClock,
        )
        .unwrap();
    let before = Ledger::open(home.path())
        .unwrap()
        .verify()
        .unwrap()
        .voucher_count;
    let obl_before = engine.obligations().list().unwrap().len();

    for _ in 0..2 {
        let view = engine
            .resume_capture_receipt(&run_id, &broker, &SystemClock)
            .unwrap();
        assert_eq!(view.status, RunStatus::Completed);
    }
    assert_eq!(
        Ledger::open(home.path())
            .unwrap()
            .verify()
            .unwrap()
            .voucher_count,
        before,
        "resume must not re-seal"
    );
    assert_eq!(
        engine.obligations().list().unwrap().len(),
        obl_before,
        "resume must not duplicate the obligation"
    );
    let events = engine.journal(&run_id).unwrap();
    for effect_key in ["seal-voucher", "obligation"] {
        assert_eq!(
            events
                .iter()
                .filter(|event| matches!(
                    &event.payload,
                    EventPayload::EffectSucceeded { effect_key: key, .. } if key == effect_key
                ))
                .count(),
            1,
            "resume must not duplicate {effect_key} success events"
        );
    }
}

// A tiny helper: count sealed vouchers via a fresh verify (no vouchers → head 0).
trait VoucherCount {
    fn ledger_voucher_count(&self, home: &std::path::Path) -> u64;
}
impl VoucherCount for ProcessEngine {
    fn ledger_voucher_count(&self, home: &std::path::Path) -> u64 {
        Ledger::open(home).unwrap().verify().unwrap().voucher_count
    }
}
