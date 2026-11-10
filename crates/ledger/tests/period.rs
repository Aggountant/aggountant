//! ADR-6 / S2 fiscal gate tests — the point of S2 is *these*, not the happy path.
//!
//! Covered here at the Ledger (fiscal) level:
//! 1. **Transition-vs-seal atomic lock** — a concurrent `voucher seal` racing
//!    `begin_close(Open→Closing)` is cleanly refused (`PERIOD_CLOSING`); no window
//!    where a voucher seals into a period the run believes is `Closing`.
//! 2. **`ClosingAuthority` fencing** — a token that does not own the period is
//!    refused (`VOUCHER_NOT_IN_RUN`); a bare `--run-id` cannot even be formed (the
//!    capability is non-serialisable with no public constructor — compile-level).
//! 3. **Reopen exception (ADR-6 §5)** — `Sealed→Open/Closing` serializes on the
//!    writer lock; reopen after filing or with a foreign token is Refused (with a
//!    rejection event); the reopen-vs-seal race is tested.
//!
//! Invariant 4 (resume reads authoritative fiscal PeriodState) lives with the
//! process engine (`aggountant-process`), which owns the crash seam.

use std::sync::{Arc, Barrier};
use std::thread;

use aggountant_domain::PolicySet;
use aggountant_ledger::{Ledger, LedgerError, PeriodState, SealRequest};
use aggountant_model::{AccountId, Currency, Date, Money, PeriodId, Posting, Side, VoucherDraft};

fn eur() -> Currency {
    Currency::new("EUR").unwrap()
}

fn posting(acc: &str, amount: &str, side: Side) -> Posting {
    Posting {
        account: AccountId::new(acc).unwrap(),
        amount: Money::parse(amount, eur()).unwrap(),
        side,
        vat_code: None,
    }
}

/// A balanced draft whose `posting_date` fixes the period it is gated against.
fn draft_on(date: &str, amount: &str) -> VoucherDraft {
    let d = Date::parse(date).unwrap();
    VoucherDraft::new(
        d,
        d,
        "Hosting invoice".into(),
        vec![
            posting("7690", amount, Side::Debit),
            posting("1910", amount, Side::Credit),
        ],
        None,
    )
    .unwrap()
}

fn req(natural_key: &str) -> SealRequest {
    SealRequest {
        natural_key: natural_key.into(),
        actor: "test".into(),
        policy_set: PolicySet::default(),
    }
}

fn q1() -> PeriodId {
    PeriodId::parse("2026-Q1").unwrap()
}
fn q2() -> PeriodId {
    PeriodId::parse("2026-Q2").unwrap()
}

// --- The thin happy path, end to end at the fiscal level -------------------

#[test]
fn open_to_closing_to_sealed_happy_path() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();

    // Open period: an ordinary seal is allowed.
    ledger
        .seal(&draft_on("2026-02-01", "10.00"), &req("k:1"))
        .unwrap();
    assert_eq!(ledger.period_view(&q1()).unwrap().state, PeriodState::Open);

    let auth = ledger.begin_close(&q1(), "run-1", "closer").unwrap();
    assert_eq!(
        ledger.period_view(&q1()).unwrap().state,
        PeriodState::Closing
    );
    assert_eq!(
        ledger.period_view(&q1()).unwrap().owning_run_id.as_deref(),
        Some("run-1")
    );

    ledger.seal_period(&q1(), &auth, "closer").unwrap();
    let view = ledger.period_view(&q1()).unwrap();
    assert_eq!(view.state, PeriodState::Sealed);
    // Two transitions folded (Open→Closing, Closing→Sealed).
    assert_eq!(view.head_transition_seq, 2);

    let report = ledger.verify().unwrap();
    assert!(report.ok);
    assert_eq!(report.period_count, 1);
}

// --- Invariant 1: transition-vs-seal atomic lock ---------------------------

