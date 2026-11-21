//! Corrections of an accepted travel report: replacement and cancellation go
//! through the same preview, approval, outbox and reconcile-only rules as the
//! report, and e-service evidence is recorded or refused, never dropped.
use aggountant_broker::{
    tulorekisteri::{
        WageCorrectionRequest, SVC_STATUS, SVC_WAGE, SVC_WAGE_CORRECTION,
        SVC_WAGE_CORRECTION_STATUS,
    },
    *,
};
use aggountant_domain::{TaxCard, TravelLine, WageCorrectionAction, WageReport};
use aggountant_ledger::{EmployeeState, Ledger};
use aggountant_model::Instant;
use aggountant_process::{
    AuthorityOutcome, EmployerReportReceipt, EmployerRunOutcome, ProcessEngine, ProcessError,
    WageCorrectionInput, WageFilingKind, WageReportState,
};
use serde_json::json;
use std::sync::{Arc, Mutex};

const HETU: &str = "010101A123N";
const IR_DELIVERY: &str = "11111111222233334444555555555555";
const IR_REPORT: &str = "22222222222233334444555555555555";
const IR_CORRECTION_DELIVERY: &str = "33333333222233334444555555555555";

fn report() -> WageReport {
    serde_json::from_value(json!({"business_id":"1234567-1","source_reference":"test-expense","employee_id":"emp-1","payment_date":"2026-10-25","period_start":"2026-09-01","period_end":"2026-09-30","joint_owner":true,"contact_name":"Fictional Contact","contact_phone":"+358 40 1234567","lines":[{"income_type":"311","amount":"120.00","kilometers":200}]})).unwrap()
}
/// The same report with the kilometres corrected.
fn corrected() -> WageReport {
    let mut r = report();
    r.lines = vec![TravelLine::Kilometres {
        amount: "96.00".parse().unwrap(),
        kilometers: 160,
    }];
    r
}
fn time() -> Instant {
    "2026-10-25T12:00:00Z".parse().unwrap()
}
fn later() -> Instant {
    "2026-11-02T08:00:00Z".parse().unwrap()
}
fn replace(report: WageReport) -> WageCorrectionInput {
    WageCorrectionInput {
        action: WageCorrectionAction::Replacement,
        report: Some(report),
    }
}
fn cancel() -> WageCorrectionInput {
    WageCorrectionInput {
        action: WageCorrectionAction::Cancellation,
        report: None,
    }
}
fn home() -> tempfile::TempDir {
    let h = tempfile::tempdir().unwrap();
    Ledger::init(h.path())
        .unwrap()
        .upsert_employee(&EmployeeState {
            employee_id: "emp-1".into(),
            name: "Fictional Employee".into(),
            hetu: HETU.into(),
            iban: "FI2112345600000785".into(),
            tax_card: TaxCard {
                withholding_pct: "0".into(),
            },
        })
        .unwrap();
    h
}

