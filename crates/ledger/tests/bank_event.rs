//! Bank-transaction identity across overlapping statement exports (issue
//! `camt-event-identity`): the atomic batch seal, and homes whose bank imports
//! were sealed before bank events were recorded.

use aggountant_domain::PolicySet;
use aggountant_ledger::{
    BankBatch, BankEventConflictCode, BankEventEntry, BankEventOutcome, Ledger, LegacyBankIdentity,
    SealRequest,
};
use aggountant_model::{AccountId, Currency, Date, Hash, Money, Posting, Side, VoucherDraft};

const KEY_PREFIX: &str = "bank-import:";

fn posting(account: &str, amount: &str, side: Side) -> Posting {
    Posting {
        account: AccountId::new(account).unwrap(),
        amount: Money::parse(amount, Currency::new("EUR").unwrap()).unwrap(),
        side,
        vat_code: None,
    }
}

fn draft(date: &str, amount: &str, contra: &str, statement: Hash) -> VoucherDraft {
    let date = Date::parse(date).unwrap();
    VoucherDraft::new(
        date,
        date,
        format!("Bank payment {amount}"),
        vec![
            posting(contra, amount, Side::Debit),
            posting("1910", amount, Side::Credit),
        ],
        Some(statement),
    )
    .unwrap()
}

fn key(bank_ref: &str) -> String {
    format!("bank-import:holvi:FI2112345600000785:{bank_ref}")
}

fn entry(bank_ref: &str, fingerprint: &str, draft: VoucherDraft) -> BankEventEntry {
    BankEventEntry {
        event_key: key(bank_ref),
        source_fingerprint: fingerprint.to_string(),
        draft,
    }
}

fn batch<'a>(statement: Hash, entries: &'a [BankEventEntry]) -> BankBatch<'a> {
    BankBatch {
        artifact: statement,
        entries,
        unbooked: &[],
        backfill: &[],
        actor: "importer",
        policy_set: "fi-default-2026",
    }
}

fn policy_set() -> String {
    PolicySet::default().id
}

