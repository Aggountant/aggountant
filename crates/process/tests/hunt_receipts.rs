//! RF2 `hunt-receipts` Run tests — the S5 receipt-hunt slice: targeted per-transaction
//! pulls, per-item seal on approval, the durable `Effect::Wait` (crash-across-the-wait
//! resume), the privacy boundary (no raw pulled body persists), idempotent dedup, and
//! the actionable `Clarify` for an unexplained transfer.

use std::path::Path;

use serde_json::{json, Value};

use aggountant_broker::{
    Broker, EmailPullAdapter, KauppatiliPullAdapter, RawResponse, Registry, ScriptedTransport,
    TransportError, Vault, SVC_EMAIL_PULL, SVC_KAUPPATILI_PULL,
};
use aggountant_domain::PolicySet;
use aggountant_ledger::Ledger;
use aggountant_process::{FixedClock, HuntReceiptsInput, ProcessEngine, RunStatus, SystemClock};

type Steps = Vec<Result<RawResponse, TransportError>>;

fn ok(status: u16, body: Value) -> Result<RawResponse, TransportError> {
    Ok(RawResponse::new(status, body))
}

fn home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    home
}

fn hunt_broker(home: &Path, email: Steps, kauppatili: Steps) -> Broker {
    let mut vault = Vault::new();
    vault.set(SVC_EMAIL_PULL, "secret");
    vault.set(SVC_KAUPPATILI_PULL, "secret");
    let mut reg = Registry::new();
    reg.register(Box::new(EmailPullAdapter::new(Box::new(
        ScriptedTransport::new(email),
    ))));
    reg.register(Box::new(KauppatiliPullAdapter::new(Box::new(
        ScriptedTransport::new(kauppatili),
    ))));
    Broker::new(home, vault, reg)
}

fn one_txn() -> HuntReceiptsInput {
    serde_json::from_value(json!({
        "transactions": [{
            "bank_account": "FI2112345600000785",
            "txn_id": "t1",
            "counterparty": "Verkkokauppa Oy",
            "amount": {"amount": "124.00", "currency": "EUR"},
            "date": "2026-03-14",
            "evidence_status": "missing",
            "proposed_draft": {
                "document_date": "2026-03-14",
                "posting_date": "2026-03-14",
                "description": "Verkkokauppa osto",
                "postings": [
                    {"account": "7690", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "debit"},
                    {"account": "1910", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "credit"}
                ]
            }
        }]
    }))
    .unwrap()
}

/// A found document body: the raw `email_body` carries PII that must NOT persist; only
/// the `document` becomes the artifact.
fn found_body() -> Value {
    json!({
        "email_body": "SECRET-BODY hetu 131052-308T iban FI2112345600000785 — full raw email",
        "document": {"receipt": "WK-999", "amount": "124.00", "counterparty": "Verkkokauppa Oy"}
    })
}

fn voucher_count(home: &Path) -> u64 {
    Ledger::open(home).unwrap().verify().unwrap().voucher_count
}

#[test]
fn targeted_pull_matches_and_each_item_seals_on_its_own_approval() {
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = hunt_broker(home.path(), vec![ok(200, found_body())], vec![]);

    let view = engine
        .start_hunt_receipts(&one_txn(), &broker, &PolicySet::default(), &SystemClock)
        .unwrap();
    // The transaction matched → its ApproveMatch gate is open, nothing sealed yet.
    assert_eq!(view.status, RunStatus::AwaitingGate);
    assert!(view
        .gates
        .iter()
        .any(|g| g.gate_id == "ApproveMatch:FI2112345600000785:t1"));
    assert_eq!(voucher_count(home.path()), 0);

    // A typed intake candidate exists (the minimal F17 home).
    let candidates = aggountant_process::Intake::open(home.path())
        .list()
        .unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].txn_id, "t1");
    assert!(!candidates[0].artifact_ref.is_empty());

    // Approving that item seals it now (a thin per-item commit).
    let view = engine
        .answer_hunt(
            &view.run_id,
            "ApproveMatch:FI2112345600000785:t1",
            "approve",
        )
        .unwrap();
    assert_eq!(view.status, RunStatus::Completed);
    assert_eq!(voucher_count(home.path()), 1);
}