#[test]
fn bare_seal_into_closing_period_is_refused_and_logged() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    ledger.begin_close(&q1(), "run-1", "closer").unwrap();

    let err = ledger
        .seal(&draft_on("2026-03-10", "5.00"), &req("k:closing"))
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_CLOSING");
    assert_eq!(
        ledger.rejection_count().unwrap(),
        1,
        "refusal is logged (O-25)"
    );
    // No voucher landed in the closing period.
    assert_eq!(ledger.verify().unwrap().voucher_count, 0);
}

#[test]
fn seal_into_sealed_period_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let auth = ledger.begin_close(&q1(), "run-1", "closer").unwrap();
    ledger.seal_period(&q1(), &auth, "closer").unwrap();

    let err = ledger
        .seal(&draft_on("2026-03-10", "5.00"), &req("k:sealed"))
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_SEALED");
}

#[test]
fn concurrent_seal_racing_transition_has_no_window() {
    const SEALERS: usize = 8;
    let home = tempfile::tempdir().unwrap();
    let _seed = Ledger::init(home.path()).unwrap();
    drop(_seed);
    let home_path = home.path().to_path_buf();

    // A barrier so every sealer and the closer are released at once, maximizing
    // the chance the transition genuinely interleaves with in-flight seals.
    let barrier = Arc::new(Barrier::new(SEALERS + 1));

    let mut sealers = Vec::new();
    for i in 0..SEALERS {
        let hp = home_path.clone();
        let b = Arc::clone(&barrier);
        sealers.push(thread::spawn(move || {
            let ledger = Ledger::open(&hp).unwrap();
            b.wait();
            ledger.seal(&draft_on("2026-02-20", "1.00"), &req(&format!("s:{i}")))
        }));
    }

    let closer = {
        let hp = home_path.clone();
        let b = Arc::clone(&barrier);
        thread::spawn(move || {
            let ledger = Ledger::open(&hp).unwrap();
            b.wait();
            ledger.begin_close(&q1(), "run-1", "closer")
        })
    };

    let mut sealed = 0usize;
    let mut refused_closing = 0usize;
    for s in sealers {
        match s.join().unwrap() {
            Ok(_) => sealed += 1,
            Err(e) => {
                // The ONLY tolerated failure is a clean PERIOD_CLOSING refusal —
                // never a torn state, a CAS conflict, or a corrupt chain.
                assert_eq!(
                    e.error_code(),
                    "PERIOD_CLOSING",
                    "a racing seal must either commit into the Open period or be \
                     cleanly refused PERIOD_CLOSING, got {e}"
                );
                refused_closing += 1;
            }
        }
    }
    closer.join().unwrap().expect("begin_close must succeed");

    let ledger = Ledger::open(&home_path).unwrap();
    // The period is Closing, the chain re-verifies, and the books reconcile:
    // exactly the `sealed` vouchers exist, and every refusal was logged.
    assert_eq!(
        ledger.period_view(&q1()).unwrap().state,
        PeriodState::Closing
    );
    let report = ledger.verify().unwrap();
    assert!(report.ok, "chain must re-verify after the race");
    assert_eq!(report.voucher_count as usize, sealed);
    assert_eq!(ledger.rejection_count().unwrap() as usize, refused_closing);
    // A post-transition seal is now always refused — the lock holds.
    let err = ledger
        .seal(&draft_on("2026-02-20", "1.00"), &req("s:after"))
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_CLOSING");
}

// --- Invariant 2: ClosingAuthority fencing ---------------------------------

#[test]
fn owning_authority_may_write_into_its_closing_period() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let auth = ledger.begin_close(&q1(), "run-1", "closer").unwrap();

    // The close-run's own write, presenting its capability, is allowed.
    let sealed = ledger
        .seal_with_authority(&draft_on("2026-03-01", "7.00"), &req("k:own"), Some(&auth))
        .unwrap();
    assert_eq!(sealed.number, 1);
}

