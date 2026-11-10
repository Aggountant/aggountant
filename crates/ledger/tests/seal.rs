//! Ledger integration tests — the S1 substrate bets (M1 questions).
//!
//! The headline is `concurrent_seal_is_atomic_gapless_and_chained`: the boldest
//! ADR-1 wager, that SQLite-primary fiscal delivers atomic gapless numbering +
//! hash-chain integrity under genuinely concurrent `voucher seal` (independent
//! connections racing on one DB file).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;

use aggountant_domain::{build_draft, DraftInput, PolicySet};
use aggountant_ledger::{Ledger, SealRequest, SoftwareVersionSpan, SOFTWARE_VERSION};
use aggountant_model::{AccountId, Currency, Date, Money, Posting, Side, VatCode, VoucherDraft};

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

/// A balanced purchase-receipt draft: debit an expense, credit the bank.
fn draft(amount: &str) -> VoucherDraft {
    let d = Date::parse("2026-03-14").unwrap();
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

#[test]
fn eu_sale_buyer_vat_id_is_visible_from_the_sealed_body() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let date = Date::parse("2026-05-31").unwrap();
    let draft = build_draft(DraftInput {
        document_date: date,
        posting_date: date,
        description: "EU goods sale".into(),
        postings: vec![
            posting("1910", "100.00", Side::Debit),
            Posting {
                account: AccountId::new("3000").unwrap(),
                amount: Money::parse("100.00", eur()).unwrap(),
                side: Side::Credit,
                vat_code: Some(VatCode::new("sales-eu-goods").unwrap()),
            },
        ],
        attachment_ref: None,
        buyer_vat_id: Some("se 556677889901".into()),
    })
    .unwrap();

    let sealed = ledger.seal(&draft, &req("co:eu-sale-1")).unwrap();
    let shown = ledger.voucher(sealed.number).unwrap();
    assert_eq!(shown.body.buyer_vat_id(), Some("SE556677889901"));
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn source_bound_seal_refuses_an_unapproved_chart_version() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let active = ledger.chart_version().unwrap();
    let err = ledger
        .seal_with_source_fingerprint_at_chart(
            &draft("50.00"),
            &req("approved-chart-mismatch"),
            "sha256:v1:fictional-source",
            "different-chart@0000",
        )
        .unwrap_err();
    assert_eq!(err.error_code(), "CHART_VERSION_CHANGED");
    assert!(err.to_string().contains(&active));
    assert_eq!(ledger.verify().unwrap().voucher_count, 0);
    assert_eq!(ledger.rejection_count().unwrap(), 1);
}

#[test]
fn seal_then_verify_light_path() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();

    // chart list (F7 read)
    let chart = ledger.chart_list(None).unwrap();
    assert!(chart.iter().any(|a| a.id.as_str() == "7690"));

    let v1 = ledger.seal(&draft("100.00"), &req("co:receipt-1")).unwrap();
    let v2 = ledger.seal(&draft("200.00"), &req("co:receipt-2")).unwrap();
    assert_eq!(v1.number, 1);
    assert_eq!(v2.number, 2);
    assert_eq!(v2.prev_hash, v1.hash, "voucher 2 chains onto voucher 1");
    assert!(!v1.replayed);

    let report = ledger.verify().unwrap();
    assert!(report.ok);
    assert_eq!(report.voucher_count, 2);
    assert_eq!(report.head_number, 2);
}

