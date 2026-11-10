//! F9 outbox **crash-safety** (`outbox-sqlite-crash-safe`). The M3 gate proved the
//! `Unknown`-then-recover path across a *timeout*; these tests prove it across a
//! crash *mid-send* — the durable pre-dispatch intent means a process that dies in
//! the dispatch→resolve window loses no effect and duplicates none on restart.

use std::panic::{catch_unwind, AssertUnwindSafe};

use serde_json::json;

use aggountant_broker::outbox::{AttemptOutcome, Outbox};
use aggountant_broker::{
    Broker, DeliverySemantics, Envelope, Provenance, RawRequest, RawResponse, Registry,
    RemoteOutcome, RetryPolicy, ScriptedTransport, SyntheticSubmitAdapter, Transport,
    TransportError, Vault, SVC_SYNTHETIC_SUBMIT,
};

fn vault_for(service: &str) -> Vault {
    let mut v = Vault::new();
    v.set(service, "secret");
    v
}

fn submit_broker(home: &std::path::Path, transport: Box<dyn Transport>) -> Broker {
    let mut reg = Registry::new();
    reg.register(Box::new(SyntheticSubmitAdapter::new(transport)));
    Broker::new(home, vault_for(SVC_SYNTHETIC_SUBMIT), reg)
}

fn filing_envelope(nk: &str) -> Envelope {
    Envelope::new(
        SVC_SYNTHETIC_SUBMIT,
        "submit-vat-return",
        nk,
        json!({"period": "2026-Q1", "net_vat": "1500.00"}),
    )
}

/// A transport that dies the instant it is called — the request bytes may already be
/// on the wire, exactly the "authority processed it but we never heard back" hazard.
struct PanicOnDispatch;
impl Transport for PanicOnDispatch {
    fn round_trip(&self, _req: &RawRequest) -> Result<RawResponse, TransportError> {
        panic!("simulated crash after the pre-dispatch intent was durably recorded");
    }
}

// --- The durable layer: an unresolved intent recovers as Unknown ------------

#[test]
fn a_crashed_intent_recovers_as_unknown() {
    let home = tempfile::tempdir().unwrap();
    let prov = Provenance {
        actor: "agent".into(),
        service: SVC_SYNTHETIC_SUBMIT.into(),
        operation: "submit-vat-return".into(),
        natural_key: "nk:1".into(),
        sent_hash: "HASH-1".into(),
        delivery_semantics: DeliverySemantics::AtMostOnce,
        attempt: 1,
        ts: "2026-08-07T00:00:00Z".into(),
    };

    // Record the pre-dispatch intent, then "crash" — never resolve it.
    {
        let outbox = Outbox::open(home.path()).unwrap();
        outbox
            .begin_attempt(SVC_SYNTHETIC_SUBMIT, "file", "nk:1", 1, "HASH-1", &prov)
            .unwrap();
    }

    // A fresh connection (restart) reads the dangling intent back as Unknown — the
    // send may have happened, so the effect is neither lost nor asserted delivered.
    let outbox = Outbox::open(home.path()).unwrap();
    let entry = outbox.load(SVC_SYNTHETIC_SUBMIT, "file").unwrap().unwrap();
    assert_eq!(entry.attempts.len(), 1);
    // A crash-recovered intent is Unknown, but its cause is distinct from a socket
    // timeout: there was never an adapter answer at all. The trail labels it
    // CRASH_UNRESOLVED so an operator can tell a mid-send crash from a timeout.
    assert!(
        matches!(&entry.attempts[0].outcome, AttemptOutcome::Unknown { code, .. } if code == "CRASH_UNRESOLVED"),
        "a crashed intent must recover as Unknown{{CRASH_UNRESOLVED}}, got {:?}",
        entry.attempts[0].outcome
    );
    assert_eq!(entry.attempts[0].sent_hash, "HASH-1");
    // The used attempt number is not reclaimed — a deliberate retry gets a fresh one.
    assert_eq!(entry.next_attempt_no(), 2);
}

