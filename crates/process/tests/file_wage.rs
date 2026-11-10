use aggountant_broker::{
    tulorekisteri::{SVC_STATUS, SVC_WAGE},
    *,
};
use aggountant_domain::{TaxCard, WageReport};
use aggountant_ledger::{EmployeeState, Ledger};
use aggountant_process::{ProcessEngine, RunStatus};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
fn report() -> WageReport {
    serde_json::from_value(json!({"business_id":"1234567-1","source_reference":"test-expense","employee_id":"emp-1","payment_date":"2026-10-25","period_start":"2026-09-01","period_end":"2026-09-30","joint_owner":true,"contact_name":"Fictional Contact","contact_phone":"+358 40 1234567","lines":[{"income_type":"311","amount":"120.00","kilometers":200},{"income_type":"331","amount":"54.00","allowance_code":3}]})).unwrap()
}
fn time() -> DateTime<Utc> {
    "2026-10-25T12:00:00Z".parse().unwrap()
}
fn employee(hetu: &str) -> EmployeeState {
    EmployeeState {
        employee_id: "emp-1".into(),
        name: "Fictional Employee".into(),
        hetu: hetu.into(),
        iban: "FI2112345600000785".into(),
        tax_card: TaxCard {
            withholding_pct: "0".into(),
        },
    }
}
fn home() -> tempfile::TempDir {
    let h = tempfile::tempdir().unwrap();
    let l = Ledger::init(h.path()).unwrap();
    l.upsert_employee(&employee("010101A123N")).unwrap();
    h
}
struct Mock {
    status: bool,
    unknown: bool,
    reject_status: bool,
    calls: Arc<AtomicUsize>,
}
impl RequestResponse for Mock {
    fn service(&self) -> &str {
        if self.status {
            SVC_STATUS
        } else {
            SVC_WAGE
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
    fn call(&self, _: &Envelope, _: &CredentialHandle) -> Result<AdapterResponse, AdapterError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.status && self.reject_status {
            return Err(AdapterError::Rejected {
                code: "TULOREKISTERI_REJECTED".into(),
                message: "safe structured rejection".into(),
            });
        }
        if self.unknown && !self.status {
            return Err(AdapterError::Timeout("test timeout".into()));
        }
        Ok(AdapterResponse::Accepted {
            receipt: "11111111222233334444555555555555/22222222222233334444555555555555/1".into(),
            body: json!({"ir_delivery_id":"11111111222233334444555555555555","ir_report_id":"22222222222233334444555555555555","version":1}),
        })
    }
}
fn broker(h: &std::path::Path, unknown: bool) -> (Broker, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    broker_with_status(h, unknown, false)
}
fn broker_with_status(
    h: &std::path::Path,
    unknown: bool,
    reject_status: bool,
) -> (Broker, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let mut v = Vault::new();
    let mut r = Registry::new();
    let send = Arc::new(AtomicUsize::new(0));
    let read = Arc::new(AtomicUsize::new(0));
    for (status, calls) in [(false, send.clone()), (true, read.clone())] {
        let a = Mock {
            status,
            unknown,
            reject_status,
            calls,
        };
        v.set(a.service(), "test");
        r.register(Box::new(a));
    }
    (Broker::new(h, v, r), send, read)
}
#[test]
fn offline_preview_start_and_approve_once_with_pii_isolation() {
    let h = home();
    let e = ProcessEngine::open(h.path());
    let r = report();
    let p = e.preview_wage(&r, time()).unwrap();
    assert_eq!(p.deadline, "2026-10-30");
    assert!(p.report.joint_owner);
    assert_eq!(p.report.period_start.to_string(), "2026-09-01");
    let preview = serde_json::to_string(&p).unwrap();
    assert!(!preview.contains("010101A123N"));
    assert!(!h.path().join("process").exists());
    let run = e.start_wage(&r, time()).unwrap();
    assert_eq!(run.status, RunStatus::AwaitingGate);
    let (b, send, _) = broker(h.path(), false);
    assert!(e.approve_wage(&run.run_id, "wrong", &b).is_err());
    assert_eq!(send.load(Ordering::SeqCst), 0);
    assert_eq!(
        e.approve_wage(&run.run_id, &p.record_hash, &b)
            .unwrap()
            .status,
        RunStatus::Completed
    );
    e.approve_wage(&run.run_id, &p.record_hash, &b).unwrap();
    assert_eq!(send.load(Ordering::SeqCst), 1);
    let sidecar = std::fs::read_to_string(
        h.path()
            .join("process/runs")
            .join(&run.run_id)
            .join("wage.json"),
    )
    .unwrap();
    assert!(!sidecar.contains("010101A123N"));
    assert!(!sidecar.contains(&p.record_hash));
    assert_ne!(run.gates[0].snapshot_hash, p.record_hash);
    let events = serde_json::to_string(&e.journal(&run.run_id).unwrap()).unwrap();
    assert!(!events.contains(&p.record_hash));
    assert_eq!(e.start_wage(&r, time()).unwrap().run_id, run.run_id);
}
#[test]
fn unknown_is_reconcile_only_across_restart_and_reapproval() {
    let h = home();
    let e = ProcessEngine::open(h.path());
    let r = report();
    let p = e.preview_wage(&r, time()).unwrap();
    let run = e.start_wage(&r, time()).unwrap();
    let (b, send, read) = broker(h.path(), true);
    assert_eq!(
        e.approve_wage(&run.run_id, &p.record_hash, &b)
            .unwrap()
            .status,
        RunStatus::NeedsAttention
    );
    let e = ProcessEngine::open(h.path());
    e.approve_wage(&run.run_id, &p.record_hash, &b).unwrap();
    assert_eq!(send.load(Ordering::SeqCst), 1);
    assert_eq!(
        e.reconcile_wage(&run.run_id, &b, time()).unwrap().status,
        RunStatus::Completed
    );
    assert_eq!(send.load(Ordering::SeqCst), 1);
    assert_eq!(read.load(Ordering::SeqCst), 1);
}
#[test]
fn pii_mutation_and_payload_mutation_refused() {
    let h = home();
    let e = ProcessEngine::open(h.path());
    let mut r = report();
    let p = e.preview_wage(&r, time()).unwrap();
    let run = e.start_wage(&r, time()).unwrap();
    r.joint_owner = false;
    assert!(e.start_wage(&r, time()).is_err());
    Ledger::open(h.path())
        .unwrap()
        .upsert_employee(&employee("131052-308T"))
        .unwrap();
    let (b, send, _) = broker(h.path(), false);
    assert!(e.approve_wage(&run.run_id, &p.record_hash, &b).is_err());
    assert_eq!(send.load(Ordering::SeqCst), 0);
    // A fresh explicit preview authorizes corrected PII before any dispatch;
    // there is no need to persist a HETU-verifiable digest in git to enforce this.
    let fresh = e.preview_wage(&report(), time()).unwrap();
    assert_eq!(
        e.approve_wage(&run.run_id, &fresh.record_hash, &b)
            .unwrap()
            .status,
        RunStatus::Completed
    );
    assert_eq!(send.load(Ordering::SeqCst), 1);
}
#[test]
fn invalid_inputs_do_not_create_runs_and_manual_requires_guid_evidence() {
    let h = home();
    let e = ProcessEngine::open(h.path());
    let mut r = report();
    r.period_end = aggountant_model::Date::parse("2026-08-01").unwrap();
    assert!(e.start_wage(&r, time()).is_err());
    let r = report();
    let run = e.start_wage(&r, time()).unwrap();
    assert!(e.complete_wage_manual(&run.run_id, "", "text").is_err());
    assert_eq!(
        e.complete_wage_manual(
            &run.run_id,
            "11111111222233334444555555555555",
            "22222222222233334444555555555555"
        )
        .unwrap()
        .status,
        RunStatus::Completed
    );
    assert!(e.complete_wage_manual("../other", "", " ").is_err());
}

#[test]
fn crash_after_broker_success_recovers_without_network() {
    let h = home();
    let e = ProcessEngine::open(h.path());
    let r = report();
    let p = e.preview_wage(&r, time()).unwrap();
    let run = e.start_wage(&r, time()).unwrap();
    let (b, send, read) = broker(h.path(), false);
    e.approve_wage(&run.run_id, &p.record_hash, &b).unwrap();
    // Crash before the final journal append: retain outbox success, remove ONLY
    // the tail event in this disposable home, leaving a valid hash chain.
    let dir = h
        .path()
        .join("process/runs")
        .join(&run.run_id)
        .join("events");
    let mut files = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    files.sort();
    std::fs::remove_file(files.last().unwrap()).unwrap();
    assert_eq!(
        e.reconcile_wage(&run.run_id, &b, time()).unwrap().status,
        RunStatus::Completed
    );
    assert_eq!(send.load(Ordering::SeqCst), 1);
    assert_eq!(read.load(Ordering::SeqCst), 0);
}

#[test]
fn status_and_manual_completion_survive_pii_correction_and_erasure() {
    for manual in [false, true] {
        let h = home();
        let e = ProcessEngine::open(h.path());
        let r = report();
        let p = e.preview_wage(&r, time()).unwrap();
        let run = e.start_wage(&r, time()).unwrap();
        let (b, send, _) = broker(h.path(), true);
        e.approve_wage(&run.run_id, &p.record_hash, &b).unwrap();
        // No open Ledger connection survives each engine step. Delete the entire
        // synthetic PII DB to prove these operations do not even reopen it.
        std::fs::remove_file(h.path().join("masterdata/pii.sqlite")).unwrap();
        let view = if manual {
            e.complete_wage_manual(
                &run.run_id,
                "11111111222233334444555555555555",
                "22222222222233334444555555555555",
            )
            .unwrap()
        } else {
            e.reconcile_wage(&run.run_id, &b, time()).unwrap()
        };
        assert_eq!(view.status, RunStatus::Completed);
        assert_eq!(send.load(Ordering::SeqCst), 1);
        assert!(!h.path().join("masterdata/pii.sqlite").exists());
    }
}
#[test]
fn definitive_status_rejection_is_journalled_not_described_as_pending() {
    let h = home();
    let e = ProcessEngine::open(h.path());
    let r = report();
    let p = e.preview_wage(&r, time()).unwrap();
    let run = e.start_wage(&r, time()).unwrap();
    let (b, send, read) = broker_with_status(h.path(), true, true);
    e.approve_wage(&run.run_id, &p.record_hash, &b).unwrap();
    assert_eq!(
        e.reconcile_wage(&run.run_id, &b, time()).unwrap().status,
        RunStatus::NeedsAttention
    );
    let events = e.journal(&run.run_id).unwrap();
    assert!(
        matches!(&events.last().unwrap().payload, aggountant_process::EventPayload::EffectRejected {error_code,..} if error_code=="TULOREKISTERI_REJECTED")
    );
    assert_eq!(send.load(Ordering::SeqCst), 1);
    assert_eq!(read.load(Ordering::SeqCst), 1);
}
