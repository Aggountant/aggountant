//! The monthly employer's separate report against a scripted Incomes Register.
use aggountant_broker::{
    tulorekisteri::{SVC_EMPLOYER, SVC_EMPLOYER_STATUS, SVC_STATUS, SVC_WAGE},
    *,
};
use aggountant_domain::{
    payroll::{
        EmployerReportAction, EmployerReportContent, MonthPayments, PensionInsurance, ReportMonth,
    },
    TaxCard, WageReport,
};
use aggountant_ledger::{EmployeeState, Ledger};
use aggountant_process::{
    EmployerMonthState, EmployerReportInput, EmployerReportReceipt, EmployerRunOutcome,
    ProcessEngine, ProcessError,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

const IR_DELIVERY: &str = "11111111222233334444555555555555";
const IR_REPORT: &str = "22222222222233334444555555555555";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Answer {
    Accept(u32),
    Reject,
    Unknown,
}

#[derive(Clone)]
struct Script {
    send: Arc<Mutex<Vec<Answer>>>,
    status: Arc<Mutex<Answer>>,
    sends: Arc<AtomicUsize>,
    reads: Arc<AtomicUsize>,
}

struct Mock {
    status: bool,
    script: Script,
}

fn accepted(version: u32) -> AdapterResponse {
    AdapterResponse::Accepted {
        receipt: format!("{IR_DELIVERY}/{IR_REPORT}/{version}"),
        body: json!({"ir_delivery_id":IR_DELIVERY,"ir_report_id":IR_REPORT,"version":version}),
    }
}

impl RequestResponse for Mock {
    fn service(&self) -> &str {
        if self.status {
            SVC_EMPLOYER_STATUS
        } else {
            SVC_EMPLOYER
        }
    }
    fn kind(&self) -> AdapterKind {
        if self.status {
            AdapterKind::Pull
        } else {
            AdapterKind::Submit
        }
    }
    fn assurance(&self) -> DeliverySemantics {
        DeliverySemantics::AtMostOnce
    }
    fn call(&self, e: &Envelope, _: &CredentialHandle) -> Result<AdapterResponse, AdapterError> {
        let answer = if self.status {
            assert_eq!(e.operation, "GetDeliveryDataStatus");
            assert!(e.body["status_timestamp"].is_string());
            self.script.reads.fetch_add(1, Ordering::SeqCst);
            *self.script.status.lock().unwrap()
        } else {
            assert_eq!(e.operation, "SendPayerSummaryReport");
            self.script.sends.fetch_add(1, Ordering::SeqCst);
            self.script.send.lock().unwrap().remove(0)
        };
        match answer {
            Answer::Accept(v) => Ok(accepted(v)),
            Answer::Reject => Err(AdapterError::Rejected {
                code: "TULOREKISTERI_REJECTED".into(),
                message: "scripted rejection".into(),
            }),
            Answer::Unknown => Err(AdapterError::Timeout("scripted timeout".into())),
        }
    }
}

fn broker(home: &std::path::Path, send: Vec<Answer>, status: Answer) -> (Broker, Script) {
    let script = Script {
        send: Arc::new(Mutex::new(send)),
        status: Arc::new(Mutex::new(status)),
        sends: Arc::new(AtomicUsize::new(0)),
        reads: Arc::new(AtomicUsize::new(0)),
    };
    let mut vault = Vault::new();
    let mut registry = Registry::new();
    for status in [false, true] {
        let m = Mock {
            status,
            script: script.clone(),
        };
        vault.set(m.service(), "test");
        registry.register(Box::new(m));
    }
    // The wage services are registered too so a wage run can be started offline.
    vault.set(SVC_WAGE, "test");
    vault.set(SVC_STATUS, "test");
    (Broker::new(home, vault, registry), script)
}

fn home(pensions: serde_json::Value) -> tempfile::TempDir {
    let h = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(h.path()).unwrap();
    ledger
        .upsert_employee(&EmployeeState {
            employee_id: "emp-1".into(),
            name: "Fictional Owner".into(),
            hetu: "010101A123N".into(),
            iban: "FI2112345600000785".into(),
            tax_card: TaxCard {
                withholding_pct: "0".into(),
            },
        })
        .unwrap();
    std::fs::create_dir_all(h.path().join(".aggountant")).unwrap();
    std::fs::write(
        h.path().join(".aggountant/tulorekisteri.json"),
        json!({"schema_version":1,"mode":"production","business_id":"1234567-1","reporting_permission_active":true,"credentials_file":null,
            "employer_report":{"registered_from":"2026-07","contact_name":"Fictional Contact","contact_phone":"+358 40 1234567","pension_insurances":pensions}})
            .to_string(),
    )
    .unwrap();
    h
}

fn month(s: &str) -> ReportMonth {
    ReportMonth::parse(s).unwrap()
}
fn input(m: &str, payments: MonthPayments) -> EmployerReportInput {
    EmployerReportInput {
        month: month(m),
        payments,
        replace: false,
    }
}
fn t(s: &str) -> DateTime<Utc> {
    s.parse().unwrap()
}
fn code(e: ProcessError) -> &'static str {
    e.error_code()
}