#[test]
fn seal_refuses_a_mistagged_generic_code_on_a_temp_rate_supply() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();

    // Electricity (7010) invoice *dated* inside the temporary-10% window, but tagged
    // with the generic standard band (24%) — a silent wrong-rate filing. The record-
    // time F25 mistag guard must refuse the seal. `posting_date` is a later, open
    // period so only the mistag (document_date-driven) is under test.
    let doc = Date::parse("2023-02-15").unwrap();
    let post = Date::parse("2026-03-14").unwrap();
    let elec = |code: &str| {
        VoucherDraft::new(
            doc,
            post,
            "Sähkölasku".into(),
            vec![
                Posting {
                    account: AccountId::new("7010").unwrap(),
                    amount: Money::parse("100.00", eur()).unwrap(),
                    side: Side::Debit,
                    vat_code: Some(VatCode::new(code).unwrap()),
                },
                posting("1910", "100.00", Side::Credit),
            ],
            None,
        )
        .unwrap()
    };

    let err = ledger
        .seal(&elec("purchase-domestic-standard"), &req("co:elec-mistag"))
        .unwrap_err();
    assert_eq!(err.error_code(), "VAT_STRUCTURE");

    // The dedicated class on the same supply/date is accepted.
    assert!(ledger
        .seal(&elec("purchase-domestic-electricity"), &req("co:elec-ok"))
        .is_ok());
}

#[test]
fn software_version_roundtrips_without_affecting_an_existing_chain() {
    let home = tempfile::tempdir().unwrap();
    let (first_hash, second_hash) = {
        let ledger = Ledger::init(home.path()).unwrap();
        let first = ledger.seal(&draft("10.00"), &req("version:1")).unwrap();
        let second = ledger.seal(&draft("20.00"), &req("version:2")).unwrap();
        assert_eq!(first.software_version, SOFTWARE_VERSION);
        assert_eq!(second.software_version, SOFTWARE_VERSION);
        (first.hash, second.hash)
    };

    // Provenance is deliberately outside canonical_bytes/the hash preimage. Change
    // only that metadata to prove the fiscal chain does not depend on it.
    let conn = rusqlite::Connection::open(home.path().join("fiscal.sqlite")).unwrap();
    conn.execute(
        "UPDATE vouchers SET software_version = CASE number
             WHEN 1 THEN 'old-commit-a' ELSE 'old-commit-b-dirty' END",
        [],
    )
    .unwrap();
    drop(conn);

    let ledger = Ledger::open(home.path()).unwrap();
    let report = ledger.verify().unwrap();
    assert!(
        report.ok,
        "an existing chain must still verify after provenance changes"
    );
    assert_eq!(report.head_hash, second_hash);
    assert_eq!(
        report.software_version_spans,
        vec![
            SoftwareVersionSpan {
                first_number: 1,
                last_number: 1,
                software_version: "old-commit-a".into(),
            },
            SoftwareVersionSpan {
                first_number: 2,
                last_number: 2,
                software_version: "old-commit-b-dirty".into(),
            },
        ]
    );

    // Idempotent read-back returns the version that originally sealed the row,
    // never the currently-running binary's version.
    let replay = ledger.seal(&draft("10.00"), &req("version:1")).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.hash, first_hash);
    assert_eq!(replay.software_version, "old-commit-a");
}