#[test]
fn no_raw_pulled_body_persists_after_the_run() {
    // The privacy boundary: only the attachment + derived metadata survive; the raw
    // email body (with its PII) is used transiently and never written to disk.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = hunt_broker(home.path(), vec![ok(200, found_body())], vec![]);
    let view = engine
        .start_hunt_receipts(&one_txn(), &broker, &PolicySet::default(), &SystemClock)
        .unwrap();
    engine
        .answer_hunt(
            &view.run_id,
            "ApproveMatch:FI2112345600000785:t1",
            "approve",
        )
        .unwrap();

    // Walk every file under the home; none may contain the raw body / its PII.
    let mut hits = Vec::new();
    for entry in walkdir(home.path()) {
        if let Ok(bytes) = std::fs::read(&entry) {
            let text = String::from_utf8_lossy(&bytes);
            if text.contains("SECRET-BODY") || text.contains("131052-308T") {
                hits.push(entry.display().to_string());
            }
        }
    }
    assert!(hits.is_empty(), "raw pulled body/PII leaked into: {hits:?}");
}

#[test]
fn rate_limit_parks_a_durable_wait_and_resumes_across_a_crash() {
    // Effect::Wait + crash-across-the-wait: a 429 parks the pull on a durable deadline
    // (journal state). A fresh engine resumes: before the deadline it stays Waiting;
    // after the backoff (injected clock advanced) it re-pulls, finds the match, and opens
    // the ApproveMatch gate. No wall-clock sleep.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let clock = FixedClock::new(2_000_000);
    // Email: first 429 (backoff 600s), then a match on the resume re-pull.
    let broker = hunt_broker(
        home.path(),
        vec![
            ok(429, json!({"retry_after_secs": 600})),
            ok(200, found_body()),
        ],
        vec![],
    );

    let view = engine
        .start_hunt_receipts(&one_txn(), &broker, &PolicySet::default(), &clock)
        .unwrap();
    assert_eq!(view.status, RunStatus::Waiting);
    let run_id = view.run_id.clone();

    // "Crash": a new engine, same durable journal.
    let engine2 = ProcessEngine::open(home.path());
    let view = engine2
        .resume_hunt_receipts(&run_id, &broker, &clock)
        .unwrap();
    assert_eq!(view.status, RunStatus::Waiting, "still within the backoff");

    clock.advance(600);
    let view = engine2
        .resume_hunt_receipts(&run_id, &broker, &clock)
        .unwrap();
    assert_eq!(view.status, RunStatus::AwaitingGate);
    assert!(view
        .gates
        .iter()
        .any(|g| g.gate_id == "ApproveMatch:FI2112345600000785:t1"));
}

#[test]
fn an_unexplained_transfer_raises_an_actionable_clarify_no_guess() {
    // Nothing found in any source → a Clarify gate, not a guessed booking.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = hunt_broker(
        home.path(),
        vec![ok(404, json!({}))],
        vec![ok(404, json!({}))],
    );
    let view = engine
        .start_hunt_receipts(&one_txn(), &broker, &PolicySet::default(), &SystemClock)
        .unwrap();
    assert!(view
        .gates
        .iter()
        .any(|g| g.gate_id == "Clarify:FI2112345600000785:t1"));
    assert_eq!(voucher_count(home.path()), 0);

    // Acknowledging it does not seal anything (no guess).
    let view = engine
        .answer_hunt(&view.run_id, "Clarify:FI2112345600000785:t1", "acknowledge")
        .unwrap();
    assert!(matches!(
        view.status,
        RunStatus::Completed | RunStatus::Running | RunStatus::AwaitingGate
    ));
    assert_eq!(voucher_count(home.path()), 0);
}

#[test]
fn idempotency_dedupes_a_re_hunted_transaction() {
    // Two separate hunts of the same transaction seal exactly one voucher (the
    // voucher-match:<acct>:<txn> natural key + the outbox/seal idempotency dedupe).
    let home = home();
    let engine = ProcessEngine::open(home.path());

    let broker1 = hunt_broker(home.path(), vec![ok(200, found_body())], vec![]);
    let v1 = engine
        .start_hunt_receipts(&one_txn(), &broker1, &PolicySet::default(), &SystemClock)
        .unwrap();
    engine
        .answer_hunt(&v1.run_id, "ApproveMatch:FI2112345600000785:t1", "approve")
        .unwrap();
    assert_eq!(voucher_count(home.path()), 1);

    let broker2 = hunt_broker(home.path(), vec![ok(200, found_body())], vec![]);
    let v2 = engine
        .start_hunt_receipts(&one_txn(), &broker2, &PolicySet::default(), &SystemClock)
        .unwrap();
    engine
        .answer_hunt(&v2.run_id, "ApproveMatch:FI2112345600000785:t1", "approve")
        .unwrap();
    // Still one voucher — the re-hunt deduped.
    assert_eq!(voucher_count(home.path()), 1);
}