#[test]
fn no_wages_month_previews_offline_submits_once_and_the_receipt_fences_it() {
    let h = home(json!([{"provider_code":46,"policy_number":"12-3456789"}]));
    let e = ProcessEngine::open(h.path());
    let at = t("2026-11-02T08:00:00Z");
    let p = e
        .preview_employer_report(input("2026-10", MonthPayments::None), at)
        .unwrap();
    assert_eq!(p.report.content, EmployerReportContent::NoWagesPayable);
    assert_eq!(p.report.action, EmployerReportAction::New);
    assert_eq!(p.deadline.to_string(), "2026-11-05");
    assert_eq!(p.report.sent_pension_insurances().len(), 1);
    assert!(p.warnings.is_empty());
    assert!(!h.path().join("process").exists(), "preview never writes");

    let (b, s) = broker(h.path(), vec![Answer::Accept(1)], Answer::Accept(1));
    let wrong = e.submit_employer_report(
        input("2026-10", MonthPayments::None),
        at,
        "deadbeef",
        &b,
        at,
    );
    assert_eq!(code(wrong.unwrap_err()), "EMPLOYER_REPORT_HASH_MISMATCH");
    // An approval binds the payments too: the same hash cannot send another content.
    let other = e.submit_employer_report(
        input("2026-10", MonthPayments::TaxFreeReimbursementsOnly),
        at,
        &p.record_hash,
        &b,
        at,
    );
    assert_eq!(code(other.unwrap_err()), "EMPLOYER_REPORT_HASH_MISMATCH");
    assert_eq!(s.sends.load(Ordering::SeqCst), 0);

    let status = e
        .submit_employer_report(
            input("2026-10", MonthPayments::None),
            at,
            &p.record_hash,
            &b,
            at,
        )
        .unwrap();
    assert_eq!(status.state, EmployerMonthState::Accepted);
    let receipt = status.current.clone().unwrap();
    assert_eq!(receipt.ir_report_id, IR_REPORT);
    assert_eq!(receipt.version, Some(1));
    // `run show` reads the same receipt offline.
    let shown = e.show(&status.runs[0].run_id).unwrap();
    assert_eq!(
        shown.effects[0].outcome,
        Some(aggountant_process::AuthorityOutcome::Accepted {
            receipt: Some(aggountant_process::AuthorityReceipt::Tulorekisteri(
                receipt.clone()
            )),
            next_command: None,
        })
    );
    assert_eq!(
        status.runs[0].record_hash.as_deref(),
        Some(p.record_hash.as_str())
    );

    for again in [at, t("2026-11-02T09:00:00Z")] {
        let p2 = e.preview_employer_report(input("2026-10", MonthPayments::None), again);
        assert_eq!(code(p2.unwrap_err()), "EMPLOYER_REPORT_ALREADY_FILED");
        let r = e.submit_employer_report(
            input("2026-10", MonthPayments::None),
            again,
            &p.record_hash,
            &b,
            again,
        );
        assert_eq!(code(r.unwrap_err()), "EMPLOYER_REPORT_ALREADY_FILED");
    }
    assert_eq!(s.sends.load(Ordering::SeqCst), 1);
}