#[test]
fn a_valid_v2_chain_migrates_byte_identically_and_continues() {
    let home = tempfile::tempdir().unwrap();
    let old_head = {
        let ledger = Ledger::init(home.path()).unwrap();
        ledger.seal(&draft("10.00"), &req("migration:1")).unwrap();
        ledger
            .seal(&draft("20.00"), &req("migration:2"))
            .unwrap()
            .hash
    };

    let db = home.path().join("fiscal.sqlite");
    let (rows_before, head_before) = {
        let conn = rusqlite::Connection::open(&db).unwrap();
        let rows = {
            let mut stmt = conn
                .prepare(
                    "SELECT number, prev_hash, hash, canonical_bytes
                     FROM vouchers ORDER BY number",
                )
                .unwrap();
            stmt.query_map([], |r| {
                Ok((
                    r.get::<_, u64>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                    r.get::<_, Vec<u8>>(3)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
        };
        let head = conn
            .query_row(
                "SELECT head_num, head_hash FROM chain_head WHERE series='main'",
                [],
                |r| Ok((r.get::<_, u64>(0)?, r.get::<_, Vec<u8>>(1)?)),
            )
            .unwrap();

        // Rewind only the schema addition, leaving a genuine valid chain in the
        // exact shape a v2 binary persisted before software provenance existed.
        conn.execute_batch(
            "ALTER TABLE vouchers DROP COLUMN source_fingerprint;
             ALTER TABLE vouchers DROP COLUMN software_version;
             ALTER TABLE period_transition DROP COLUMN receipt_unique_identifier;
             ALTER TABLE period_transition DROP COLUMN receipt_accepted_timestamp;
             ALTER TABLE period_transition DROP COLUMN manual_channel;
             ALTER TABLE period_transition DROP COLUMN manual_evidence_ref;
             ALTER TABLE period_transition DROP COLUMN remote_status_evidence_ref;
             PRAGMA user_version = 2;",
        )
        .unwrap();
        (rows, head)
    };

    let ledger = Ledger::open(home.path()).unwrap(); // applies migration v3
    let report = ledger.verify().unwrap();
    assert!(report.ok);
    assert_eq!(report.head_hash, old_head);
    assert_eq!(
        report.software_version_spans,
        vec![SoftwareVersionSpan {
            first_number: 1,
            last_number: 2,
            software_version: "unknown-legacy".into(),
        }]
    );

    let conn = rusqlite::Connection::open(&db).unwrap();
    let rows_after = {
        let mut stmt = conn
            .prepare(
                "SELECT number, prev_hash, hash, canonical_bytes
                 FROM vouchers ORDER BY number",
            )
            .unwrap();
        stmt.query_map([], |r| {
            Ok((
                r.get::<_, u64>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, Vec<u8>>(2)?,
                r.get::<_, Vec<u8>>(3)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    };
    let head_after = conn
        .query_row(
            "SELECT head_num, head_hash FROM chain_head WHERE series='main'",
            [],
            |r| Ok((r.get::<_, u64>(0)?, r.get::<_, Vec<u8>>(1)?)),
        )
        .unwrap();
    drop(conn);
    assert_eq!(
        rows_after, rows_before,
        "migration changed hashed voucher bytes"
    );
    assert_eq!(
        head_after, head_before,
        "migration changed the materialized head"
    );

    let next = ledger.seal(&draft("30.00"), &req("migration:3")).unwrap();
    assert_eq!(next.prev_hash, old_head);
    assert_eq!(next.software_version, SOFTWARE_VERSION);
    assert!(ledger.verify().unwrap().ok);
}

#[test]
fn source_fingerprint_flags_changed_source_even_when_draft_is_identical() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let request = req("bank:event-1");

    let first = ledger
        .seal_with_source_fingerprint(&draft("50.00"), &request, "sha256:v1:source-a")
        .unwrap();
    assert_eq!(
        first.source_fingerprint.as_deref(),
        Some("sha256:v1:source-a")
    );

    let replay = ledger
        .seal_with_source_fingerprint(&draft("50.00"), &request, "sha256:v1:source-a")
        .unwrap();
    assert!(replay.replayed, "an exact source retry remains idempotent");

    let err = ledger
        .seal_with_source_fingerprint(&draft("50.00"), &request, "sha256:v1:source-b")
        .unwrap_err();
    assert_eq!(err.error_code(), "IDEMPOTENCY_KEY_REUSE");
    assert!(err
        .to_string()
        .contains("projected voucher content is unchanged"));
    assert_eq!(ledger.rejection_count().unwrap(), 1);
    assert_eq!(ledger.verify().unwrap().voucher_count, 1);
}

#[test]
fn source_fingerprint_presence_transitions_do_not_rewrite_or_bypass_provenance() {
    let home = tempfile::tempdir().unwrap();
    {
        let ledger = Ledger::init(home.path()).unwrap();
        let unbound_request = req("legacy:unbound");
        ledger.seal(&draft("25.00"), &unbound_request).unwrap();
        let replay = ledger
            .seal_with_source_fingerprint(
                &draft("25.00"),
                &unbound_request,
                "sha256:v1:presented-later",
            )
            .unwrap();
        assert!(replay.replayed);
        assert_eq!(
            replay.source_fingerprint, None,
            "a replay cannot authenticate and adopt historical source provenance"
        );

        let bound_request = req("source:bound");
        ledger
            .seal_with_source_fingerprint(
                &draft("30.00"),
                &bound_request,
                "sha256:v1:bound-at-seal",
            )
            .unwrap();
        let err = ledger.seal(&draft("30.00"), &bound_request).unwrap_err();
        assert_eq!(err.error_code(), "IDEMPOTENCY_KEY_REUSE");
    }

    let ledger = Ledger::open(home.path()).unwrap();
    let replay = ledger
        .seal_with_source_fingerprint(
            &draft("25.00"),
            &req("legacy:unbound"),
            "sha256:v1:another-unverifiable-value",
        )
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.source_fingerprint, None);
    let err = ledger
        .seal(&draft("30.00"), &req("source:bound"))
        .unwrap_err();
    assert_eq!(err.error_code(), "IDEMPOTENCY_KEY_REUSE");
    assert_eq!(ledger.verify().unwrap().voucher_count, 2);
}

#[test]
fn a_valid_v3_chain_adds_source_column_without_changing_chain_bytes() {
    let home = tempfile::tempdir().unwrap();
    let old_head = {
        let ledger = Ledger::init(home.path()).unwrap();
        ledger
            .seal(&draft("10.00"), &req("source-migration:1"))
            .unwrap();
        ledger
            .seal(&draft("20.00"), &req("source-migration:2"))
            .unwrap()
            .hash
    };
    let db = home.path().join("fiscal.sqlite");
    let (rows_before, head_before) = {
        let conn = rusqlite::Connection::open(&db).unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT number, prev_hash, hash, canonical_bytes
                 FROM vouchers ORDER BY number",
            )
            .unwrap();
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, u64>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                    r.get::<_, Vec<u8>>(3)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        drop(stmt);
        let head = conn
            .query_row(
                "SELECT head_num, head_hash FROM chain_head WHERE series='main'",
                [],
                |r| Ok((r.get::<_, u64>(0)?, r.get::<_, Vec<u8>>(1)?)),
            )
            .unwrap();
        conn.execute_batch(
            "ALTER TABLE vouchers DROP COLUMN source_fingerprint;
             ALTER TABLE period_transition DROP COLUMN receipt_unique_identifier;
             ALTER TABLE period_transition DROP COLUMN receipt_accepted_timestamp;
             ALTER TABLE period_transition DROP COLUMN manual_channel;
             ALTER TABLE period_transition DROP COLUMN manual_evidence_ref;
             ALTER TABLE period_transition DROP COLUMN remote_status_evidence_ref;
             PRAGMA user_version = 3;",
        )
        .unwrap();
        (rows, head)
    };

    let ledger = Ledger::open(home.path()).unwrap();
    let report = ledger.verify().unwrap();
    assert!(report.ok);
    assert_eq!(report.head_hash, old_head);

    let conn = rusqlite::Connection::open(&db).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT number, prev_hash, hash, canonical_bytes
             FROM vouchers ORDER BY number",
        )
        .unwrap();
    let rows_after = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, u64>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, Vec<u8>>(2)?,
                r.get::<_, Vec<u8>>(3)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    drop(stmt);
    let head_after = conn
        .query_row(
            "SELECT head_num, head_hash FROM chain_head WHERE series='main'",
            [],
            |r| Ok((r.get::<_, u64>(0)?, r.get::<_, Vec<u8>>(1)?)),
        )
        .unwrap();
    let legacy_fingerprint: Option<String> = conn
        .query_row(
            "SELECT source_fingerprint FROM vouchers WHERE number=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        rows_after, rows_before,
        "migration changed hashed voucher bytes"
    );
    assert_eq!(
        head_after, head_before,
        "migration changed chain head bytes"
    );
    assert_eq!(legacy_fingerprint, None);
}

#[test]
fn seal_is_idempotent_on_natural_key() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();

    let first = ledger.seal(&draft("50.00"), &req("co:same")).unwrap();
    let replay = ledger.seal(&draft("50.00"), &req("co:same")).unwrap();

    assert!(!first.replayed);
    assert!(replay.replayed, "second seal with same natural_key replays");
    assert_eq!(first.number, replay.number);
    assert_eq!(first.hash, replay.hash);
    assert_eq!(ledger.verify().unwrap().voucher_count, 1);
}

#[test]
fn refused_write_is_logged_and_surfaced() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();

    // Account 9999 is not in the chart → refused write.
    let d = Date::parse("2026-03-14").unwrap();
    let bad = VoucherDraft::new(
        d,
        d,
        "unknown account".into(),
        vec![
            posting("9999", "10.00", Side::Debit),
            posting("1910", "10.00", Side::Credit),
        ],
        None,
    )
    .unwrap();

    let err = ledger.seal(&bad, &req("co:bad")).unwrap_err();
    assert_eq!(err.error_code(), "ACCOUNT_NOT_FOUND");
    assert_eq!(
        ledger.rejection_count().unwrap(),
        1,
        "refusal is logged (O-25)"
    );
    // No voucher was written.
    assert_eq!(ledger.verify().unwrap().voucher_count, 0);
}

#[test]
fn artifact_attach_roundtrips_and_gates_seal() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();

    let bytes = b"%PDF-1.4 fake receipt bytes";
    let addr = ledger.attach_artifact(bytes).unwrap();

    let d = Date::parse("2026-03-14").unwrap();
    let with_attachment = VoucherDraft::new(
        d,
        d,
        "receipt with attachment".into(),
        vec![
            posting("7690", "10.00", Side::Debit),
            posting("1910", "10.00", Side::Credit),
        ],
        Some(addr),
    )
    .unwrap();

    // validate sees the attachment as present
    let report = ledger.validate(&with_attachment).unwrap();
    assert!(report.attachment_present);
    let sealed = ledger.seal(&with_attachment, &req("co:att")).unwrap();
    assert_eq!(sealed.number, 1);
}

#[test]
fn idempotency_conflict_on_natural_key_reuse() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();

    // Same natural_key, DIFFERENT draft content → must be refused, not a silent
    // replay of the old voucher (silent data loss).
    ledger.seal(&draft("100.00"), &req("acme:key")).unwrap();
    let err = ledger.seal(&draft("999.00"), &req("acme:key")).unwrap_err();
    assert_eq!(err.error_code(), "IDEMPOTENCY_KEY_REUSE");
    // The refusal is logged, and no second voucher was written.
    assert_eq!(ledger.rejection_count().unwrap(), 1);
    assert_eq!(ledger.verify().unwrap().voucher_count, 1);

    // Same key + same content still replays cleanly.
    let replay = ledger.seal(&draft("100.00"), &req("acme:key")).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.number, 1);
}