#[test]
fn a_resolved_outcome_is_durable_across_reopen() {
    let home = tempfile::tempdir().unwrap();
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            200,
            json!({"receipt": "R-ok"}),
        ))])),
    );
    let outcome = broker
        .submit("agent", "file", &filing_envelope("nk:1"))
        .unwrap();
    assert_eq!(
        outcome,
        RemoteOutcome::Delivered {
            receipt: "R-ok".into()
        }
    );

    // Reopen the outbox on the same home: the terminal success survived.
    let outbox = Outbox::open(home.path()).unwrap();
    let entry = outbox.load(SVC_SYNTHETIC_SUBMIT, "file").unwrap().unwrap();
    assert_eq!(entry.attempts.len(), 1);
    assert!(entry.terminal_success().is_some());
}

// --- broker-unknown-cause-observability: the cause is durable, not in-memory -

#[test]
fn an_ambiguous_unknowns_cause_survives_an_outbox_reopen() {
    let home = tempfile::tempdir().unwrap();
    // A submit hits an ambiguous proxy 503 → a resolved Unknown carrying HTTP_503.
    {
        let broker = submit_broker(
            home.path(),
            Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
                503,
                json!({}),
            ))])),
        );
        assert_eq!(
            broker
                .submit("agent", "file", &filing_envelope("nk:1"))
                .unwrap(),
            RemoteOutcome::Unknown
        );
    }

    // Reopen on the same home (a restart): the cause is persisted in `outcome_json`,
    // not just held in memory, so a triaging operator still sees HTTP_503 after a
    // process restart — the observability guarantee is durable.
    let outbox = Outbox::open(home.path()).unwrap();
    let entry = outbox.load(SVC_SYNTHETIC_SUBMIT, "file").unwrap().unwrap();
    assert!(
        matches!(&entry.attempts[0].outcome, AttemptOutcome::Unknown { code, .. } if code == "HTTP_503"),
        "the ambiguous cause must survive a reopen, got {:?}",
        entry.attempts[0].outcome
    );
}

// --- End to end: crash mid-send, then a deliberate retry recovers via 409 ----

#[test]
fn crash_mid_send_then_retry_recovers_no_double_effect() {
    let home = tempfile::tempdir().unwrap();
    let env = filing_envelope("nk:1");

    // First submit crashes inside the transport — AFTER `begin_attempt` committed the
    // durable intent, BEFORE any outcome could be resolved. This proves the *ordering*
    // invariant (the intent's transaction commits before the adapter is called, so the
    // panic cannot roll it back); it is not a power-loss durability test.
    {
        let broker = submit_broker(home.path(), Box::new(PanicOnDispatch));
        let crashed = catch_unwind(AssertUnwindSafe(|| broker.submit("agent", "file", &env)));
        assert!(
            crashed.is_err(),
            "the transport panic simulates a crash mid-send"
        );
    }

    // Restart: the request actually landed at the authority. A deliberate retry meets
    // its 409 and recovers — no double filing, same sent_hash throughout.
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Ok(RawResponse::new(
            409,
            json!({"receipt": "R-recovered", "idempotency_key": "nk:1"}),
        ))])),
    );
    let recovered = broker.submit("agent", "file", &env).unwrap();
    assert_eq!(
        recovered,
        RemoteOutcome::RecoveredAlreadyProcessed {
            receipt: "R-recovered".into()
        }
    );

    let entry = broker
        .outbox_entry(SVC_SYNTHETIC_SUBMIT, "file")
        .unwrap()
        .unwrap();
    // Two attempts: the crashed intent (Unknown, cause CRASH_UNRESOLVED) and the
    // recovering retry.
    assert_eq!(entry.attempts.len(), 2);
    assert!(matches!(
        &entry.attempts[0].outcome,
        AttemptOutcome::Unknown { code, .. } if code == "CRASH_UNRESOLVED"
    ));
    assert!(matches!(
        entry.attempts[1].outcome,
        AttemptOutcome::RecoveredAlreadyProcessed { .. }
    ));
    assert_eq!(
        entry.attempts[0].sent_hash, entry.attempts[1].sent_hash,
        "the deliberate retry reuses the crashed attempt's sent_hash"
    );
}

// --- A deliberate retry after Unknown must reuse the ORIGINAL payload --------