/// Seal `bank_ref` the way the import did before bank events: `Ledger::seal`
/// under the event key, with the statement as attachment and no fingerprint.
fn seal_legacy(ledger: &Ledger, bank_ref: &str, draft: &VoucherDraft) -> u64 {
    ledger
        .seal(
            draft,
            &SealRequest {
                natural_key: key(bank_ref),
                actor: "importer".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap()
        .number
}

/// Turn the home's fiscal store back into the schema the previous release wrote
/// (v8): the v9 migration only adds the two bank-event tables.
fn downgrade_to_v8(home: &std::path::Path) {
    let conn = rusqlite::Connection::open(home.join("fiscal.sqlite")).unwrap();
    conn.execute_batch(
        "DROP INDEX bank_source_observation_by_artifact;
         DROP TABLE bank_source_observation;
         DROP TABLE bank_event;
         PRAGMA user_version = 8;",
    )
    .unwrap();
}

#[test]
fn a_home_written_by_the_previous_schema_opens_verifies_and_backfills() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let ledger = Ledger::init(home).unwrap();
    let statement_a = ledger.attach_artifact(b"<statement A/>").unwrap();
    let first = draft("2026-12-30", "20.00", "7690", statement_a);
    let second = draft("2026-12-31", "30.00", "7690", statement_a);
    assert_eq!(seal_legacy(&ledger, "REF-B", &first), 1);
    assert_eq!(seal_legacy(&ledger, "REF-C", &second), 2);
    let before: Vec<String> = (1..=2)
        .map(|n| ledger.voucher(n).unwrap().voucher_hash)
        .collect();
    drop(ledger);
    downgrade_to_v8(home);

    let ledger = Ledger::open(home).unwrap();
    let report = ledger.verify().unwrap();
    assert!(report.ok, "{report:?}");
    let after: Vec<String> = (1..=2)
        .map(|n| ledger.voucher(n).unwrap().voucher_hash)
        .collect();
    assert_eq!(before, after, "sealed vouchers changed on migration");

    let legacy = ledger.bank_vouchers_without_event(KEY_PREFIX).unwrap();
    assert_eq!(legacy.len(), 2);
    assert_eq!(legacy[0].event_key, key("REF-B"));
    assert_eq!(legacy[0].attachment_ref, Some(statement_a));

    // Statement B repeats REF-B and adds REF-D.
    let statement_b = ledger.attach_artifact(b"<statement B/>").unwrap();
    let entries = [
        entry(
            "REF-B",
            "fp-b",
            draft("2026-12-30", "20.00", "7690", statement_b),
        ),
        entry(
            "REF-D",
            "fp-d",
            draft("2027-01-02", "5.00", "7690", statement_b),
        ),
    ];
    let backfill = [
        LegacyBankIdentity {
            event_key: key("REF-B"),
            source_fingerprint: "fp-b".into(),
        },
        LegacyBankIdentity {
            event_key: key("REF-C"),
            source_fingerprint: "fp-c".into(),
        },
    ];
    let policy = policy_set();
    let request = BankBatch {
        backfill: &backfill,
        policy_set: &policy,
        ..batch(statement_b, &entries)
    };
    let report = ledger.seal_bank_batch(&request).unwrap();
    assert!(report.written);
    assert_eq!(report.backfilled_count, 2);
    assert_eq!(
        report.count(BankEventOutcome::DuplicateSourceObservation),
        1
    );
    assert_eq!(report.count(BankEventOutcome::NewVoucher), 1);
    assert_eq!(report.results[0].voucher.as_ref().unwrap().number, 1);
    assert_eq!(report.results[1].voucher.as_ref().unwrap().number, 3);
    assert!(ledger.verify().unwrap().ok);
    assert!(ledger
        .bank_vouchers_without_event(KEY_PREFIX)
        .unwrap()
        .is_empty());
    let observed = ledger.bank_source_observations(None).unwrap();
    assert_eq!(observed.len(), 4, "{observed:?}");
    assert_eq!(
        ledger
            .bank_source_observations(Some(&statement_a))
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn a_conflict_late_in_the_batch_writes_nothing_and_is_audited() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let policy = policy_set();
    let statement_a = ledger.attach_artifact(b"<statement A/>").unwrap();
    let first = [entry(
        "REF-B",
        "fp-b",
        draft("2026-12-30", "20.00", "7690", statement_a),
    )];
    let sealed = ledger
        .seal_bank_batch(&BankBatch {
            policy_set: &policy,
            ..batch(statement_a, &first)
        })
        .unwrap();
    assert_eq!(sealed.count(BankEventOutcome::NewVoucher), 1);

    let statement_b = ledger.attach_artifact(b"<statement B/>").unwrap();
    let rejections = ledger.rejection_count().unwrap();
    for (fingerprint, contra, code) in [
        (
            "fp-b-changed",
            "7690",
            BankEventConflictCode::SourceFactsChanged,
        ),
        (
            "fp-b",
            "7600",
            BankEventConflictCode::PostingProjectionChanged,
        ),
    ] {
        let entries = [
            entry(
                "REF-D",
                "fp-d",
                draft("2027-01-02", "5.00", "7690", statement_b),
            ),
            entry(
                "REF-B",
                fingerprint,
                draft("2026-12-30", "20.00", contra, statement_b),
            ),
        ];
        let request = BankBatch {
            policy_set: &policy,
            ..batch(statement_b, &entries)
        };
        let preview = ledger.preview_bank_batch(&request).unwrap();
        assert_eq!(preview.conflicts.len(), 1);
        assert_eq!(preview.conflicts[0].code, code);
        let report = ledger.seal_bank_batch(&request).unwrap();
        assert!(!report.written);
        assert_eq!(report.conflicts[0].code, code);
        assert_eq!(report.conflicts[0].voucher_number, 1);
        // The new entry before the conflict was not sealed either.
        assert!(report.results[0].voucher.is_none());
    }
    assert_eq!(ledger.rejection_count().unwrap(), rejections + 2);
    let verify = ledger.verify().unwrap();
    assert!(verify.ok);
    assert_eq!(verify.voucher_count, 1);
    assert_eq!(ledger.bank_source_observations(None).unwrap().len(), 1);
}

#[test]
fn a_preview_writes_nothing() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let policy = policy_set();
    // The statement is not stored yet: a preview addresses it as pending.
    let statement = ledger.artifact_address(b"<statement/>");
    let entries = [entry(
        "REF-B",
        "fp-b",
        draft("2026-12-30", "20.00", "7690", statement),
    )];
    let report = ledger
        .preview_bank_batch(&BankBatch {
            policy_set: &policy,
            ..batch(statement, &entries)
        })
        .unwrap();
    assert!(!report.written);
    assert_eq!(report.count(BankEventOutcome::NewVoucher), 1);
    assert!(report.results[0].voucher.is_none());
    assert_eq!(ledger.verify().unwrap().voucher_count, 0);
    assert!(ledger.bank_source_observations(None).unwrap().is_empty());
}