#[test]
fn reimbursements_only_month_reports_zero_contribution_and_never_no_wages() {
    let h = home(json!([{"provider_code":46,"policy_number":"12-3456789"}]));
    let e = ProcessEngine::open(h.path());
    let at = t("2026-11-02T08:00:00Z");
    // Without a file-wage report in the month the approver is warned, not refused.
    let p = e
        .preview_employer_report(
            input("2026-10", MonthPayments::TaxFreeReimbursementsOnly),
            at,
        )
        .unwrap();
    assert_eq!(p.warnings.len(), 1);

    let travel: WageReport = serde_json::from_value(json!({"business_id":"1234567-1","source_reference":"trip-2026-10","employee_id":"emp-1","payment_date":"2026-10-25","period_start":"2026-10-01","period_end":"2026-10-20","joint_owner":true,"contact_name":"Fictional Contact","contact_phone":"+358 40 1234567","lines":[{"income_type":"311","amount":"120.00","kilometers":200}]})).unwrap();
    e.start_wage(&travel, t("2026-10-25T12:00:00Z")).unwrap();

    let none = e.preview_employer_report(input("2026-10", MonthPayments::None), at);
    assert_eq!(code(none.unwrap_err()), "EMPLOYER_REPORT_PAYMENTS_CONFLICT");
    let p = e
        .preview_employer_report(
            input("2026-10", MonthPayments::TaxFreeReimbursementsOnly),
            at,
        )
        .unwrap();
    assert!(p.warnings.is_empty());
    assert_eq!(p.report.content.transaction(), ("102", Some("0.00")));
    // Pension insurances accompany only "no wages payable".
    assert!(p.report.sent_pension_insurances().is_empty());

    let (b, s) = broker(h.path(), vec![Answer::Accept(1)], Answer::Accept(1));
    let status = e
        .submit_employer_report(
            input("2026-10", MonthPayments::TaxFreeReimbursementsOnly),
            at,
            &p.record_hash,
            &b,
            at,
        )
        .unwrap();
    assert_eq!(status.state, EmployerMonthState::Accepted);
    assert_eq!(s.sends.load(Ordering::SeqCst), 1);
}

#[test]
fn month_with_wages_and_out_of_range_months_are_refused_before_anything_is_written() {
    let h = home(json!([]));
    let e = ProcessEngine::open(h.path());
    let at = t("2026-11-02T08:00:00Z");
    let cases = [
        (
            input("2026-10", MonthPayments::Wages),
            "EMPLOYER_REPORT_WAGES_UNSUPPORTED",
        ),
        (
            input("2026-06", MonthPayments::None),
            "EMPLOYER_REPORT_BEFORE_REGISTRATION",
        ),
        (
            input("2026-12", MonthPayments::None),
            "EMPLOYER_REPORT_FUTURE_MONTH",
        ),
        (
            EmployerReportInput {
                replace: true,
                ..input("2026-10", MonthPayments::None)
            },
            "EMPLOYER_REPORT_NOTHING_TO_REPLACE",
        ),
    ];
    for (i, want) in cases {
        assert_eq!(code(e.preview_employer_report(i, at).unwrap_err()), want);
    }
    let p = e
        .preview_employer_report(input("2026-10", MonthPayments::None), at)
        .unwrap();
    let (b, s) = broker(h.path(), vec![Answer::Accept(1)], Answer::Accept(1));
    let stale = e.submit_employer_report(
        input("2026-10", MonthPayments::None),
        at,
        &p.record_hash,
        &b,
        t("2026-11-04T08:00:00Z"),
    );
    assert_eq!(code(stale.unwrap_err()), "EMPLOYER_REPORT_STALE_TIMESTAMP");
    assert_eq!(s.sends.load(Ordering::SeqCst), 0);
    assert!(!h.path().join("process/runs").exists());
}