#[test]
fn discarding_a_match_completes_the_run_and_marks_intake_discarded() {
    // F10: a discarded match skips its seal (terminal non-success) so the Run can still
    // reach Completed, and the intake candidate is marked Discarded (not left Pending).
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = hunt_broker(home.path(), vec![ok(200, found_body())], vec![]);
    let view = engine
        .start_hunt_receipts(&one_txn(), &broker, &PolicySet::default(), &SystemClock)
        .unwrap();
    let view = engine
        .answer_hunt(
            &view.run_id,
            "ApproveMatch:FI2112345600000785:t1",
            "discard",
        )
        .unwrap();
    assert_eq!(view.status, RunStatus::Completed);
    assert_eq!(voucher_count(home.path()), 0);
    let candidates = aggountant_process::Intake::open(home.path())
        .list()
        .unwrap();
    assert_eq!(
        candidates[0].status,
        aggountant_process::MatchStatus::Discarded
    );
}

#[test]
fn acknowledging_a_clarify_completes_the_run() {
    // F10: a clarified (no-evidence) transaction skips its seal, so acknowledging the
    // Clarify gate reaches Completed rather than wedging AwaitingGate/Running forever.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = hunt_broker(
        home.path(),
        vec![ok(404, json!({}))],
        vec![ok(404, json!({}))],
    );
    let view = engine
        .start_hunt_receipts(&one_txn(), &broker, &PolicySet::default(), &SystemClock)
        .unwrap();
    let view = engine
        .answer_hunt(&view.run_id, "Clarify:FI2112345600000785:t1", "acknowledge")
        .unwrap();
    assert_eq!(view.status, RunStatus::Completed);
    assert_eq!(voucher_count(home.path()), 0);
}

#[test]
fn a_pull_body_without_a_typed_document_is_refused_no_raw_persist() {
    // F3: a 200 whose body has no `document` must NOT persist the raw body as an artifact.
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let broker = hunt_broker(
        home.path(),
        vec![ok(
            200,
            json!({"email_body": "raw PII 131052-308T", "amount": "124.00"}),
        )],
        vec![],
    );
    let err = engine
        .start_hunt_receipts(&one_txn(), &broker, &PolicySet::default(), &SystemClock)
        .unwrap_err();
    assert_eq!(err.error_code(), "PROCESS_STORE");
    // Nothing leaked to disk.
    for entry in walkdir(home.path()) {
        if let Ok(bytes) = std::fs::read(&entry) {
            assert!(
                !String::from_utf8_lossy(&bytes).contains("131052-308T"),
                "raw body leaked into {}",
                entry.display()
            );
        }
    }
}

#[test]
fn two_banks_sharing_a_txn_id_do_not_collide() {
    // F6: the same txn_id under two different bank accounts gets two distinct gates and
    // both can seal independently (keyed by <bank>:<txn>, not txn_id alone).
    let home = home();
    let engine = ProcessEngine::open(home.path());
    let input: HuntReceiptsInput = serde_json::from_value(json!({
        "transactions": [
            txn_json("FI2112345600000785", "t1"),
            txn_json("FI5542345670000081", "t1"),
        ]
    }))
    .unwrap();
    let broker = hunt_broker(
        home.path(),
        vec![ok(200, found_body()), ok(200, found_body())],
        vec![],
    );
    let view = engine
        .start_hunt_receipts(&input, &broker, &PolicySet::default(), &SystemClock)
        .unwrap();
    assert!(view
        .gates
        .iter()
        .any(|g| g.gate_id == "ApproveMatch:FI2112345600000785:t1"));
    assert!(view
        .gates
        .iter()
        .any(|g| g.gate_id == "ApproveMatch:FI5542345670000081:t1"));
    engine
        .answer_hunt(
            &view.run_id,
            "ApproveMatch:FI2112345600000785:t1",
            "approve",
        )
        .unwrap();
    let view = engine
        .answer_hunt(
            &view.run_id,
            "ApproveMatch:FI5542345670000081:t1",
            "approve",
        )
        .unwrap();
    assert_eq!(view.status, RunStatus::Completed);
    assert_eq!(voucher_count(home.path()), 2);
}

fn txn_json(bank: &str, txn: &str) -> Value {
    json!({
        "bank_account": bank, "txn_id": txn, "counterparty": "Verkkokauppa Oy",
        "amount": {"amount": "124.00", "currency": "EUR"}, "date": "2026-03-14",
        "evidence_status": "missing",
        "proposed_draft": {
            "document_date": "2026-03-14", "posting_date": "2026-03-14", "description": "osto",
            "postings": [
                {"account": "7690", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "debit"},
                {"account": "1910", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "credit"}
            ]
        }
    })
}

/// Recursively list files under `root` (test-only, no external crate).
fn walkdir(root: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out
}
