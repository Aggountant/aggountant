use aggountant_domain::PolicySet;
use aggountant_ledger::{Ledger, LedgerError, SealRequest, VoucherListQuery};
use aggountant_model::{AccountId, Currency, Date, Hash, Money, Posting, Side, VoucherDraft};

fn posting(account: &str, amount: &str, side: Side) -> Posting {
    Posting {
        account: AccountId::new(account).unwrap(),
        amount: Money::parse(amount, Currency::new("EUR").unwrap()).unwrap(),
        side,
        vat_code: None,
    }
}

fn draft(document: &str, posting_date: &str, attachment_ref: Option<Hash>) -> VoucherDraft {
    VoucherDraft::new(
        Date::parse(document).unwrap(),
        Date::parse(posting_date).unwrap(),
        "Fictional office service".into(),
        vec![
            posting("7690", "25.00", Side::Debit),
            posting("1910", "25.00", Side::Credit),
        ],
        attachment_ref,
    )
    .unwrap()
}

fn seal(ledger: &Ledger, key: &str, document: &str, posting_date: &str, attachment: Option<Hash>) {
    ledger
        .seal(
            &draft(document, posting_date, attachment),
            &SealRequest {
                natural_key: key.into(),
                actor: "fictional-agent".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
}

fn query(
    attachment_ref: Option<Hash>,
    posting_from: Option<&str>,
    posting_to: Option<&str>,
    limit: usize,
) -> VoucherListQuery {
    VoucherListQuery {
        attachment_ref,
        posting_from: posting_from.map(|date| Date::parse(date).unwrap()),
        posting_to: posting_to.map(|date| Date::parse(date).unwrap()),
        limit,
    }
}

#[test]
fn show_binds_exact_body_number_hash_and_separate_provenance() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let attachment = ledger
        .attach_artifact(b"fictional attachment alpha")
        .unwrap();
    let expected_draft = draft("2026-01-14", "2026-01-16", Some(attachment));
    let sealed = ledger
        .seal(
            &expected_draft,
            &SealRequest {
                natural_key: "fictional:alpha".into(),
                actor: "fictional-agent".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();

    let shown = ledger.voucher(sealed.number).unwrap();
    assert_eq!(shown.number, sealed.number);
    assert_eq!(shown.voucher_hash, sealed.hash);
    assert_eq!(shown.body.number(), sealed.number);
    assert_eq!(shown.body.document_date(), expected_draft.document_date());
    assert_eq!(shown.body.posting_date(), expected_draft.posting_date());
    assert_eq!(shown.body.postings(), expected_draft.postings());
    assert_eq!(shown.body.attachment_ref(), Some(attachment));
    assert_eq!(shown.provenance.natural_key, "fictional:alpha");
    assert_eq!(shown.provenance.actor, "fictional-agent");
    assert_eq!(shown.provenance.chart_version, shown.body.chart_version());
}

#[test]
fn list_filters_with_and_semantics_keeps_shared_attachment_candidates_ordered() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let shared = ledger.attach_artifact(b"fictional shared source").unwrap();
    let other = ledger.attach_artifact(b"fictional other source").unwrap();
    seal(
        &ledger,
        "fictional:1",
        "2026-01-01",
        "2026-01-02",
        Some(shared),
    );
    seal(
        &ledger,
        "fictional:2",
        "2026-02-01",
        "2026-02-02",
        Some(other),
    );
    seal(
        &ledger,
        "fictional:3",
        "2026-03-01",
        "2026-03-02",
        Some(shared),
    );

    let shared_rows = ledger
        .list_vouchers(&query(Some(shared), None, None, 100))
        .unwrap();
    assert_eq!(shared_rows.count, 2);
    assert!(!shared_rows.truncated);
    assert_eq!(
        shared_rows
            .vouchers
            .iter()
            .map(|row| row.number)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert!(shared_rows
        .vouchers
        .iter()
        .all(|row| row.match_status == "possible_source_match"));

    let and_rows = ledger
        .list_vouchers(&query(
            Some(shared),
            Some("2026-02-01"),
            Some("2026-03-31"),
            100,
        ))
        .unwrap();
    assert_eq!(and_rows.count, 1);
    assert_eq!(and_rows.vouchers[0].number, 3);

    let zero = ledger
        .list_vouchers(&query(
            Some(other),
            Some("2026-03-01"),
            Some("2026-03-31"),
            100,
        ))
        .unwrap();
    assert_eq!(zero.count, 0);
    assert!(zero.vouchers.is_empty());
    assert!(!zero.truncated);
}

#[test]
fn list_limit_is_bounded_and_truncation_is_truthful() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    for number in 1..=101 {
        seal(
            &ledger,
            &format!("fictional:{number}"),
            "2026-04-01",
            "2026-04-02",
            None,
        );
    }
    let result = ledger
        .list_vouchers(&query(None, Some("2026-04-01"), Some("2026-04-30"), 100))
        .unwrap();
    assert_eq!(result.count, 100);
    assert_eq!(result.vouchers.len(), 100);
    assert!(result.truncated);
    assert_eq!(result.vouchers.first().unwrap().number, 1);
    assert_eq!(result.vouchers.last().unwrap().number, 100);
}

#[test]
fn invalid_queries_and_missing_voucher_are_read_only() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    seal(&ledger, "fictional:one", "2026-05-01", "2026-05-02", None);
    let before = ledger.verify().unwrap();
    let rejection_before = ledger.rejection_count().unwrap();

    for invalid in [
        query(None, None, None, 100),
        query(None, Some("2026-05-01"), None, 100),
        query(None, Some("2026-05-03"), Some("2026-05-01"), 100),
        query(None, Some("2026-05-01"), Some("2026-05-31"), 0),
        query(None, Some("2026-05-01"), Some("2026-05-31"), 101),
    ] {
        assert!(matches!(
            ledger.list_vouchers(&invalid),
            Err(LedgerError::InvalidVoucherRead(_))
        ));
    }
    assert!(matches!(
        ledger.voucher(0),
        Err(LedgerError::InvalidVoucherRead(_))
    ));
    assert!(matches!(
        ledger.voucher(99),
        Err(LedgerError::VoucherNotFound(_))
    ));

    let after = ledger.verify().unwrap();
    assert_eq!(after.voucher_count, before.voucher_count);
    assert_eq!(after.head_number, before.head_number);
    assert_eq!(after.head_hash, before.head_hash);
    assert_eq!(ledger.rejection_count().unwrap(), rejection_before);
}

#[test]
fn corrupt_or_missing_canonical_fields_fail_as_source_invariants() {
    for mutation in [
        "UPDATE vouchers SET canonical_bytes = CAST(json_remove(CAST(canonical_bytes AS TEXT), '$.posting_date') AS BLOB) WHERE number = 1",
        "UPDATE vouchers SET canonical_bytes = CAST(replace(CAST(canonical_bytes AS TEXT), '\"number\":1', '\"number\":2') AS BLOB) WHERE number = 1",
    ] {
        let home = tempfile::tempdir().unwrap();
        {
            let ledger = Ledger::init(home.path()).unwrap();
            seal(&ledger, "fictional:corrupt", "2026-06-01", "2026-06-02", None);
        }
        let connection = rusqlite::Connection::open(home.path().join("fiscal.sqlite")).unwrap();
        connection.execute(mutation, []).unwrap();
        drop(connection);

        let ledger = Ledger::open(home.path()).unwrap();
        let error = ledger.voucher(1).unwrap_err();
        assert_eq!(error.error_code(), "VOUCHER_READ_CORRUPT");
        assert!(matches!(error, LedgerError::VoucherReadCorrupt(_)));
    }
}
