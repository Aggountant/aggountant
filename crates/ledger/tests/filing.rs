//! S3 — the VAT filing is an **audited, hash-chained** fiscal event, replacing the
//! S2 `mark_vat_filed` stub. Proves the M3 fiscal-side invariants:
//! - filing appends a `period_transition` self-edge bound to the `sent_hash`, and
//!   `verify` folds it (`fold(period_transition).vat_filed == row`);
//! - **filing is irreversible** — a second filing is refused;
//! - a tamper that clears `vat_filed` to reopen a filed period is now **caught**
//!   by `voucher verify` (the S2 gap, closed);
//! - filing a non-`Sealed` period is `PERIOD_NOT_SEALED`.

use aggountant_ledger::{Ledger, PeriodState};
use aggountant_model::PeriodId;

fn q1() -> PeriodId {
    PeriodId::parse("2026-Q1").unwrap()
}

/// Seal Q1 (Open→Closing→Sealed) and return an initialized ledger home.
fn sealed_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let auth = ledger.begin_close(&q1(), "close-run", "closer").unwrap();
    ledger.seal_period(&q1(), &auth, "closer").unwrap();
    home
}

#[test]
fn file_vat_appends_an_audited_hash_chained_event() {
    let home = sealed_home();
    let ledger = Ledger::open(home.path()).unwrap();

    let filing_auth = ledger.acquire_filing_authority(&q1(), "file-run").unwrap();
    let view = ledger
        .file_vat(&q1(), &filing_auth, "deadbeef-sent-hash", "filer")
        .unwrap();
    assert!(view.vat_filed);
    assert_eq!(view.state, PeriodState::Sealed);
    // Three transitions folded: Open→Closing, Closing→Sealed, Sealed→Sealed (file).
    assert_eq!(view.head_transition_seq, 3);

    // The chain — including the filing self-edge — re-verifies.
    let report = ledger.verify().unwrap();
    assert!(report.ok);
    assert_eq!(report.period_count, 1);
}

#[test]
fn filing_is_irreversible() {
    let home = sealed_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let auth = ledger.acquire_filing_authority(&q1(), "file-run").unwrap();
    ledger.file_vat(&q1(), &auth, "sent-1", "filer").unwrap();

    // A second acquire is refused (already filed) …
    let err = ledger
        .acquire_filing_authority(&q1(), "file-run")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_TRANSITION_CONFLICT");
    // … and even presenting the earlier token, the filing CAS refuses a re-file.
    let err = ledger
        .file_vat(&q1(), &auth, "sent-2", "filer")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_TRANSITION_CONFLICT");
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn verify_catches_a_cleared_vat_filed_flag() {
    // The S2 gap: clearing `vat_filed` to sneak a reopen past the guard. Now the
    // filing is folded, so the row and the fold disagree → HASH_CHAIN_BROKEN.
    let home = sealed_home();
    {
        let ledger = Ledger::open(home.path()).unwrap();
        let auth = ledger.acquire_filing_authority(&q1(), "file-run").unwrap();
        ledger.file_vat(&q1(), &auth, "sent-1", "filer").unwrap();
        assert!(ledger.verify().unwrap().ok);
    }
    {
        let conn = rusqlite::Connection::open(home.path().join("fiscal.sqlite")).unwrap();
        conn.execute(
            "UPDATE period SET vat_filed=0 WHERE period_id='2026-Q1'",
            [],
        )
        .unwrap();
    }
    let ledger = Ledger::open(home.path()).unwrap();
    let err = ledger.verify().unwrap_err();
    assert_eq!(err.error_code(), "HASH_CHAIN_BROKEN");
}

#[test]
fn filing_a_non_sealed_period_is_period_not_sealed() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    // Q1 is Open (never sealed): acquiring a filing authority is refused.
    let err = ledger
        .acquire_filing_authority(&q1(), "file-run")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_NOT_SEALED");
}

#[test]
fn refiling_same_sent_hash_is_idempotent_but_different_conflicts() {
    // Crash-recovery: a re-file with the SAME sent_hash replays (Ok); a re-file with
    // a DIFFERENT sent_hash is refused (filing stays irreversible).
    let home = sealed_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let auth = ledger.acquire_filing_authority(&q1(), "file-run").unwrap();
    ledger.file_vat(&q1(), &auth, "sent-1", "filer").unwrap();

    // Same sent_hash → idempotent replay (the authority we can still present a token
    // for, because it was minted before the first file committed).
    let view = ledger.file_vat(&q1(), &auth, "sent-1", "filer").unwrap();
    assert!(view.vat_filed);
    assert_eq!(
        view.head_transition_seq, 3,
        "no second filing edge was appended"
    );

    // Different sent_hash → conflict.
    let err = ledger
        .file_vat(&q1(), &auth, "sent-2", "filer")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_TRANSITION_CONFLICT");
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn verify_rejects_a_forged_transition_after_filing() {
    // A tamper that appends a reopen edge *after* the filing self-edge must break
    // verify (filing is irreversible; nothing may follow it).
    let home = sealed_home();
    {
        let ledger = Ledger::open(home.path()).unwrap();
        let auth = ledger.acquire_filing_authority(&q1(), "file-run").unwrap();
        ledger.file_vat(&q1(), &auth, "sent-1", "filer").unwrap();
        assert!(ledger.verify().unwrap().ok);
    }
    // Forge a 4th transition (Sealed→Closing) directly in the DB, chained onto the
    // filing edge's hash so the per-edge hash checks pass — only the "nothing after
    // filing" rule should catch it.
    {
        let conn = rusqlite::Connection::open(home.path().join("fiscal.sqlite")).unwrap();
        let (seq, head_hash): (i64, Vec<u8>) = conn
            .query_row(
                "SELECT period_seq, hash FROM period_transition
                 WHERE period_id='2026-Q1' ORDER BY period_seq DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        conn.execute(
            "INSERT INTO period_transition(period_id, period_seq, prev_hash, from_state,
                to_state, owning_run_id, sent_hash, actor, ts, hash)
             VALUES('2026-Q1', ?1, ?2, 'Sealed', 'Closing', 'forged', NULL, 'x', 'x', ?3)",
            rusqlite::params![seq + 1, head_hash, vec![9u8; 32]],
        )
        .unwrap();
    }
    let ledger = Ledger::open(home.path()).unwrap();
    let err = ledger.verify().unwrap_err();
    assert_eq!(err.error_code(), "HASH_CHAIN_BROKEN");
}

#[test]
fn manual_filing_evidence_is_audited_and_verifies() {
    let home = sealed_home();
    let ledger = Ledger::open(home.path()).unwrap();
    let authority = ledger
        .acquire_filing_authority(&q1(), "manual-file-vat")
        .unwrap();
    ledger
        .file_vat_manually(
            &q1(),
            &authority,
            "ilmoitin",
            "archive/receipt.txt",
            Some("archive/remote-status.txt"),
            "test",
        )
        .unwrap();
    assert!(ledger.period_view(&q1()).unwrap().vat_filed);
    let evidence = ledger.manual_vat_filing_evidence(&q1()).unwrap().unwrap();
    assert_eq!(evidence.channel, "ilmoitin");
    assert_eq!(evidence.evidence_ref, "archive/receipt.txt");
    assert_eq!(
        evidence.remote_status_evidence_ref.as_deref(),
        Some("archive/remote-status.txt")
    );
    assert!(ledger.verify().unwrap().ok);
}
