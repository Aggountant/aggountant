//! Bank-transaction identity across overlapping statement exports (issue
//! `camt-event-identity`): the atomic batch seal, and homes whose bank imports
//! were sealed before bank events were recorded.

use aggountant_domain::PolicySet;
use aggountant_ledger::{
    BankBatch, BankEntryFacts, BankEventBooking, BankEventConflictCode, BankEventEntry,
    BankEventOutcome, CandidateEvidence, Ledger, LegacyBankIdentity, SealRequest,
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

/// The bank facts of a money-out entry of `amount` on account 1910.
fn facts(bank_ref: &str, date: &str, amount: &str) -> BankEntryFacts {
    BankEntryFacts {
        bank_ref: bank_ref.to_string(),
        bank_account: AccountId::new("1910").unwrap(),
        amount: Money::parse(amount, Currency::new("EUR").unwrap()).unwrap(),
        bank_side: Side::Credit,
        booking_date: Date::parse(date).unwrap(),
        value_date: None,
    }
}

/// A new-voucher entry; its facts are read from the draft's 1910 leg.
fn entry(bank_ref: &str, fingerprint: &str, draft: VoucherDraft) -> BankEventEntry {
    new_entry(bank_ref, fingerprint, draft, Vec::new())
}

fn new_entry(
    bank_ref: &str,
    fingerprint: &str,
    draft: VoucherDraft,
    not_booked_by: Vec<u64>,
) -> BankEventEntry {
    let bank_leg = draft
        .postings()
        .iter()
        .find(|p| p.account.as_str() == "1910")
        .unwrap();
    BankEventEntry {
        event_key: key(bank_ref),
        source_fingerprint: fingerprint.to_string(),
        facts: facts(
            bank_ref,
            &draft.posting_date().to_string(),
            &bank_leg.amount.amount_str(),
        ),
        booking: BankEventBooking::New {
            draft,
            not_booked_by,
        },
    }
}

/// An entry bound to the existing voucher `number`.
fn bound_entry(
    bank_ref: &str,
    fingerprint: &str,
    facts: BankEntryFacts,
    number: u64,
) -> BankEventEntry {
    BankEventEntry {
        event_key: key(bank_ref),
        source_fingerprint: fingerprint.to_string(),
        facts,
        booking: BankEventBooking::Existing {
            voucher_number: number,
        },
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
        event_key_prefix: KEY_PREFIX,
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

/// Seal a voucher for a bank payment the way a company without the camt import
/// books it: `voucher seal` under a key of its own choosing.
fn seal_by_hand(ledger: &Ledger, natural_key: &str, description: &str, d: &VoucherDraft) -> u64 {
    let d = VoucherDraft::new(
        d.document_date(),
        d.posting_date(),
        description.to_string(),
        d.postings().to_vec(),
        d.attachment_ref(),
    )
    .unwrap();
    ledger
        .seal(
            &d,
            &SealRequest {
                natural_key: natural_key.into(),
                actor: "bookkeeper".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap()
        .number
}

#[test]
fn a_voucher_sealed_by_hand_stops_a_new_entry_it_may_book() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let policy = policy_set();
    let statement = ledger.attach_artifact(b"<statement/>").unwrap();
    let manual = ledger
        .attach_artifact(b"<statement attached by hand/>")
        .unwrap();
    // #1 names the bank reference; #2 matches by account, amount and a date
    // three days off; #3 is another payment.
    let by_ref = seal_by_hand(
        &ledger,
        "bank:REF-B",
        "Office rent",
        &draft("2026-12-30", "20.00", "7690", manual),
    );
    let by_amount = seal_by_hand(
        &ledger,
        "receipt-17",
        "Office chair",
        &draft("2026-12-27", "45.00", "7690", manual),
    );
    seal_by_hand(
        &ledger,
        "receipt-18",
        "Paper",
        &draft("2026-12-29", "9.00", "7690", manual),
    );
    let rejections = ledger.rejection_count().unwrap();

    let entries = [
        entry(
            "REF-B",
            "fp-b",
            draft("2026-12-30", "20.00", "7690", statement),
        ),
        entry(
            "REF-C",
            "fp-c",
            draft("2026-12-30", "45.00", "7690", statement),
        ),
        entry(
            "REF-D",
            "fp-d",
            draft("2026-12-30", "12.00", "7690", statement),
        ),
    ];
    let request = BankBatch {
        policy_set: &policy,
        ..batch(statement, &entries)
    };
    let preview = ledger.preview_bank_batch(&request).unwrap();
    let found: Vec<(String, u64, Option<CandidateEvidence>)> = preview
        .conflicts
        .iter()
        .map(|c| (c.event_key.clone(), c.voucher_number, c.evidence))
        .collect();
    assert_eq!(
        found,
        [
            (key("REF-B"), by_ref, Some(CandidateEvidence::BankReference)),
            (
                key("REF-C"),
                by_amount,
                Some(CandidateEvidence::AccountAmountDate)
            ),
        ]
    );
    assert!(preview
        .conflicts
        .iter()
        .all(|c| c.code == BankEventConflictCode::BankEntryPossiblyBooked));

    let report = ledger.seal_bank_batch(&request).unwrap();
    assert!(!report.written);
    assert_eq!(report.conflicts.len(), 2);
    assert_eq!(ledger.rejection_count().unwrap(), rejections + 2);
    assert_eq!(ledger.verify().unwrap().voucher_count, 3);
    assert!(ledger.bank_source_observations(None).unwrap().is_empty());
}

#[test]
fn a_bank_reference_matches_only_as_a_whole_token() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let policy = policy_set();
    let statement = ledger.attach_artifact(b"<statement/>").unwrap();
    seal_by_hand(
        &ledger,
        "bank:REF-10",
        "Rent",
        &draft("2026-11-02", "20.00", "7690", statement),
    );
    let entries = [entry(
        "REF-1",
        "fp-1",
        draft("2026-12-30", "5.00", "7690", statement),
    )];
    let preview = ledger
        .preview_bank_batch(&BankBatch {
            policy_set: &policy,
            ..batch(statement, &entries)
        })
        .unwrap();
    assert!(preview.conflicts.is_empty(), "{:?}", preview.conflicts);
}

#[test]
fn a_voucher_ruled_out_for_an_entry_lets_it_seal() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let policy = policy_set();
    let statement = ledger.attach_artifact(b"<statement/>").unwrap();
    let other = seal_by_hand(
        &ledger,
        "receipt-17",
        "Office chair",
        &draft("2026-12-30", "45.00", "7690", statement),
    );
    let entries = [new_entry(
        "REF-C",
        "fp-c",
        draft("2026-12-30", "45.00", "7690", statement),
        vec![other],
    )];
    let report = ledger
        .seal_bank_batch(&BankBatch {
            policy_set: &policy,
            ..batch(statement, &entries)
        })
        .unwrap();
    assert!(report.written, "{:?}", report.conflicts);
    assert_eq!(report.count(BankEventOutcome::NewVoucher), 1);
}

#[test]
fn an_entry_binds_to_the_voucher_that_books_it_without_changing_it() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let policy = policy_set();
    let manual = ledger
        .attach_artifact(b"<statement attached by hand/>")
        .unwrap();
    let booked = draft("2026-12-30", "20.00", "7690", manual);
    let number = seal_by_hand(&ledger, "bank:REF-B", "Office rent", &booked);
    let hash_before = ledger.voucher(number).unwrap().voucher_hash;

    // The binding also takes the voucher out of the candidates of the batch's
    // other entry with the same amount and date.
    let statement = ledger.attach_artifact(b"<statement/>").unwrap();
    let entries = [
        bound_entry(
            "REF-B",
            "fp-b",
            facts("REF-B", "2026-12-30", "20.00"),
            number,
        ),
        entry(
            "REF-E",
            "fp-e",
            draft("2026-12-30", "20.00", "7690", statement),
        ),
    ];
    let request = BankBatch {
        policy_set: &policy,
        ..batch(statement, &entries)
    };
    let preview = ledger.preview_bank_batch(&request).unwrap();
    assert!(preview.conflicts.is_empty(), "{:?}", preview.conflicts);
    assert_eq!(
        preview.results[0].outcome,
        BankEventOutcome::BoundExistingVoucher
    );
    assert_eq!(preview.results[0].voucher.as_ref().unwrap().number, number);
    assert!(ledger.bank_source_observations(None).unwrap().is_empty());

    let report = ledger.seal_bank_batch(&request).unwrap();
    assert!(report.written);
    assert_eq!(report.count(BankEventOutcome::BoundExistingVoucher), 1);
    assert_eq!(report.count(BankEventOutcome::NewVoucher), 1);
    let verify = ledger.verify().unwrap();
    assert!(verify.ok, "{verify:?}");
    assert_eq!(verify.voucher_count, 2);
    assert_eq!(ledger.voucher(number).unwrap().voucher_hash, hash_before);
    let observed = ledger.bank_source_observations(None).unwrap();
    assert_eq!(observed.len(), 2);
    assert_eq!(observed[0].voucher_number, number);
    assert_eq!(observed[0].event_key, key("REF-B"));

    // Another export with the same binding is a duplicate observation.
    let overlap = ledger.attach_artifact(b"<overlapping statement/>").unwrap();
    let again = [bound_entry(
        "REF-B",
        "fp-b",
        facts("REF-B", "2026-12-30", "20.00"),
        number,
    )];
    let report = ledger
        .seal_bank_batch(&BankBatch {
            policy_set: &policy,
            ..batch(overlap, &again)
        })
        .unwrap();
    assert!(report.written);
    assert_eq!(
        report.count(BankEventOutcome::DuplicateSourceObservation),
        1
    );

    // A new-voucher assignment for the bound transaction books nothing twice.
    let assigned = [entry(
        "REF-B",
        "fp-b",
        draft("2026-12-30", "20.00", "7690", overlap),
    )];
    let report = ledger
        .preview_bank_batch(&BankBatch {
            policy_set: &policy,
            ..batch(overlap, &assigned)
        })
        .unwrap();
    assert_eq!(
        report.conflicts[0].code,
        BankEventConflictCode::PostingProjectionChanged
    );

    // The event key is taken: a seal under it replays nothing new.
    let sealed = ledger.seal(
        &draft("2026-12-30", "20.00", "7690", overlap),
        &SealRequest {
            natural_key: key("REF-B"),
            actor: "bookkeeper".into(),
            policy_set: PolicySet::default(),
        },
    );
    let err = sealed.unwrap_err();
    assert_eq!(err.error_code(), "IDEMPOTENCY_KEY_REUSE", "{err}");
    assert_eq!(ledger.verify().unwrap().voucher_count, 2);
}

#[test]
fn a_binding_to_a_voucher_that_does_not_book_the_entry_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let policy = policy_set();
    let manual = ledger
        .attach_artifact(b"<statement attached by hand/>")
        .unwrap();
    let rent = seal_by_hand(
        &ledger,
        "bank:REF-B",
        "Office rent",
        &draft("2026-12-30", "20.00", "7690", manual),
    );
    let statement = ledger.attach_artifact(b"<statement/>").unwrap();
    let mismatch = |entries: &[BankEventEntry]| {
        let report = ledger
            .seal_bank_batch(&BankBatch {
                policy_set: &policy,
                ..batch(statement, entries)
            })
            .unwrap();
        assert!(!report.written);
        assert_eq!(report.conflicts.len(), 1, "{:?}", report.conflicts);
        assert_eq!(
            report.conflicts[0].code,
            BankEventConflictCode::BankVoucherMismatch
        );
        report.conflicts[0].message.clone()
    };
    // Another amount, a date out of tolerance, money in instead of out.
    mismatch(&[bound_entry(
        "REF-B",
        "fp-b",
        facts("REF-B", "2026-12-30", "21.00"),
        rent,
    )]);
    mismatch(&[bound_entry(
        "REF-B",
        "fp-b",
        facts("REF-B", "2026-12-08", "20.00"),
        rent,
    )]);
    let mut money_in = facts("REF-B", "2026-12-30", "20.00");
    money_in.bank_side = Side::Debit;
    mismatch(&[bound_entry("REF-B", "fp-b", money_in, rent)]);
    // One voucher cannot book two transactions.
    let message = mismatch(&[
        bound_entry("REF-B", "fp-b", facts("REF-B", "2026-12-30", "20.00"), rent),
        bound_entry("REF-F", "fp-f", facts("REF-F", "2026-12-30", "20.00"), rent),
    ]);
    assert!(
        message.contains("already bound to another bank transaction"),
        "{message}"
    );
    // A voucher the import sealed for another transaction.
    let imported = seal_legacy(
        &ledger,
        "REF-G",
        &draft("2026-12-30", "20.00", "7690", manual),
    );
    let message = mismatch(&[bound_entry(
        "REF-B",
        "fp-b",
        facts("REF-B", "2026-12-30", "20.00"),
        imported,
    )]);
    assert!(message.contains("sealed by the bank import"), "{message}");
    // A number that is not sealed.
    let err = ledger
        .preview_bank_batch(&BankBatch {
            policy_set: &policy,
            ..batch(
                statement,
                &[bound_entry(
                    "REF-B",
                    "fp-b",
                    facts("REF-B", "2026-12-30", "20.00"),
                    9,
                )],
            )
        })
        .unwrap_err();
    assert_eq!(err.error_code(), "VOUCHER_NOT_FOUND");
    assert!(ledger.bank_source_observations(None).unwrap().is_empty());
    assert_eq!(ledger.verify().unwrap().voucher_count, 2);
}