#[test]
fn rejection_is_definitive_and_a_corrected_report_may_be_sent() {
    let h = home(json!([]));
    let e = ProcessEngine::open(h.path());
    let at = t("2026-11-02T08:00:00Z");
    let (b, s) = broker(
        h.path(),
        vec![Answer::Reject, Answer::Accept(1)],
        Answer::Accept(1),
    );
    let p = e
        .preview_employer_report(input("2026-10", MonthPayments::None), at)
        .unwrap();
    let status = e
        .submit_employer_report(
            input("2026-10", MonthPayments::None),
            at,
            &p.record_hash,
            &b,
            at,
        )
        .unwrap();
    assert_eq!(status.state, EmployerMonthState::Rejected);
    assert!(
        matches!(&status.runs[0].outcome, EmployerRunOutcome::Rejected { error_code } if error_code == "TULOREKISTERI_REJECTED")
    );
    // The same bytes are never sent again.
    let same = e.submit_employer_report(
        input("2026-10", MonthPayments::None),
        at,
        &p.record_hash,
        &b,
        at,
    );
    assert_eq!(code(same.unwrap_err()), "EMPLOYER_REPORT_ALREADY_SENT");
    let missing = e
        .missing_employer_reports(NaiveDate::from_ymd_opt(2026, 11, 16).unwrap())
        .unwrap();
    assert!(missing
        .months
        .iter()
        .any(|m| m.month == month("2026-10") && m.state == EmployerMonthState::Rejected));
    // A new preview gets a new record reference and may be sent.
    let later = t("2026-11-02T10:00:00Z");
    let p2 = e
        .preview_employer_report(input("2026-10", MonthPayments::None), later)
        .unwrap();
    assert_ne!(p2.delivery_id, p.delivery_id);
    let status = e
        .submit_employer_report(
            input("2026-10", MonthPayments::None),
            later,
            &p2.record_hash,
            &b,
            later,
        )
        .unwrap();
    assert_eq!(status.state, EmployerMonthState::Accepted);
    assert_eq!(status.runs.len(), 2);
    assert_eq!(s.sends.load(Ordering::SeqCst), 2);
}