/// How the fake register answers a correction send and its status query.
#[derive(Clone, Copy, PartialEq)]
enum Answer {
    Accept,
    /// The send times out; the status query then answers `status`.
    Timeout {
        status: StatusAnswer,
    },
    Reject,
}
#[derive(Clone, Copy, PartialEq)]
enum StatusAnswer {
    Accepted,
    Rejected,
    Pending,
}
#[derive(Default)]
struct Seen {
    sends: Vec<WageCorrectionRequest>,
    status_queries: Vec<WageCorrectionRequest>,
}
struct Mock {
    service: &'static str,
    answer: Answer,
    seen: Arc<Mutex<Seen>>,
}
impl Mock {
    fn accepted(request: &WageCorrectionRequest) -> AdapterResponse {
        let version = request.correction.corrects_version.map_or(2, |v| v + 1);
        AdapterResponse::Accepted {
            receipt: format!("{IR_CORRECTION_DELIVERY}/{IR_REPORT}/{version}"),
            body: json!({"ir_delivery_id":IR_CORRECTION_DELIVERY,"ir_report_id":IR_REPORT,"version":version}),
        }
    }
}
impl RequestResponse for Mock {
    fn service(&self) -> &str {
        self.service
    }
    fn kind(&self) -> AdapterKind {
        if self.service == SVC_STATUS || self.service == SVC_WAGE_CORRECTION_STATUS {
            AdapterKind::Pull
        } else {
            AdapterKind::Submit
        }
    }
    fn assurance(&self) -> DeliverySemantics {
        DeliverySemantics::AtMostOnce
    }
    fn call(&self, e: &Envelope, _: &CredentialHandle) -> Result<AdapterResponse, AdapterError> {
        let rejected = || AdapterError::Rejected {
            code: "TULOREKISTERI_REJECTED".into(),
            message: "safe structured rejection".into(),
        };
        match self.service {
            // The report itself is always accepted as version 1.
            SVC_WAGE => Ok(AdapterResponse::Accepted {
                receipt: format!("{IR_DELIVERY}/{IR_REPORT}/1"),
                body: json!({}),
            }),
            SVC_WAGE_CORRECTION => {
                let request: WageCorrectionRequest =
                    serde_json::from_value(e.body.clone()).unwrap();
                assert_eq!(
                    e.natural_key,
                    request.correction.delivery_id(request.timestamp)
                );
                let answer = Self::accepted(&request);
                self.seen.lock().unwrap().sends.push(request);
                match self.answer {
                    Answer::Accept => Ok(answer),
                    Answer::Timeout { .. } => Err(AdapterError::Timeout("test timeout".into())),
                    Answer::Reject => Err(rejected()),
                }
            }
            SVC_WAGE_CORRECTION_STATUS => {
                let request: WageCorrectionRequest =
                    serde_json::from_value(e.body.clone()).unwrap();
                let answer = Self::accepted(&request);
                self.seen.lock().unwrap().status_queries.push(request);
                match self.answer {
                    Answer::Timeout {
                        status: StatusAnswer::Accepted,
                    } => Ok(answer),
                    Answer::Timeout {
                        status: StatusAnswer::Rejected,
                    } => Err(rejected()),
                    _ => Err(AdapterError::Timeout("pending".into())),
                }
            }
            _ => unreachable!("no status query of the report itself is expected"),
        }
    }
}
fn broker(h: &std::path::Path, answer: Answer) -> (Broker, Arc<Mutex<Seen>>) {
    let mut vault = Vault::new();
    let mut registry = Registry::new();
    let seen = Arc::new(Mutex::new(Seen::default()));
    for service in [
        SVC_WAGE,
        SVC_STATUS,
        SVC_WAGE_CORRECTION,
        SVC_WAGE_CORRECTION_STATUS,
    ] {
        vault.set(service, "test");
        registry.register(Box::new(Mock {
            service,
            answer,
            seen: seen.clone(),
        }));
    }
    (Broker::new(h, vault, registry), seen)
}
/// A home whose report was accepted through the API as version 1.
fn accepted(h: &std::path::Path) -> (ProcessEngine, String) {
    let e = ProcessEngine::open(h);
    let p = e.preview_wage(&report(), time()).unwrap();
    let run = e.start_wage(&report(), time()).unwrap();
    let (b, _) = broker(h, Answer::Accept);
    e.approve_wage(&run.run_id, &p.record_hash, &b).unwrap();
    (e, run.run_id)
}
fn code<T: std::fmt::Debug>(r: Result<T, ProcessError>) -> &'static str {
    r.unwrap_err().error_code()
}
fn sends(seen: &Arc<Mutex<Seen>>) -> usize {
    seen.lock().unwrap().sends.len()
}
fn run_files(h: &std::path::Path) -> String {
    fn walk(dir: &std::path::Path, out: &mut String) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push_str(&String::from_utf8_lossy(&std::fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = String::new();
    walk(&h.join("process"), &mut out);
    out
}

#[test]
fn manual_evidence_for_an_accepted_report_is_refused_not_dropped() {
    let h = home();
    let (e, run) = accepted(h.path());
    let before = e.journal(&run).unwrap().len();
    let err = e
        .complete_wage_manual(&run, IR_CORRECTION_DELIVERY, IR_REPORT)
        .unwrap_err();
    assert_eq!(err.error_code(), "WAGE_REPORT_ALREADY_ACCEPTED");
    assert_eq!(err.kind(), "user");
    let message = err.to_string();
    assert!(message.contains("nothing was recorded"), "{message}");
    assert!(message.contains("file-wage correction"), "{message}");
    assert_eq!(e.journal(&run).unwrap().len(), before);
    // The same holds for a report that was itself completed from e-service evidence.
    let h = home();
    let e = ProcessEngine::open(h.path());
    let run = e.start_wage(&report(), time()).unwrap().run_id;
    e.complete_wage_manual(&run, IR_DELIVERY, IR_REPORT)
        .unwrap();
    assert_eq!(
        code(e.complete_wage_manual(&run, IR_CORRECTION_DELIVERY, IR_REPORT)),
        "WAGE_REPORT_ALREADY_ACCEPTED"
    );
}

#[test]
fn replacement_is_previewed_approved_by_hash_and_sent_once() {
    let h = home();
    let (e, run) = accepted(h.path());
    let (b, seen) = broker(h.path(), Answer::Accept);
    let before = run_files(h.path());
    let preview = e
        .preview_wage_correction(&run, &replace(corrected()), later())
        .unwrap();
    assert_eq!(run_files(h.path()), before, "a preview writes nothing");
    assert_eq!(preview.ir_report_id, IR_REPORT);
    assert_eq!(preview.corrects_version, Some(1));
    assert_eq!(preview.previous_report, report());
    assert_eq!(preview.report, Some(corrected()));
    assert!(preview.sensitive.is_some());
    assert!(preview.warnings.is_empty());
    assert!(!serde_json::to_string(&preview).unwrap().contains(HETU));

    // The approval names the previewed record; anything else sends nothing.
    assert_eq!(
        code(e.submit_wage_correction(&run, &replace(corrected()), later(), "wrong", &b, later())),
        "WAGE_CORRECTION_HASH_MISMATCH"
    );
    let mut other = corrected();
    other.contact_name = serde_json::from_value(json!("Another Contact")).unwrap();
    assert_eq!(
        code(e.submit_wage_correction(
            &run,
            &replace(other),
            later(),
            &preview.record_hash,
            &b,
            later()
        )),
        "WAGE_CORRECTION_HASH_MISMATCH"
    );
    let stale: Instant = "2026-11-04T08:00:00Z".parse().unwrap();
    assert_eq!(
        code(e.submit_wage_correction(
            &run,
            &replace(corrected()),
            later(),
            &preview.record_hash,
            &b,
            stale
        )),
        "WAGE_CORRECTION_STALE_TIMESTAMP"
    );
    assert_eq!(sends(&seen), 0);
    assert_eq!(run_files(h.path()), before);

    let status = e
        .submit_wage_correction(
            &run,
            &replace(corrected()),
            later(),
            &preview.record_hash,
            &b,
            later(),
        )
        .unwrap();
    assert_eq!(status.state, WageReportState::Replaced);
    assert_eq!(status.report, corrected());
    assert!(status.report_is_current);
    let current = status.current.clone().unwrap();
    assert_eq!(current.ir_report_id, IR_REPORT);
    assert_eq!(current.version, Some(2));
    assert_eq!(
        current.ir_delivery_id.as_deref(),
        Some(IR_CORRECTION_DELIVERY)
    );
    assert_eq!(status.filings.len(), 2);
    assert_eq!(status.filings[0].kind, WageFilingKind::Report);
    assert_eq!(status.filings[1].kind, WageFilingKind::Replacement);
    assert_eq!(status.filings[1].sequence, 2);
    assert!(status.filings[1].run_id.starts_with("wc-"));
    {
        let seen = seen.lock().unwrap();
        assert_eq!(seen.sends.len(), 1);
        let sent = &seen.sends[0];
        assert_eq!(sent.hetu, HETU);
        assert_eq!(sent.correction.report, corrected());
        assert_eq!(sent.correction.ir_report_id, IR_REPORT);
        assert_eq!(sent.correction.corrects_version, Some(1));
        assert_eq!(sent.status_timestamp, None);
        // Its own record reference, never the first record's.
        assert_ne!(
            sent.correction.delivery_id(sent.timestamp),
            report().delivery_id()
        );
    }
    // Process state holds neither the identity code nor the hash derived from it.
    let files = run_files(h.path());
    assert!(!files.contains(HETU));
    assert!(!files.contains(&preview.record_hash));

    // Repeating the approved submit never sends again.
    assert_eq!(
        code(e.submit_wage_correction(
            &run,
            &replace(corrected()),
            later(),
            &preview.record_hash,
            &b,
            later()
        )),
        "WAGE_CORRECTION_UNCHANGED"
    );
    assert_eq!(sends(&seen), 1);

    // `run show` of the report no longer presents its first receipt as current.
    let shown = e.show(&run).unwrap();
    let Some(AuthorityOutcome::Superseded {
        receipt,
        next_command,
        message,
    }) = &shown.effects[0].outcome
    else {
        panic!("{:?}", shown.effects[0].outcome)
    };
    assert!(receipt.is_some());
    assert_eq!(
        *next_command,
        format!("aggountant file-wage correction status {run}")
    );
    assert!(message.contains("replaced"), "{message}");
    // The correction Run shows its own receipt.
    let correction = e.show(&status.filings[1].run_id).unwrap();
    assert!(matches!(
        correction.effects[0].outcome,
        Some(AuthorityOutcome::Accepted {
            receipt: Some(_),
            ..
        })
    ));

    // A further correction acts on version 2.
    let again = e.preview_wage_correction(&run, &cancel(), later()).unwrap();
    assert_eq!(again.corrects_version, Some(2));
    assert_eq!(again.previous_report, corrected());
    assert!(again.sensitive.is_none() && again.report.is_none());
}

#[test]
fn replacement_cannot_change_what_the_register_forbids_or_repeat_the_report() {
    let h = home();
    let (e, run) = accepted(h.path());
    assert_eq!(
        code(e.preview_wage_correction(&run, &replace(report()), later())),
        "WAGE_CORRECTION_UNCHANGED"
    );
    let mut period = corrected();
    period.period_end = aggountant_model::Date::parse("2026-09-29").unwrap();
    let mut date = corrected();
    date.payment_date = aggountant_model::Date::parse("2026-10-26").unwrap();
    let mut earner = corrected();
    earner.employee_id = "emp-2".into();
    for forbidden in [period, date, earner] {
        let err = e
            .preview_wage_correction(&run, &replace(forbidden), later())
            .unwrap_err();
        assert_eq!(err.error_code(), "WAGE_CORRECTION_REPLACEMENT_NOT_ALLOWED");
        assert!(err.to_string().contains("--action cancel"), "{err}");
    }
    let mut invalid = corrected();
    invalid.lines.clear();
    assert_eq!(
        code(e.preview_wage_correction(&run, &replace(invalid), later())),
        "WAGE_CORRECTION_INVALID"
    );
    for (action, report) in [
        (WageCorrectionAction::Replacement, None),
        (WageCorrectionAction::Cancellation, Some(corrected())),
    ] {
        assert_eq!(
            code(e.preview_wage_correction(&run, &WageCorrectionInput { action, report }, later())),
            "WAGE_CORRECTION_INVALID"
        );
    }
    assert!(e
        .preview_wage_correction("../other", &cancel(), later())
        .is_err());
}

#[test]
fn only_an_accepted_report_is_corrected() {
    let h = home();
    let e = ProcessEngine::open(h.path());
    let run = e.start_wage(&report(), time()).unwrap().run_id;
    assert_eq!(
        e.wage_report_status(&run).unwrap().state,
        WageReportState::Pending
    );
    assert_eq!(
        code(e.preview_wage_correction(&run, &cancel(), later())),
        "WAGE_CORRECTION_NOT_ACCEPTED"
    );
    assert_eq!(
        code(e.complete_wage_correction_manual(
            &run,
            WageCorrectionAction::Cancellation,
            evidence(Some(2))
        )),
        "WAGE_CORRECTION_NOT_ACCEPTED"
    );
}

#[test]
fn cancellation_is_sent_without_pii_and_ends_the_report() {
    let h = home();
    let (e, run) = accepted(h.path());
    // A cancellation needs no personal data: it works after the PII store is gone.
    std::fs::remove_file(h.path().join("masterdata/pii.sqlite")).unwrap();
    let (b, seen) = broker(h.path(), Answer::Accept);
    let preview = e.preview_wage_correction(&run, &cancel(), later()).unwrap();
    let status = e
        .submit_wage_correction(&run, &cancel(), later(), &preview.record_hash, &b, later())
        .unwrap();
    assert_eq!(status.state, WageReportState::Cancelled);
    assert_eq!(status.current, None);
    assert_eq!(status.filings[1].kind, WageFilingKind::Cancellation);
    {
        let seen = seen.lock().unwrap();
        assert_eq!(seen.sends.len(), 1);
        assert_eq!(seen.sends[0].hetu, "");
        assert_eq!(
            seen.sends[0].correction.action,
            WageCorrectionAction::Cancellation
        );
    }
    let Some(AuthorityOutcome::Superseded { message, .. }) =
        &e.show(&run).unwrap().effects[0].outcome
    else {
        panic!("a cancelled report must not read as accepted")
    };
    assert!(message.contains("cancelled"), "{message}");
    // Nothing is left to correct, by any route.
    for attempt in [
        e.preview_wage_correction(&run, &cancel(), later())
            .map(|_| ()),
        e.preview_wage_correction(&run, &replace(corrected()), later())
            .map(|_| ()),
        e.complete_wage_correction_manual(
            &run,
            WageCorrectionAction::Cancellation,
            evidence(Some(3)),
        )
        .map(|_| ()),
    ] {
        assert_eq!(code(attempt), "WAGE_CORRECTION_REPORT_CANCELLED");
    }
    assert_eq!(sends(&seen), 1);
}

#[test]
fn unknown_outcome_blocks_every_send_until_the_status_query_settles_it() {
    for status_answer in [StatusAnswer::Accepted, StatusAnswer::Rejected] {
        let h = home();
        let (e, run) = accepted(h.path());
        let (b, seen) = broker(
            h.path(),
            Answer::Timeout {
                status: StatusAnswer::Pending,
            },
        );
        let preview = e
            .preview_wage_correction(&run, &replace(corrected()), later())
            .unwrap();
        let submit = |b: &Broker| {
            e.submit_wage_correction(
                &run,
                &replace(corrected()),
                later(),
                &preview.record_hash,
                b,
                later(),
            )
        };
        let status = submit(&b).unwrap();
        assert_eq!(status.state, WageReportState::CorrectionOutcomeUnknown);
        assert_eq!(
            status.filings[1].outcome,
            EmployerRunOutcome::OutcomeUnknown
        );
        // The first receipt is still the last known, but it is not presented as settled.
        assert!(matches!(
            e.show(&run).unwrap().effects[0].outcome,
            Some(AuthorityOutcome::Superseded { .. })
        ));
        let correction_run = status.filings[1].run_id.clone();
        let Some(AuthorityOutcome::OutcomeUnknown { next_command, .. }) =
            &e.show(&correction_run).unwrap().effects[0].outcome
        else {
            panic!("the correction Run must read as unknown")
        };
        assert_eq!(
            next_command.as_deref(),
            Some(format!("aggountant file-wage correction status {run}").as_str())
        );
        assert!(e.wage_correction_in_doubt(&run).unwrap());

        // Neither the same correction, a different one, nor another action is sent.
        assert_eq!(code(submit(&b)), "WAGE_CORRECTION_OUTCOME_UNKNOWN");
        let other_time: Instant = "2026-11-02T09:00:00Z".parse().unwrap();
        assert_eq!(
            code(e.preview_wage_correction(&run, &cancel(), other_time)),
            "WAGE_CORRECTION_OUTCOME_UNKNOWN"
        );
        assert_eq!(sends(&seen), 1);

        // A status query without a definitive answer leaves it parked.
        let err = e.reconcile_wage_correction(&run, &b, later()).unwrap_err();
        assert_eq!(err.error_code(), "WAGE_CORRECTION_STATUS_UNKNOWN");
        assert_eq!(err.kind(), "system");
        assert!(e.wage_correction_in_doubt(&run).unwrap());

        // A definitive status answer settles it. The query carries no identity
        // code and works after the PII store is gone.
        std::fs::remove_file(h.path().join("masterdata/pii.sqlite")).unwrap();
        let (b, seen) = broker(
            h.path(),
            Answer::Timeout {
                status: status_answer,
            },
        );
        let settled = e.reconcile_wage_correction(&run, &b, later()).unwrap();
        {
            let seen = seen.lock().unwrap();
            assert!(seen.sends.is_empty(), "reconcile never sends");
            assert_eq!(seen.status_queries.len(), 1);
            assert_eq!(seen.status_queries[0].hetu, "");
            assert_eq!(seen.status_queries[0].status_timestamp, Some(later()));
        }
        match status_answer {
            StatusAnswer::Accepted => {
                assert_eq!(settled.state, WageReportState::Replaced);
                assert_eq!(settled.current.unwrap().version, Some(2));
            }
            _ => {
                // The correction did not arrive: the report stands as it was, and
                // a new correction may be made.
                assert_eq!(settled.state, WageReportState::Accepted);
                assert_eq!(settled.current.unwrap().version, Some(1));
                assert!(matches!(
                    settled.filings[1].outcome,
                    EmployerRunOutcome::Rejected { .. }
                ));
                assert!(matches!(
                    e.show(&run).unwrap().effects[0].outcome,
                    Some(AuthorityOutcome::Accepted { .. })
                ));
            }
        }
        // Settled: a further status call is offline and changes nothing.
        e.reconcile_wage_correction(&run, &b, later()).unwrap();
        assert_eq!(seen.lock().unwrap().status_queries.len(), 1);
    }
}

#[test]
fn definitive_rejection_allows_a_new_correction_with_a_new_record() {
    let h = home();
    let (e, run) = accepted(h.path());
    let (b, seen) = broker(h.path(), Answer::Reject);
    let preview = e
        .preview_wage_correction(&run, &replace(corrected()), later())
        .unwrap();
    let status = e
        .submit_wage_correction(
            &run,
            &replace(corrected()),
            later(),
            &preview.record_hash,
            &b,
            later(),
        )
        .unwrap();
    assert_eq!(status.state, WageReportState::Accepted);
    assert_eq!(status.current.unwrap().version, Some(1));
    // The rejected record is spent; the same approval cannot send it again.
    assert_eq!(
        code(e.submit_wage_correction(
            &run,
            &replace(corrected()),
            later(),
            &preview.record_hash,
            &b,
            later()
        )),
        "WAGE_CORRECTION_ALREADY_SENT"
    );
    assert_eq!(sends(&seen), 1);
    let retry_time: Instant = "2026-11-02T09:00:00Z".parse().unwrap();
    let retry = e
        .preview_wage_correction(&run, &replace(corrected()), retry_time)
        .unwrap();
    assert_ne!(retry.delivery_id, preview.delivery_id);
    assert_ne!(retry.record_hash, preview.record_hash);
    let (b, _) = broker(h.path(), Answer::Accept);
    let status = e
        .submit_wage_correction(
            &run,
            &replace(corrected()),
            retry_time,
            &retry.record_hash,
            &b,
            retry_time,
        )
        .unwrap();
    assert_eq!(status.state, WageReportState::Replaced);
    assert_eq!(status.filings.len(), 3);
}

#[test]
fn crash_after_an_accepted_send_recovers_from_the_outbox_without_network() {
    let h = home();
    let (e, run) = accepted(h.path());
    let (b, seen) = broker(h.path(), Answer::Accept);
    let preview = e.preview_wage_correction(&run, &cancel(), later()).unwrap();
    let status = e
        .submit_wage_correction(&run, &cancel(), later(), &preview.record_hash, &b, later())
        .unwrap();
    // Crash before the final journal append: keep the outbox success and remove
    // only the tail event, leaving a valid hash chain.
    let dir = h
        .path()
        .join("process/runs")
        .join(&status.filings[1].run_id)
        .join("events");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    std::fs::remove_file(files.last().unwrap()).unwrap();
    assert_eq!(
        e.wage_report_status(&run).unwrap().state,
        WageReportState::CorrectionOutcomeUnknown
    );
    let settled = e.reconcile_wage_correction(&run, &b, later()).unwrap();
    assert_eq!(settled.state, WageReportState::Cancelled);
    let seen = seen.lock().unwrap();
    assert_eq!(seen.sends.len(), 1);
    assert!(seen.status_queries.is_empty());
}

fn evidence(version: Option<u32>) -> EmployerReportReceipt {
    EmployerReportReceipt {
        ir_delivery_id: None,
        ir_report_id: IR_REPORT.into(),
        version,
        source: "e_service".into(),
    }
}

#[test]
fn a_correction_made_in_the_e_service_is_recorded_from_evidence() {
    let h = home();
    let (e, run) = accepted(h.path());
    let manual = |action, receipt| e.complete_wage_correction_manual(&run, action, receipt);
    // Evidence must be the register's references, name this report, and be newer
    // than what is recorded.
    let mut malformed = evidence(Some(2));
    malformed.ir_report_id = "not-a-guid".into();
    let mut another = evidence(Some(2));
    another.ir_report_id = "99999999222233334444555555555555".into();
    for bad in [malformed, another, evidence(Some(1)), evidence(Some(0))] {
        assert_eq!(
            code(manual(WageCorrectionAction::Replacement, bad)),
            "WAGE_CORRECTION_INVALID_EVIDENCE"
        );
    }
    assert_eq!(e.wage_report_status(&run).unwrap().filings.len(), 1);

    let status = manual(WageCorrectionAction::Replacement, evidence(Some(2))).unwrap();
    assert_eq!(status.state, WageReportState::Replaced);
    let current = status.current.clone().unwrap();
    assert_eq!(current.version, Some(2));
    assert_eq!(current.source, "e_service");
    assert!(!status.filings[1].sent_by_aggountant);
    // aggountant no longer knows the content, and says so.
    assert!(!status.report_is_current);
    assert_eq!(status.report, report());
    assert!(matches!(
        e.show(&run).unwrap().effects[0].outcome,
        Some(AuthorityOutcome::Superseded { .. })
    ));
    // The same evidence again is not a newer version.
    assert_eq!(
        code(manual(WageCorrectionAction::Replacement, evidence(Some(2)))),
        "WAGE_CORRECTION_INVALID_EVIDENCE"
    );

    // A later API correction acts on the recorded version, warns that the content
    // changed outside, and may repeat the first report's content.
    let preview = e
        .preview_wage_correction(&run, &replace(report()), later())
        .unwrap();
    assert_eq!(preview.corrects_version, Some(2));
    assert!(
        preview.warnings[0].contains("e-service"),
        "{:?}",
        preview.warnings
    );

    let status = manual(WageCorrectionAction::Cancellation, evidence(None)).unwrap();
    assert_eq!(status.state, WageReportState::Cancelled);
    assert_eq!(status.current, None);
    assert_eq!(status.filings.len(), 3);
    assert_eq!(status.filings[2].sequence, 3);
    // Evidence is recorded without the identity code.
    assert!(!run_files(h.path()).contains(HETU));
}

#[test]
fn e_service_evidence_settles_an_unknown_correction_only_with_a_newer_version() {
    let h = home();
    let (e, run) = accepted(h.path());
    let (b, seen) = broker(
        h.path(),
        Answer::Timeout {
            status: StatusAnswer::Pending,
        },
    );
    let preview = e.preview_wage_correction(&run, &cancel(), later()).unwrap();
    e.submit_wage_correction(&run, &cancel(), later(), &preview.record_hash, &b, later())
        .unwrap();
    let manual = |action, receipt| e.complete_wage_correction_manual(&run, action, receipt);
    for (action, receipt) in [
        // Another action than the one in doubt.
        (WageCorrectionAction::Replacement, evidence(Some(2))),
        // The version that was being cancelled proves nothing.
        (WageCorrectionAction::Cancellation, evidence(Some(1))),
        (WageCorrectionAction::Cancellation, evidence(None)),
    ] {
        assert_eq!(
            code(manual(action, receipt)),
            "WAGE_CORRECTION_INVALID_EVIDENCE"
        );
    }
    assert!(e.wage_correction_in_doubt(&run).unwrap());
    let status = manual(WageCorrectionAction::Cancellation, evidence(Some(2))).unwrap();
    assert_eq!(status.state, WageReportState::Cancelled);
    assert_eq!(
        status.filings.len(),
        2,
        "the unsettled Run is settled, not doubled"
    );
    assert_eq!(sends(&seen), 1);
}

#[test]
fn the_same_unversioned_evidence_is_not_recorded_twice() {
    let h = home();
    let (e, run) = accepted(h.path());
    let manual = || {
        e.complete_wage_correction_manual(&run, WageCorrectionAction::Replacement, evidence(None))
    };
    let first = manual().unwrap();
    let correction_run = first.filings[1].run_id.clone();
    let journal = e.journal(&correction_run).unwrap().len();
    let err = manual().unwrap_err();
    assert_eq!(err.error_code(), "WAGE_CORRECTION_INVALID_EVIDENCE");
    assert!(err.to_string().contains("--report-version"), "{err}");
    assert_eq!(e.journal(&correction_run).unwrap().len(), journal);
    let status = e.wage_report_status(&run).unwrap();
    assert_eq!(status.filings.len(), 2);
    assert_eq!(status.filings[1].sequence, 2);
}

#[test]
fn an_earlier_correction_run_is_superseded_by_a_later_one() {
    let h = home();
    let (e, run) = accepted(h.path());
    let (b, _) = broker(h.path(), Answer::Accept);
    let preview = e
        .preview_wage_correction(&run, &replace(corrected()), later())
        .unwrap();
    let status = e
        .submit_wage_correction(
            &run,
            &replace(corrected()),
            later(),
            &preview.record_hash,
            &b,
            later(),
        )
        .unwrap();
    let replacement_run = status.filings[1].run_id.clone();
    let outcome = |id: &str| e.show(id).unwrap().effects[0].outcome.clone();
    assert!(matches!(
        outcome(&replacement_run),
        Some(AuthorityOutcome::Accepted { .. })
    ));
    let preview = e.preview_wage_correction(&run, &cancel(), later()).unwrap();
    let status = e
        .submit_wage_correction(&run, &cancel(), later(), &preview.record_hash, &b, later())
        .unwrap();
    let Some(AuthorityOutcome::Superseded { next_command, .. }) = outcome(&replacement_run) else {
        panic!("the replaced version's receipt must not read as current")
    };
    assert_eq!(
        next_command,
        format!("aggountant file-wage correction status {run}")
    );
    // The cancellation is the last word and reads as accepted.
    assert!(matches!(
        outcome(&status.filings[2].run_id),
        Some(AuthorityOutcome::Accepted { .. })
    ));
}

#[test]
fn evidence_that_settles_a_sent_replacement_does_not_prove_its_content() {
    let h = home();
    let (e, run) = accepted(h.path());
    let (b, _) = broker(
        h.path(),
        Answer::Timeout {
            status: StatusAnswer::Pending,
        },
    );
    let preview = e
        .preview_wage_correction(&run, &replace(corrected()), later())
        .unwrap();
    e.submit_wage_correction(
        &run,
        &replace(corrected()),
        later(),
        &preview.record_hash,
        &b,
        later(),
    )
    .unwrap();
    let status = e
        .complete_wage_correction_manual(&run, WageCorrectionAction::Replacement, evidence(Some(2)))
        .unwrap();
    assert_eq!(status.state, WageReportState::Replaced);
    assert!(!status.report_is_current);
    // The content sent may not be what the register holds, so sending it is allowed
    // and the preview warns.
    let other_time: Instant = "2026-11-02T09:00:00Z".parse().unwrap();
    let again = e
        .preview_wage_correction(&run, &replace(corrected()), other_time)
        .unwrap();
    assert!(
        again.warnings[0].contains("e-service"),
        "{:?}",
        again.warnings
    );
}

#[test]
fn a_replacement_cannot_name_another_income_earner() {
    let h = home();
    let (e, run) = accepted(h.path());
    // The stored identity code is corrected after the report was accepted.
    let ledger = Ledger::open(h.path()).unwrap();
    let mut employee = ledger.get_employee("emp-1").unwrap();
    employee.hetu = "131052-308T".into();
    ledger.upsert_employee(&employee).unwrap();
    drop(ledger);
    let err = e
        .preview_wage_correction(&run, &replace(corrected()), later())
        .unwrap_err();
    assert_eq!(err.error_code(), "WAGE_CORRECTION_REPLACEMENT_NOT_ALLOWED");
    assert!(err.to_string().contains("identity code"), "{err}");
    assert!(!err.to_string().contains("131052"));
    // The way to change the income earner is to cancel, which names no person.
    e.preview_wage_correction(&run, &cancel(), later()).unwrap();

    // A report given in the e-service has no send to compare with: the preview
    // says so instead of refusing or staying silent.
    let h = home();
    let e = ProcessEngine::open(h.path());
    let run = e.start_wage(&report(), time()).unwrap().run_id;
    e.complete_wage_manual(&run, IR_DELIVERY, IR_REPORT)
        .unwrap();
    assert!(!h.path().join("integration").exists());
    let preview = e
        .preview_wage_correction(&run, &replace(corrected()), later())
        .unwrap();
    assert!(
        preview.warnings.iter().any(|w| w.contains("identity code")),
        "{:?}",
        preview.warnings
    );
    assert!(
        !h.path().join("integration").exists(),
        "a preview writes nothing"
    );
}
