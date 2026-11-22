use aggountant_domain::{vat_expand, PolicySet, VatExpandInput, VatTreatment};
use aggountant_ledger::{Ledger, SealRequest};
use aggountant_model::{AccountId, Currency, Date, Money};
use rusqlite::Connection;

fn expanded_draft() -> aggountant_model::VoucherDraft {
    let d = Date::parse("2026-02-15").unwrap();
    vat_expand(&VatExpandInput {
        amount: Money::parse("125.50", Currency::new("EUR").unwrap()).unwrap(),
        net_account: AccountId::new("7690").unwrap(),
        contra_account: AccountId::new("1910").unwrap(),
        treatment: VatTreatment::DomesticInput {
            rate_class: "domestic-standard".into(),
        },
        document_date: d,
        posting_date: d,
        description: "VAT purchase".into(),
        attachment_ref: None,
    })
    .unwrap()
}

#[test]
fn ledger_preserves_sealed_vat_provenance() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let draft = expanded_draft();
    let sealed = ledger
        .seal(
            &draft,
            &SealRequest {
                natural_key: "vat-provenance".into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
    let view = ledger.voucher(sealed.number).unwrap();
    assert_eq!(view.body.vat_expansion(), draft.vat_expansion());
    assert!(ledger.verify().unwrap().ok);

    let canonical: Vec<u8> = Connection::open(home.path().join("fiscal.sqlite"))
        .unwrap()
        .query_row(
            "SELECT canonical_bytes FROM vouchers WHERE number = ?1",
            [sealed.number],
            |row| row.get(0),
        )
        .unwrap();
    let tampered = String::from_utf8(canonical)
        .unwrap()
        .replacen("\"25.5\"", "\"24.0\"", 1)
        .into_bytes();
    Connection::open(home.path().join("fiscal.sqlite"))
        .unwrap()
        .execute(
            "UPDATE vouchers SET canonical_bytes = ?1 WHERE number = ?2",
            rusqlite::params![tampered, sealed.number],
        )
        .unwrap();
    assert!(
        ledger.verify().is_err(),
        "persisted sealed bytes must be hash-checked"
    );
}

#[test]
fn ledger_refuses_postings_that_disagree_with_expansion_provenance() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let mut value = serde_json::to_value(expanded_draft()).unwrap();
    let postings = value["postings"].as_array_mut().unwrap();
    postings[0]["amount"]["amount"] = serde_json::json!("100.10");
    postings[1]["amount"]["amount"] = serde_json::json!("25.40");
    let draft = serde_json::from_value(value).unwrap();
    let error = ledger
        .seal(
            &draft,
            &SealRequest {
                natural_key: "false-expansion".into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap_err();
    assert_eq!(error.error_code(), "VAT_STRUCTURE");
    assert!(error.to_string().contains("does not exactly reproduce"));
}

#[test]
fn ledger_refuses_a_seal_policy_that_disagrees_with_expansion_provenance() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let policy = PolicySet {
        id: "different-policy".into(),
    };
    let error = ledger
        .seal(
            &expanded_draft(),
            &SealRequest {
                natural_key: "wrong-policy".into(),
                actor: "test".into(),
                policy_set: policy,
            },
        )
        .unwrap_err();
    assert_eq!(error.error_code(), "INVALID_VAT_EXPANSION");
    assert!(error.to_string().contains("differs from seal policy_set"));
}

/// Every `vat_expansion.treatment` object of type `no-vat` in a fiscal store's
/// sealed bodies, as raw JSON.
fn stored_no_vat_treatments(home: &std::path::Path) -> Vec<serde_json::Value> {
    let connection = Connection::open(home.join("fiscal.sqlite")).unwrap();
    let mut statement = connection
        .prepare("SELECT canonical_bytes FROM vouchers ORDER BY number")
        .unwrap();
    let bodies: Vec<Vec<u8>> = statement
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    bodies
        .iter()
        .filter_map(|bytes| {
            let body: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            let treatment = body.get("vat_expansion")?.get("treatment")?.clone();
            (treatment["type"] == "no-vat").then_some(treatment)
        })
        .collect()
}

#[test]
fn a_stored_no_vat_expansion_carries_no_members_and_a_member_never_decoded() {
    // The write path: a no-vat expansion sealed through the ledger stores its
    // treatment as the tag alone. A scan of the synthetic store finds no member.
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let d = Date::parse("2026-02-15").unwrap();
    let draft = vat_expand(&VatExpandInput {
        amount: Money::parse("40.00", Currency::new("EUR").unwrap()).unwrap(),
        net_account: AccountId::new("7690").unwrap(),
        contra_account: AccountId::new("1910").unwrap(),
        treatment: VatTreatment::NoVat,
        document_date: d,
        posting_date: d,
        description: "Fictional exempt purchase".into(),
        attachment_ref: None,
    })
    .unwrap();
    let sealed = ledger
        .seal(
            &draft,
            &SealRequest {
                natural_key: "no-vat-provenance".into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
    let treatments = stored_no_vat_treatments(home.path());
    assert_eq!(treatments, vec![serde_json::json!({"type":"no-vat"})]);

    // A body whose no-vat treatment carries a member is not what the ledger writes
    // and never decodes: the strict decoder refuses it before any hash check.
    let canonical: Vec<u8> = Connection::open(home.path().join("fiscal.sqlite"))
        .unwrap()
        .query_row(
            "SELECT canonical_bytes FROM vouchers WHERE number = ?1",
            [sealed.number],
            |row| row.get(0),
        )
        .unwrap();
    let canonical = String::from_utf8(canonical).unwrap();
    assert_eq!(canonical.matches(r#"{"type":"no-vat"}"#).count(), 1);
    let with_member = canonical.replacen(
        r#"{"type":"no-vat"}"#,
        r#"{"rate_class":"domestic-standard","type":"no-vat"}"#,
        1,
    );
    assert!(aggountant_wire::decode_voucher_body(with_member.as_bytes()).is_err());
}