#[test]
fn unknown_outcome_blocks_every_send_until_the_status_query_settles_it() {
    for (status_answer, settled) in [
        (Answer::Accept(1), EmployerMonthState::Accepted),
        (Answer::Reject, EmployerMonthState::Rejected),
    ] {
        let h = home(json!([]));
        let e = ProcessEngine::open(h.path());
        let at = t("2026-11-02T08:00:00Z");
        let (b, s) = broker(h.path(), vec![Answer::Unknown], status_answer);
        let p = e
            .preview_employer_report(input("2026-10", MonthPayments::None), at)
            .unwrap();
        let status = e
            .submit_employer_report(
                input("2026-10", MonthPayments::None),
                at,
                &p.record_hash,
                &b,
                at,
            )
            .unwrap();
        assert_eq!(status.state, EmployerMonthState::OutcomeUnknown);
        assert!(e.employer_report_in_doubt(month("2026-10")).unwrap());
        for again in [at, t("2026-11-02T09:00:00Z")] {
            let r = e.preview_employer_report(input("2026-10", MonthPayments::None), again);
            assert_eq!(code(r.unwrap_err()), "EMPLOYER_REPORT_OUTCOME_UNKNOWN");
            let r = e.submit_employer_report(
                input("2026-10", MonthPayments::None),
                at,
                &p.record_hash,
                &b,
                at,
            );
            assert_eq!(code(r.unwrap_err()), "EMPLOYER_REPORT_OUTCOME_UNKNOWN");
        }
        let status = e
            .reconcile_employer_report(month("2026-10"), &b, t("2026-11-02T08:10:00Z"))
            .unwrap();
        assert_eq!(status.state, settled);
        assert_eq!(s.sends.load(Ordering::SeqCst), 1);
        assert_eq!(s.reads.load(Ordering::SeqCst), 1);
        // Settled months need no further query.
        e.reconcile_employer_report(month("2026-10"), &b, t("2026-11-02T08:20:00Z"))
            .unwrap();
        assert_eq!(s.reads.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn a_status_query_without_a_definitive_answer_keeps_the_month_parked() {
    let h = home(json!([]));
    let e = ProcessEngine::open(h.path());
    let at = t("2026-11-02T08:00:00Z");
    let (b, s) = broker(h.path(), vec![Answer::Unknown], Answer::Unknown);
    let p = e
        .preview_employer_report(input("2026-10", MonthPayments::None), at)
        .unwrap();
    e.submit_employer_report(
        input("2026-10", MonthPayments::None),
        at,
        &p.record_hash,
        &b,
        at,
    )
    .unwrap();
    let err = e
        .reconcile_employer_report(month("2026-10"), &b, at)
        .unwrap_err();
    assert_eq!(err.error_code(), "EMPLOYER_REPORT_STATUS_UNKNOWN");
    assert_eq!(err.kind(), "system");
    assert_eq!(
        e.employer_report_status(month("2026-10")).unwrap().state,
        EmployerMonthState::OutcomeUnknown
    );
    // E-service evidence settles it; nothing is sent.
    let status = e
        .complete_employer_report_manual(
            month("2026-10"),
            EmployerReportReceipt {
                ir_delivery_id: None,
                ir_report_id: IR_REPORT.into(),
                version: Some(1),
                source: "ignored".into(),
            },
            at,
        )
        .unwrap();
    assert_eq!(status.state, EmployerMonthState::Accepted);
    assert_eq!(status.current.unwrap().source, "e_service");
    assert_eq!(s.sends.load(Ordering::SeqCst), 1);
}

#[test]
fn crash_after_an_accepted_send_recovers_from_the_outbox_without_network() {
    let h = home(json!([]));
    let e = ProcessEngine::open(h.path());
    let at = t("2026-11-02T08:00:00Z");
    let (b, s) = broker(h.path(), vec![Answer::Accept(1)], Answer::Unknown);
    let p = e
        .preview_employer_report(input("2026-10", MonthPayments::None), at)
        .unwrap();
    let status = e
        .submit_employer_report(
            input("2026-10", MonthPayments::None),
            at,
            &p.record_hash,
            &b,
            at,
        )
        .unwrap();
    let run_id = status.runs[0].run_id.clone();
    // Remove only the tail event of this disposable run: the outbox keeps the success.
    let dir = h.path().join("process/runs").join(&run_id).join("events");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    std::fs::remove_file(files.last().unwrap()).unwrap();
    assert_eq!(
        e.employer_report_status(month("2026-10")).unwrap().state,
        EmployerMonthState::OutcomeUnknown
    );
    let status = e
        .reconcile_employer_report(month("2026-10"), &b, at)
        .unwrap();
    assert_eq!(status.state, EmployerMonthState::Accepted);
    assert_eq!(s.sends.load(Ordering::SeqCst), 1);
    assert_eq!(s.reads.load(Ordering::SeqCst), 0);
}

#[test]
fn replacement_targets_the_accepted_report_and_its_version() {
    let h = home(json!([]));
    let e = ProcessEngine::open(h.path());
    let at = t("2026-11-02T08:00:00Z");
    let (b, s) = broker(
        h.path(),
        vec![Answer::Accept(1), Answer::Accept(2)],
        Answer::Accept(1),
    );
    let p = e
        .preview_employer_report(input("2026-10", MonthPayments::None), at)
        .unwrap();
    e.submit_employer_report(
        input("2026-10", MonthPayments::None),
        at,
        &p.record_hash,
        &b,
        at,
    )
    .unwrap();
    let replace = |payments| EmployerReportInput {
        replace: true,
        ..input("2026-10", payments)
    };
    let unchanged = e.preview_employer_report(replace(MonthPayments::None), at);
    assert_eq!(code(unchanged.unwrap_err()), "EMPLOYER_REPORT_UNCHANGED");
    let later = t("2026-11-03T08:00:00Z");
    let r = e
        .preview_employer_report(replace(MonthPayments::TaxFreeReimbursementsOnly), later)
        .unwrap();
    assert_eq!(
        r.report.action,
        EmployerReportAction::Replacement {
            ir_report_id: IR_REPORT.into(),
            replaces_version: Some(1)
        }
    );
    let status = e
        .submit_employer_report(
            replace(MonthPayments::TaxFreeReimbursementsOnly),
            later,
            &r.record_hash,
            &b,
            later,
        )
        .unwrap();
    assert_eq!(status.state, EmployerMonthState::Accepted);
    assert_eq!(status.current.unwrap().version, Some(2));
    assert_eq!(
        status.runs.iter().map(|r| r.sequence).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(s.sends.load(Ordering::SeqCst), 2);

    // Changing pension details is not a replacement under the register's rules.
    let h = home(json!([{"provider_code":46,"policy_number":"12-3456789"}]));
    let e = ProcessEngine::open(h.path());
    let (b, _) = broker(h.path(), vec![Answer::Accept(1)], Answer::Accept(1));
    let p = e
        .preview_employer_report(input("2026-10", MonthPayments::None), at)
        .unwrap();
    e.submit_employer_report(
        input("2026-10", MonthPayments::None),
        at,
        &p.record_hash,
        &b,
        at,
    )
    .unwrap();
    let profile = h.path().join(".aggountant/tulorekisteri.json");
    let changed = std::fs::read_to_string(&profile)
        .unwrap()
        .replace("12-3456789", "98-7654321");
    std::fs::write(&profile, changed).unwrap();
    let r = e.preview_employer_report(replace(MonthPayments::None), later);
    assert_eq!(
        code(r.unwrap_err()),
        "EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED"
    );
    // Moving from "no wages payable" with a pension policy to a contribution is
    // a replacement; it repeats the previous version's policy, as the register's
    // processing rule requires, not the changed profile.
    let r = e
        .preview_employer_report(replace(MonthPayments::TaxFreeReimbursementsOnly), later)
        .unwrap();
    assert_eq!(
        r.report.sent_pension_insurances(),
        [PensionInsurance {
            provider_code: 46,
            policy_number: "12-3456789".into()
        }]
    );
}

#[test]
fn e_service_evidence_settles_a_replacement_only_with_a_newer_version() {
    let h = home(json!([]));
    let e = ProcessEngine::open(h.path());
    let at = t("2026-11-02T08:00:00Z");
    let later = t("2026-11-03T08:00:00Z");
    let (b, s) = broker(
        h.path(),
        vec![Answer::Accept(1), Answer::Unknown],
        Answer::Unknown,
    );
    let p = e
        .preview_employer_report(input("2026-10", MonthPayments::None), at)
        .unwrap();
    e.submit_employer_report(
        input("2026-10", MonthPayments::None),
        at,
        &p.record_hash,
        &b,
        at,
    )
    .unwrap();
    let replace = EmployerReportInput {
        replace: true,
        ..input("2026-10", MonthPayments::TaxFreeReimbursementsOnly)
    };
    let r = e.preview_employer_report(replace, later).unwrap();
    let status = e
        .submit_employer_report(replace, later, &r.record_hash, &b, later)
        .unwrap();
    assert_eq!(status.state, EmployerMonthState::OutcomeUnknown);
    let evidence = |version| EmployerReportReceipt {
        ir_delivery_id: None,
        ir_report_id: IR_REPORT.into(),
        version,
        source: "e_service".into(),
    };
    // The e-service still showing version 1 (or no version) proves nothing about
    // the replacement; the month stays parked.
    for stale in [None, Some(1)] {
        let r = e.complete_employer_report_manual(month("2026-10"), evidence(stale), later);
        assert_eq!(code(r.unwrap_err()), "EMPLOYER_REPORT_INVALID_EVIDENCE");
        assert!(e.employer_report_in_doubt(month("2026-10")).unwrap());
    }
    let status = e
        .complete_employer_report_manual(month("2026-10"), evidence(Some(2)), later)
        .unwrap();
    assert_eq!(status.state, EmployerMonthState::Accepted);
    assert_eq!(status.current.unwrap().version, Some(2));
    assert_eq!(s.sends.load(Ordering::SeqCst), 2);
}

#[test]
fn e_service_evidence_cannot_settle_a_replacement_of_an_unknown_version() {
    let h = home(json!([]));
    let e = ProcessEngine::open(h.path());
    let at = t("2026-11-02T08:00:00Z");
    let later = t("2026-11-03T08:00:00Z");
    let (b, _) = broker(h.path(), vec![Answer::Unknown], Answer::Unknown);
    // The month was given in the e-service and recorded without its version.
    e.complete_employer_report_manual(
        month("2026-10"),
        EmployerReportReceipt {
            ir_delivery_id: None,
            ir_report_id: IR_REPORT.into(),
            version: None,
            source: "e_service".into(),
        },
        at,
    )
    .unwrap();
    let replace = EmployerReportInput {
        replace: true,
        ..input("2026-10", MonthPayments::TaxFreeReimbursementsOnly)
    };
    let r = e.preview_employer_report(replace, later).unwrap();
    e.submit_employer_report(replace, later, &r.record_hash, &b, later)
        .unwrap();
    for version in [Some(2), Some(5)] {
        let r = e.complete_employer_report_manual(
            month("2026-10"),
            EmployerReportReceipt {
                ir_delivery_id: None,
                ir_report_id: IR_REPORT.into(),
                version,
                source: "e_service".into(),
            },
            later,
        );
        assert_eq!(code(r.unwrap_err()), "EMPLOYER_REPORT_INVALID_EVIDENCE");
    }
    assert!(e.employer_report_in_doubt(month("2026-10")).unwrap());
}

#[test]
fn missing_months_list_unreported_months_with_deadlines_and_write_nothing() {
    let h = home(json!([]));
    let e = ProcessEngine::open(h.path());
    let (b, _) = broker(h.path(), vec![Answer::Accept(1)], Answer::Accept(1));
    let at = t("2026-09-03T08:00:00Z");
    let p = e
        .preview_employer_report(input("2026-08", MonthPayments::None), at)
        .unwrap();
    e.submit_employer_report(
        input("2026-08", MonthPayments::None),
        at,
        &p.record_hash,
        &b,
        at,
    )
    .unwrap();
    // July was given in the e-service before aggountant was used.
    e.complete_employer_report_manual(
        month("2026-07"),
        EmployerReportReceipt {
            ir_delivery_id: Some(IR_DELIVERY.into()),
            ir_report_id: "33333333222233334444555555555555".into(),
            version: None,
            source: "e_service".into(),
        },
        at,
    )
    .unwrap();
    let before = std::fs::read_dir(h.path().join("process/runs"))
        .unwrap()
        .count();
    let m = e
        .missing_employer_reports(NaiveDate::from_ymd_opt(2026, 11, 3).unwrap())
        .unwrap();
    let got: Vec<_> = m
        .months
        .iter()
        .map(|m| {
            (
                m.month.to_string(),
                m.deadline.to_string(),
                m.overdue,
                m.state,
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            (
                "2026-09".to_string(),
                "2026-10-05".to_string(),
                true,
                EmployerMonthState::NotFiled
            ),
            (
                "2026-10".to_string(),
                "2026-11-05".to_string(),
                false,
                EmployerMonthState::NotFiled
            ),
        ]
    );
    assert_eq!(
        std::fs::read_dir(h.path().join("process/runs"))
            .unwrap()
            .count(),
        before
    );
    // Recording July again, or September evidence for an accepted month, is refused.
    let again = e.complete_employer_report_manual(
        month("2026-08"),
        EmployerReportReceipt {
            ir_delivery_id: None,
            ir_report_id: IR_REPORT.into(),
            version: None,
            source: "e_service".into(),
        },
        at,
    );
    assert_eq!(code(again.unwrap_err()), "EMPLOYER_REPORT_ALREADY_FILED");
    let bad = e.complete_employer_report_manual(
        month("2026-09"),
        EmployerReportReceipt {
            ir_delivery_id: None,
            ir_report_id: "not-a-reference".into(),
            version: None,
            source: "e_service".into(),
        },
        at,
    );
    assert_eq!(code(bad.unwrap_err()), "EMPLOYER_REPORT_INVALID_EVIDENCE");
}

#[test]
fn unconfigured_company_is_told_what_to_set() {
    let h = tempfile::tempdir().unwrap();
    let e = ProcessEngine::open(h.path());
    let r = e.preview_employer_report(
        input("2026-10", MonthPayments::None),
        t("2026-11-02T08:00:00Z"),
    );
    assert_eq!(code(r.unwrap_err()), "EMPLOYER_REPORT_NOT_CONFIGURED");
    let r = e.missing_employer_reports(NaiveDate::from_ymd_opt(2026, 11, 2).unwrap());
    assert_eq!(code(r.unwrap_err()), "EMPLOYER_REPORT_NOT_CONFIGURED");
}