#[test]
fn foreign_authority_is_fenced_on_closing_write() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    ledger.begin_close(&q1(), "run-1", "closer").unwrap();
    let auth_q2 = ledger.begin_close(&q2(), "run-2", "closer").unwrap();

    // A token that owns Q2 cannot authorize a write into Q1 — VOUCHER_NOT_IN_RUN.
    let err = ledger
        .seal_with_authority(
            &draft_on("2026-03-01", "7.00"),
            &req("k:foreign"),
            Some(&auth_q2),
        )
        .unwrap_err();
    assert_eq!(err.error_code(), "VOUCHER_NOT_IN_RUN");
    // And the refusal is logged.
    assert!(ledger.rejection_count().unwrap() >= 1);
}

#[test]
fn acquire_authority_fences_a_foreign_run() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let auth = ledger.begin_close(&q1(), "run-1", "closer").unwrap();
    ledger.seal_period(&q1(), &auth, "closer").unwrap();

    // Only the owning run can re-derive a token from the authoritative state.
    assert!(
        ledger.acquire_authority(&q1(), "run-1").is_ok(),
        "the owner re-acquires its authority"
    );
    let err = ledger.acquire_authority(&q1(), "run-999").unwrap_err();
    assert_eq!(err.error_code(), "VOUCHER_NOT_IN_RUN");
}

#[test]
fn seal_period_with_foreign_token_is_conflict() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    ledger.begin_close(&q1(), "run-1", "closer").unwrap();
    let auth_q2 = ledger.begin_close(&q2(), "run-2", "closer").unwrap();

    // Presenting Q2's authority to seal Q1 is refused before it touches the row.
    let err = ledger.seal_period(&q1(), &auth_q2, "closer").unwrap_err();
    assert_eq!(err.error_code(), "VOUCHER_NOT_IN_RUN");
    // Q1 is untouched — still Closing.
    assert_eq!(
        ledger.period_view(&q1()).unwrap().state,
        PeriodState::Closing
    );
}

// --- Invariant 3: reopen exception (ADR-6 §5) ------------------------------

#[test]
fn reopen_before_filing_succeeds_after_filing_refused() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let auth = ledger.begin_close(&q1(), "run-1", "closer").unwrap();
    ledger.seal_period(&q1(), &auth, "closer").unwrap();

    // Pre-filing: the owning run may reopen Sealed→Closing.
    let auth2 = ledger.acquire_authority(&q1(), "run-1").unwrap();
    let view = ledger
        .reopen_period(&q1(), &auth2, PeriodState::Closing, "closer")
        .unwrap();
    assert_eq!(view.state, PeriodState::Closing);

    // Re-seal, then file the VAT return.
    ledger.seal_period(&q1(), &auth2, "closer").unwrap();
    let filing = ledger
        .acquire_filing_authority(&q1(), "manual-file-vat")
        .unwrap();
    ledger
        .file_vat_manually(
            &q1(),
            &filing,
            "omavero",
            "archive/receipt.pdf",
            None,
            "test",
        )
        .unwrap();
    assert!(ledger.period_view(&q1()).unwrap().vat_filed);

    // Post-filing: reopen is forbidden (the pre-filing CAS guard fails).
    let auth3 = ledger.acquire_authority(&q1(), "manual-file-vat").unwrap();
    let before = ledger.rejection_count().unwrap();
    let err = ledger
        .reopen_period(&q1(), &auth3, PeriodState::Closing, "closer")
        .unwrap_err();
    assert_eq!(err.error_code(), "PERIOD_TRANSITION_CONFLICT");
    assert_eq!(
        ledger.rejection_count().unwrap(),
        before + 1,
        "the refused reopen is logged, not silent"
    );
    // Still Sealed + filed — no silent mutation.
    let view = ledger.period_view(&q1()).unwrap();
    assert_eq!(view.state, PeriodState::Sealed);
    assert!(view.vat_filed);
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn reopen_with_foreign_token_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let auth = ledger.begin_close(&q1(), "run-1", "closer").unwrap();
    ledger.seal_period(&q1(), &auth, "closer").unwrap();
    let foreign = ledger.begin_close(&q2(), "run-2", "closer").unwrap();

    let err = ledger
        .reopen_period(&q1(), &foreign, PeriodState::Closing, "closer")
        .unwrap_err();
    assert_eq!(err.error_code(), "VOUCHER_NOT_IN_RUN");
    assert_eq!(
        ledger.period_view(&q1()).unwrap().state,
        PeriodState::Sealed
    );
}