#[test]
fn verify_detects_tampered_chain_head_hash() {
    let home = tempfile::tempdir().unwrap();
    {
        let ledger = Ledger::init(home.path()).unwrap();
        ledger.seal(&draft("10.00"), &req("k:1")).unwrap();
        ledger.seal(&draft("20.00"), &req("k:2")).unwrap();
        assert!(ledger.verify().unwrap().ok);
    }
    // Corrupt only the materialized chain-head hash (leaving the voucher rows
    // intact) — the old verify would have missed this.
    {
        let conn = rusqlite::Connection::open(home.path().join("fiscal.sqlite")).unwrap();
        conn.execute(
            "UPDATE chain_head SET head_hash = ?1 WHERE series = 'main'",
            rusqlite::params![vec![0xAAu8; 32]],
        )
        .unwrap();
    }
    let ledger = Ledger::open(home.path()).unwrap();
    let err = ledger.verify().unwrap_err();
    assert_eq!(err.error_code(), "HASH_CHAIN_BROKEN");
}

#[test]
fn verify_detects_tampered_canonical_bytes() {
    let home = tempfile::tempdir().unwrap();
    {
        let ledger = Ledger::init(home.path()).unwrap();
        ledger.seal(&draft("10.00"), &req("k:1")).unwrap();
    }
    // Reorder/whitespace the stored canonical bytes: still valid JSON, but not
    // byte-equal to the frozen encoding → NON_CANONICAL_WIRE (ADR-2 read path).
    {
        let conn = rusqlite::Connection::open(home.path().join("fiscal.sqlite")).unwrap();
        let bytes: Vec<u8> = conn
            .query_row(
                "SELECT canonical_bytes FROM vouchers WHERE number = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let mut tampered = b"  ".to_vec();
        tampered.extend_from_slice(&bytes);
        conn.execute(
            "UPDATE vouchers SET canonical_bytes = ?1 WHERE number = 1",
            rusqlite::params![tampered],
        )
        .unwrap();
    }
    let ledger = Ledger::open(home.path()).unwrap();
    let err = ledger.verify().unwrap_err();
    assert_eq!(err.error_code(), "NON_CANONICAL_WIRE");
}

#[test]
fn verify_stays_consistent_under_concurrent_seals() {
    // A concurrent seal must never cause a spurious HASH_CHAIN_BROKEN: verify
    // reads one snapshot (ADR-1). Run a background sealer while verifying.
    let home = tempfile::tempdir().unwrap();
    let _seed = Ledger::init(home.path()).unwrap();
    drop(_seed);
    let home_path = home.path().to_path_buf();

    let sealer_home = home_path.clone();
    let sealer = thread::spawn(move || {
        let ledger = Ledger::open(&sealer_home).unwrap();
        for j in 0..60 {
            ledger
                .seal(&draft("1.00"), &req(&format!("s:{j}")))
                .unwrap();
        }
    });

    let verifier = Ledger::open(&home_path).unwrap();
    for _ in 0..200 {
        // Every call must succeed; count is monotonic but never inconsistent.
        verifier
            .verify()
            .expect("verify must never spuriously fail");
    }
    sealer.join().unwrap();
    assert_eq!(verifier.verify().unwrap().voucher_count, 60);
}

#[test]
fn concurrent_seal_is_atomic_gapless_and_chained() {
    const THREADS: usize = 8;
    const PER_THREAD: usize = 20;
    let total = (THREADS * PER_THREAD) as u64;

    let home = tempfile::tempdir().unwrap();
    // Initialize once up front so masterdata/fiscal exist before the race.
    let _seed = Ledger::init(home.path()).unwrap();
    drop(_seed);

    let home_path = home.path().to_path_buf();
    let numbers = Arc::new(Mutex::new(Vec::<u64>::new()));
    let hashes = Arc::new(Mutex::new(Vec::<String>::new()));

    let mut handles = Vec::new();
    for tid in 0..THREADS {
        let home_path = home_path.clone();
        let numbers = Arc::clone(&numbers);
        let hashes = Arc::clone(&hashes);
        handles.push(thread::spawn(move || {
            // Each thread has its OWN Ledger → own SQLite connections racing on
            // the shared DB file. This is the real ADR-1 concurrency bet.
            let ledger = Ledger::open(&home_path).unwrap();
            for j in 0..PER_THREAD {
                let natural_key = format!("co:t{tid}-{j}");
                let sealed = ledger
                    .seal(&draft("12.34"), &req(&natural_key))
                    .unwrap_or_else(|e| panic!("seal failed for {natural_key}: {e}"));
                numbers.lock().unwrap().push(sealed.number);
                hashes.lock().unwrap().push(sealed.hash);
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    // Gapless: the multiset of allocated numbers is exactly {1..=total}.
    let mut nums = numbers.lock().unwrap().clone();
    nums.sort_unstable();
    assert_eq!(nums.len() as u64, total);
    let expected: Vec<u64> = (1..=total).collect();
    assert_eq!(nums, expected, "numbers must be gapless and unique");

    // Unique hashes: no two vouchers collide.
    let unique: HashSet<_> = hashes.lock().unwrap().iter().cloned().collect();
    assert_eq!(
        unique.len() as u64,
        total,
        "all voucher hashes are distinct"
    );

    // The whole chain re-verifies (byte-equality read path + hash recompute).
    let ledger = Ledger::open(&home_path).unwrap();
    let report = ledger.verify().unwrap();
    assert!(report.ok);
    assert_eq!(report.voucher_count, total);
    assert_eq!(report.head_number, total);
    assert_eq!(ledger.rejection_count().unwrap(), 0);
}
