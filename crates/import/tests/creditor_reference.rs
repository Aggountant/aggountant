//! Golden tests for the structured creditor reference (`RmtInf/Strd/
//! CdtrRefInf/Ref`) of a bank entry. They also pin that the reference leaves
//! every entry's identity (event key, source fingerprint, default description)
//! as the build before it computed it, because production homes hold vouchers
//! sealed from statements that carry the element.

use aggountant_import::{
    parse_camt, project_voucher, Assignment, ContraPosting, CreditorReference,
    CreditorReferenceKind,
};
use aggountant_model::AccountId;
use serde_json::Value;

const REFERENCES: &str = include_str!("fixtures/reference-camt053-sample.xml");
const REFERENCES_GOLDEN: &str = include_str!("fixtures/reference-camt053-sample.parsed.json");

#[test]
fn reference_statement_matches_its_golden_output() {
    let actual = serde_json::to_value(parse_camt(REFERENCES).unwrap()).unwrap();
    let expected: Value = serde_json::from_str(REFERENCES_GOLDEN).unwrap();
    assert_eq!(
        actual,
        expected,
        "parse output differs from the golden file:\n{}",
        serde_json::to_string_pretty(&actual).unwrap()
    );
}

#[test]
fn each_form_of_reference_is_classified_and_checked() {
    let stmt = parse_camt(REFERENCES).unwrap();
    let refs: Vec<&CreditorReference> = stmt
        .transactions
        .iter()
        .map(|t| t.creditor_reference.as_ref().unwrap())
        .collect();
    let summary: Vec<(&str, CreditorReferenceKind, bool, Option<&str>)> = refs
        .iter()
        .map(|r| {
            (
                r.value.as_str(),
                r.kind,
                r.valid,
                r.canonical.as_ref().map(|c| c.as_str()),
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            (
                "1234561",
                CreditorReferenceKind::Domestic,
                true,
                Some("1234561")
            ),
            (
                "RF18539007547034",
                CreditorReferenceKind::Rf,
                true,
                Some("RF18539007547034")
            ),
            // A zero-padded reference keeps its value and compares by the
            // canonical form without the zeros.
            (
                "00000000000012345672",
                CreditorReferenceKind::Domestic,
                true,
                Some("12345672")
            ),
            // A wrong check digit is shown, not refused or dropped.
            ("1234562", CreditorReferenceKind::Domestic, false, None),
            ("INV-2026-0617", CreditorReferenceKind::Other, false, None),
        ]
    );
}

#[test]
fn spaces_and_lowercase_rf_are_presentation() {
    let spaced = CreditorReference::read("12345 61".into());
    assert_eq!(spaced.value, "12345 61");
    assert_eq!(spaced.kind, CreditorReferenceKind::Domestic);
    assert_eq!(spaced.canonical.unwrap().as_str(), "1234561");

    let rf = CreditorReference::read("rf18 5390 0754 7034".into());
    assert_eq!(rf.kind, CreditorReferenceKind::Rf);
    assert_eq!(rf.canonical.unwrap().as_str(), "RF18539007547034");

    let bad_rf = CreditorReference::read("RF19539007547034".into());
    assert_eq!(bad_rf.kind, CreditorReferenceKind::Rf);
    assert!(!bad_rf.valid);

    for not_a_reference in ["0000", "123", "123456789012345678901"] {
        let r = CreditorReference::read(not_a_reference.into());
        assert_eq!(r.kind, CreditorReferenceKind::Domestic, "{not_a_reference}");
        assert!(!r.valid, "{not_a_reference}");
    }
}

/// Remove every `<Strd>…</Strd>` block: the same statement as a bank that
/// does not report the structured reference would export it.
fn without_strd(xml: &str) -> String {
    let mut out = String::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<Strd>") {
        out.push_str(&rest[..start]);
        let end = rest[start..].find("</Strd>").unwrap() + start + "</Strd>".len();
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn description(txn: &aggountant_import::BankTransaction) -> String {
    let assignment = Assignment {
        bank_ref: txn.bank_ref.clone(),
        bank_account: AccountId::new("1910").unwrap(),
        contra_postings: vec![ContraPosting {
            account: AccountId::new("2700").unwrap(),
            amount: txn.amount.clone(),
            vat_code: None,
        }],
        vat_treatment: None,
        document_date: None,
        description: None,
        not_booked_by: Vec::new(),
    };
    project_voucher(txn, &assignment, None)
        .unwrap()
        .description()
        .to_string()
}

#[test]
fn the_reference_does_not_change_entry_identity() {
    let with = parse_camt(REFERENCES).unwrap();
    let stripped = without_strd(REFERENCES);
    assert!(!stripped.contains("<CdtrRefInf>"));
    let without = parse_camt(&stripped).unwrap();
    assert_eq!(with.transactions.len(), without.transactions.len());
    for (a, b) in with.transactions.iter().zip(&without.transactions) {
        assert!(a.creditor_reference.is_some());
        assert!(b.creditor_reference.is_none());
        assert_eq!(a.natural_key, b.natural_key);
        assert_eq!(a.source_fingerprint(), b.source_fingerprint());
        assert_eq!(description(a), description(b));
    }
}

/// The event keys, source fingerprints and descriptions the build before the
/// creditor reference computed for the fixture (`import camt seal --dry-run`).
/// A change here makes the next overlapping import of every production
/// statement that carries the reference stop with `SOURCE_FACTS_CHANGED` or
/// `POSTING_PROJECTION_CHANGED`; it is an identity break, not a fixture update.
#[test]
fn the_reference_leaves_entry_identity_as_the_previous_build_computed_it() {
    let expected = [
        (
            "bank-import:holvi:FI2112345600000785:ESIM-2026-0603-001",
            "sha256:v1:10ec80054d97f0c34368b0bdc0918e3dc4d944d19d0bf1fe3217cda49c4e42ad",
            "Asiakas Esimerkki Ky",
        ),
        (
            "bank-import:holvi:FI2112345600000785:ESIM-2026-0610-002",
            "sha256:v1:39122822620c48bdb2e719b1c3a6387072f115cd36474b2a4aa30124346a1331",
            "Toimittaja Esimerkki Oy — Lasku 2026-31",
        ),
        (
            "bank-import:holvi:FI2112345600000785:ESIM-2026-0612-003",
            "sha256:v1:363dcf3d4f3383add48d981ebd2761b65c9dc5b2437aa479f4f7703889cd7d39",
            "Verohallinto",
        ),
        (
            "bank-import:holvi:FI2112345600000785:ESIM-2026-0618-004",
            "sha256:v1:9ab92541d20f508c731021a17c8a0df4ac7b782574513d0a57f47af17b41c27c",
            "Kustantamo Esimerkki",
        ),
        (
            "bank-import:holvi:FI2112345600000785:ESIM-2026-0625-005",
            "sha256:v1:b0ce10ac6d8c71eca15eb0123531efc267a923a9f4f6056a7376cef9c6e863c0",
            "Example Software AB — Subscription June",
        ),
    ];
    let stmt = parse_camt(REFERENCES).unwrap();
    assert_eq!(stmt.transactions.len(), expected.len());
    for (txn, (key, fingerprint, text)) in stmt.transactions.iter().zip(expected) {
        assert_eq!(txn.natural_key, key);
        assert_eq!(txn.source_fingerprint(), fingerprint, "{key}");
        assert_eq!(description(txn), text, "{key}");
    }
}

#[test]
fn a_remittance_without_a_reference_has_none() {
    // Strd with other content (here a referred document) but no CdtrRefInf.
    let xml = REFERENCES.replace(
        "<CdtrRefInf>\n                  <Ref>1234562</Ref>\n                </CdtrRefInf>",
        "<RfrdDocInf><Nb>2026-31</Nb></RfrdDocInf>",
    );
    assert_ne!(xml, REFERENCES);
    let stmt = parse_camt(&xml).unwrap();
    assert!(stmt.transactions[3].creditor_reference.is_none());
    let json = serde_json::to_value(&stmt.transactions[3]).unwrap();
    assert!(json.get("creditor_reference").is_none(), "{json}");
}