#[test]
fn reopen_vs_seal_race_never_lets_a_voucher_into_a_sealed_or_closing_period() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let auth = ledger.begin_close(&q1(), "run-1", "closer").unwrap();
    ledger.seal_period(&q1(), &auth, "closer").unwrap();
    let home_path = home.path().to_path_buf();
    drop(ledger);

    let barrier = Arc::new(Barrier::new(2));

    let reopener = {
        let hp = home_path.clone();
        let b = Arc::clone(&barrier);
        thread::spawn(move || {
            let ledger = Ledger::open(&hp).unwrap();
            let a = ledger.acquire_authority(&q1(), "run-1").unwrap();
            b.wait();
            ledger.reopen_period(&q1(), &a, PeriodState::Closing, "closer")
        })
    };
    let sealer = {
        let hp = home_path.clone();
        let b = Arc::clone(&barrier);
        thread::spawn(move || {
            let ledger = Ledger::open(&hp).unwrap();
            b.wait();
            ledger.seal(&draft_on("2026-02-05", "3.00"), &req("s:race"))
        })
    };

    reopener.join().unwrap().expect("reopen must succeed");
    // Whether the seal ran while the period was Sealed or (after reopen) Closing,
    // a bare seal is refused with one of the two lawful period codes — never Ok.
    let sealer_err = sealer.join().unwrap().unwrap_err();
    assert!(
        matches!(sealer_err.error_code(), "PERIOD_SEALED" | "PERIOD_CLOSING"),
        "a bare seal racing reopen must be refused, got {sealer_err}"
    );

    let ledger = Ledger::open(&home_path).unwrap();
    assert_eq!(ledger.verify().unwrap().voucher_count, 0);
    assert!(ledger.verify().unwrap().ok);
}

// --- fold(period_transition) == period (F18) -------------------------------

#[test]
fn verify_detects_a_tampered_period_state_row() {
    let home = tempfile::tempdir().unwrap();
    {
        let ledger = Ledger::init(home.path()).unwrap();
        let auth = ledger.begin_close(&q1(), "run-1", "closer").unwrap();
        ledger.seal_period(&q1(), &auth, "closer").unwrap();
        assert!(ledger.verify().unwrap().ok);
    }
    // Overwrite the materialized state row so it disagrees with the fold of its
    // transition log — the ADR-6 defense-in-depth check must catch it.
    {
        let conn = rusqlite::Connection::open(home.path().join("fiscal.sqlite")).unwrap();
        conn.execute(
            "UPDATE period SET state='Open' WHERE period_id='2026-Q1'",
            [],
        )
        .unwrap();
    }
    let ledger = Ledger::open(home.path()).unwrap();
    let err = ledger.verify().unwrap_err();
    assert_eq!(err.error_code(), "HASH_CHAIN_BROKEN");
}

#[test]
fn verify_detects_a_tampered_transition_log() {
    let home = tempfile::tempdir().unwrap();
    {
        let ledger = Ledger::init(home.path()).unwrap();
        let auth = ledger.begin_close(&q1(), "run-1", "closer").unwrap();
        ledger.seal_period(&q1(), &auth, "closer").unwrap();
    }
    // Flip a logged transition's owning_run_id without recomputing its hash.
    {
        let conn = rusqlite::Connection::open(home.path().join("fiscal.sqlite")).unwrap();
        conn.execute(
            "UPDATE period_transition SET owning_run_id='forged' WHERE period_seq=1",
            [],
        )
        .unwrap();
    }
    let ledger = Ledger::open(home.path()).unwrap();
    let err = ledger.verify().unwrap_err();
    assert_eq!(err.error_code(), "HASH_CHAIN_BROKEN");
}