#[test]
fn a_retry_after_unknown_with_changed_bytes_is_refused() {
    let home = tempfile::tempdir().unwrap();
    // First submit times out → attempt 1 recorded as Unknown (a durable open intent).
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![Err(TransportError::Timeout)])),
    );
    let outcome = broker
        .submit("agent", "file", &filing_envelope("nk:1"))
        .unwrap();
    assert_eq!(outcome, RemoteOutcome::Unknown);

    // A resubmit under the same effect_key + natural_key but DIFFERENT bytes would go
    // to the authority under the same idempotency key with a fresh sent_hash — a
    // double-effect hazard. It must be refused, not dispatched.
    let tampered = Envelope::new(
        SVC_SYNTHETIC_SUBMIT,
        "submit-vat-return",
        "nk:1",
        json!({"period": "2026-Q1", "net_vat": "9999.00"}),
    );
    let err = broker.submit("agent", "file", &tampered).unwrap_err();
    assert_eq!(err.error_code(), "IDEMPOTENCY_MISMATCH");

    // No second attempt was recorded — the mismatch short-circuited before dispatch.
    let entry = broker
        .outbox_entry(SVC_SYNTHETIC_SUBMIT, "file")
        .unwrap()
        .unwrap();
    assert_eq!(
        entry.attempts.len(),
        1,
        "the tampered retry never dispatched"
    );
}

// --- A prior Unknown must NOT consume this call's transport retry budget -----

#[test]
fn a_deliberate_retry_after_unknowns_gets_the_full_retry_budget() {
    let home = tempfile::tempdir().unwrap();
    // History: two timeouts (attempts 1 & 2 → Unknown), then a deliberate retry that
    // hits ONE retryable transport failure (a refused connection — every 5xx is now
    // ambiguous → Unknown for a submit) before succeeding (200). With the durable
    // attempt index at 3 and max_attempts=3, counting historical attempts against the
    // budget would exhaust immediately; the per-call budget must not.
    let broker = submit_broker(
        home.path(),
        Box::new(ScriptedTransport::new(vec![
            Err(TransportError::Timeout),
            Err(TransportError::Timeout),
            Err(TransportError::ConnectionRefused("conn reset".into())),
            Ok(RawResponse::new(200, json!({"receipt": "R-ok"}))),
        ])),
    )
    .with_retry(RetryPolicy { max_attempts: 3 });
    let env = filing_envelope("nk:1");

    assert_eq!(
        broker.submit("agent", "file", &env).unwrap(),
        RemoteOutcome::Unknown
    );
    assert_eq!(
        broker.submit("agent", "file", &env).unwrap(),
        RemoteOutcome::Unknown
    );
    // The third call retries the 503 within its own budget and delivers.
    assert_eq!(
        broker.submit("agent", "file", &env).unwrap(),
        RemoteOutcome::Delivered {
            receipt: "R-ok".into()
        }
    );

    let entry = broker
        .outbox_entry(SVC_SYNTHETIC_SUBMIT, "file")
        .unwrap()
        .unwrap();
    // 1,2 Unknown; 3 the retryable refused connection; 4 the delivering retry.
    assert_eq!(entry.attempts.len(), 4);
    assert!(matches!(
        entry.attempts[3].outcome,
        AttemptOutcome::Delivered { .. }
    ));
}

#[test]
fn outbox_is_git_excluded_even_in_an_existing_home_with_custom_rules() {
    let home = tempfile::tempdir().unwrap();
    assert!(std::process::Command::new("git")
        .args(["init", "-q"])
        .arg(home.path())
        .status()
        .unwrap()
        .success());
    let dir = home.path().join("integration/outbox");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(".gitignore"), "# operator rules\n!/outbox.db\n").unwrap();
    Outbox::open(home.path()).unwrap();
    let rules = std::fs::read_to_string(dir.join(".gitignore")).unwrap();
    assert!(rules.contains("# operator rules"));
    for name in ["outbox.db", "outbox.db-wal", "outbox.db-shm"] {
        assert!(std::process::Command::new("git")
            .arg("-C")
            .arg(home.path())
            .args(["check-ignore", "-q", &format!("integration/outbox/{name}")])
            .status()
            .unwrap()
            .success());
    }
    Outbox::open(home.path()).unwrap();
    assert_eq!(
        rules,
        std::fs::read_to_string(dir.join(".gitignore")).unwrap()
    );
}