/// A `LedgerError` maps to a stable code — a guard the process/CLI layers depend
/// on when branching on `error_code`.
#[test]
fn period_error_codes_are_stable() {
    let closing = LedgerError::PeriodClosing("x".into());
    let sealed = LedgerError::PeriodSealed("x".into());
    let not_in_run = LedgerError::VoucherNotInRun("x".into());
    let conflict = LedgerError::PeriodTransitionConflict("x".into());
    assert_eq!(closing.error_code(), "PERIOD_CLOSING");
    assert_eq!(sealed.error_code(), "PERIOD_SEALED");
    assert_eq!(not_in_run.error_code(), "VOUCHER_NOT_IN_RUN");
    assert_eq!(conflict.error_code(), "PERIOD_TRANSITION_CONFLICT");
}

#[test]
fn july_close_does_not_lock_august() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let july = PeriodId::parse("2026-07").unwrap();
    let auth = ledger.begin_close(&july, "july-close", "closer").unwrap();
    ledger.seal_period(&july, &auth, "closer").unwrap();

    let august = ledger
        .seal(&draft_on("2026-08-01", "8.00"), &req("august:1"))
        .unwrap();
    assert_eq!(august.number, 1);
    let error = ledger
        .seal(&draft_on("2026-07-31", "7.00"), &req("july:late"))
        .unwrap_err();
    assert_eq!(error.error_code(), "PERIOD_SEALED");
}

#[test]
fn month_and_quarter_locks_cannot_bypass_each_other() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let q3 = PeriodId::parse("2026-Q3").unwrap();
    let q3_auth = ledger.begin_close(&q3, "q3", "closer").unwrap();
    let error = ledger
        .seal_with_authority(
            &draft_on("2026-07-01", "1.00"),
            &req("wrong-authority"),
            Some(&ledger.begin_close(&q2(), "q2", "closer").unwrap()),
        )
        .unwrap_err();
    assert_eq!(error.error_code(), "VOUCHER_NOT_IN_RUN");
    ledger.seal_period(&q3, &q3_auth, "closer").unwrap();
    let july = PeriodId::parse("2026-07").unwrap();
    assert_eq!(
        ledger
            .begin_close(&july, "july", "closer")
            .unwrap_err()
            .error_code(),
        "PERIOD_TRANSITION_CONFLICT"
    );

    let other = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(other.path()).unwrap();
    let july_auth = ledger.begin_close(&july, "july", "closer").unwrap();
    ledger.seal_period(&july, &july_auth, "closer").unwrap();
    assert_eq!(
        ledger
            .begin_close(&q3, "q3", "closer")
            .unwrap_err()
            .error_code(),
        "PERIOD_TRANSITION_CONFLICT"
    );
}

#[test]
fn nonoverlapping_mixed_granularity_is_rejected() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let q1_auth = ledger.begin_close(&q1(), "q1", "closer").unwrap();
    ledger.seal_period(&q1(), &q1_auth, "closer").unwrap();
    let october = PeriodId::parse("2026-10").unwrap();
    let error = ledger.begin_close(&october, "oct", "closer").unwrap_err();
    assert_eq!(error.error_code(), "PERIOD_TRANSITION_CONFLICT");
}

#[test]
fn monthly_reopen_to_closing_keeps_the_existing_correction_route() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let july = PeriodId::parse("2026-07").unwrap();
    let authority = ledger.begin_close(&july, "july", "closer").unwrap();
    ledger.seal_period(&july, &authority, "closer").unwrap();
    ledger
        .reopen_period(&july, &authority, PeriodState::Closing, "closer")
        .unwrap();
    let correction = ledger
        .seal_with_authority(
            &draft_on("2026-07-31", "2.00"),
            &req("july:correction"),
            Some(&authority),
        )
        .unwrap();
    assert_eq!(correction.number, 1);
    ledger.seal_period(&july, &authority, "closer").unwrap();
    assert!(ledger.verify().unwrap().ok);
}
