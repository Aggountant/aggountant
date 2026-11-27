//! Golden tests for the passthrough fields of a bank entry: the bank
//! transaction code, reversal and return information, charges and the
//! instructed amount. They also pin that these fields leave every entry's
//! identity (event key, source fingerprint, default description) as the build
//! before them computed it, because production homes hold vouchers sealed from
//! statements that carry these elements.

use aggountant_import::{
    parse_camt, project_voucher, Assignment, ContraPosting, Direction, ReasonCode, ReversalKind,
};
use aggountant_model::{AccountId, Currency, Money, Side};
use serde_json::Value;

const RICH: &str = include_str!("fixtures/rich-camt053-sample.xml");
const RICH_GOLDEN: &str = include_str!("fixtures/rich-camt053-sample.parsed.json");
const REVERSAL: &str = include_str!("fixtures/reversal-camt052-sample.xml");
const REVERSAL_GOLDEN: &str = include_str!("fixtures/reversal-camt052-sample.parsed.json");
const HOLVI: &str = include_str!("fixtures/holvi-camt053-sample.xml");
/// The parse output of the build before the passthrough fields, verbatim.
const HOLVI_BEFORE: &str = include_str!("fixtures/holvi-camt053-sample.parsed.json");

fn golden(xml: &str, expected: &str) {
    let actual = serde_json::to_value(parse_camt(xml).unwrap()).unwrap();
    let expected: Value = serde_json::from_str(expected).unwrap();
    assert_eq!(
        actual,
        expected,
        "parse output differs from the golden file:\n{}",
        serde_json::to_string_pretty(&actual).unwrap()
    );
}

#[test]
fn rich_statement_matches_its_golden_output() {
    golden(RICH, RICH_GOLDEN);
}

#[test]
fn reversal_report_matches_its_golden_output() {
    golden(REVERSAL, REVERSAL_GOLDEN);
}

#[test]
fn a_file_without_the_elements_parses_exactly_as_before() {
    golden(HOLVI, HOLVI_BEFORE);
}

#[test]
fn a_card_entry_carries_its_transaction_code() {
    let stmt = parse_camt(RICH).unwrap();
    let code = stmt.transactions[0].transaction_code.as_ref().unwrap();
    let domain = code.domain.as_ref().unwrap();
    assert_eq!(
        (
            domain.domain.as_str(),
            domain.family.as_str(),
            domain.sub_family.as_str()
        ),
        ("PMNT", "CCRD", "POSD")
    );
    let proprietary = code.proprietary.as_ref().unwrap();
    assert_eq!(proprietary.code, "705");
    assert_eq!(proprietary.issuer.as_deref(), Some("FFFS"));
    assert!(stmt.transactions[0].reversal.is_none());
    assert!(stmt.transactions[0].charges.is_none());
    assert!(stmt.transactions[0].instructed_amount.is_none());
}

#[test]
fn charges_and_instructed_amount_are_read() {
    let stmt = parse_camt(RICH).unwrap();
    let charges = stmt.transactions[1].charges.as_ref().unwrap();
    assert_eq!(charges.total.as_ref().unwrap().amount_str(), "3.50");
    assert_eq!(charges.records.len(), 1);
    let record = &charges.records[0];
    assert_eq!(record.amount.amount_str(), "3.50");
    assert_eq!(record.direction, Some(Direction::Debit));
    assert_eq!(record.charge_type.as_deref(), Some("COMM"));
    assert_eq!(record.bearer.as_deref(), Some("DEBT"));
    assert_eq!(record.included_in_amount, None);

    let fx = stmt.transactions[2].instructed_amount.as_ref().unwrap();
    assert_eq!(fx.amount_str(), "100.00");
    assert_eq!(fx.currency().as_str(), "USD");
    // The entry amount stays what moved on the account.
    assert_eq!(stmt.transactions[2].amount.currency().as_str(), "EUR");
}

#[test]
fn a_returned_payment_is_a_return_with_its_reason() {
    let stmt = parse_camt(RICH).unwrap();
    let txn = &stmt.transactions[3];
    let reversal = txn.reversal.as_ref().unwrap();
    assert_eq!(reversal.kind, ReversalKind::Return);
    assert_eq!(reversal.reason, Some(ReasonCode::Code("AC04".into())));
    assert_eq!(reversal.additional_info, vec!["Tili suljettu".to_string()]);
    assert_eq!(reversal.end_to_end_id.as_deref(), Some("OSTO-2026-0012"));
    let original = reversal.original_transaction_code.as_ref().unwrap();
    assert_eq!(original.domain.as_ref().unwrap().sub_family, "ESCT");
}

/// The source fingerprints and descriptions the build before the passthrough
/// fields computed for the rich fixture (`import camt seal --dry-run`). A
/// change here makes the next overlapping import of every production statement
/// that carries these elements stop with `SOURCE_FACTS_CHANGED` or
/// `POSTING_PROJECTION_CHANGED`; it is an identity break, not a fixture update.
#[test]
fn passthrough_fields_leave_entry_identity_unchanged() {
    let expected = [
        (
            "bank-import:holvi:FI2112345600000785:ESIM-2026-0403-001",
            "sha256:v1:81e24239426032d9486a39083e53eae63a7a02abd7b36f38b1cad349997ae644",
            "Paperikauppa Esimerkki — Korttiosto 4321",
        ),
        (
            "bank-import:holvi:FI2112345600000785:ESIM-2026-0410-002",
            "sha256:v1:a91354d26e0454a5b6ca204aa85e92c130ba7ab816b180e121d9aeee9584d691",
            "Toimittaja AB — Lasku 2026-17",
        ),
        (
            "bank-import:holvi:FI2112345600000785:ESIM-2026-0415-003",
            "sha256:v1:045bd5e4068d3f2c2b68bb68bf8b8609d679ee7fb0d3503595c34db119a16efd",
            "Example Cloud Inc — Pilvipalvelu huhtikuu",
        ),
        (
            "bank-import:holvi:FI2112345600000785:ESIM-2026-0422-004",
            "sha256:v1:fdb26eaf817a86dce535f967fd251cbeaf1b0ad161aba7255e50cbdcfb00670a",
            "Alihankkija Oy — Lasku 2026-12",
        ),
    ];
    let stmt = parse_camt(RICH).unwrap();
    assert_eq!(stmt.transactions.len(), expected.len());
    for (txn, (key, fingerprint, description)) in stmt.transactions.iter().zip(expected) {
        assert_eq!(txn.natural_key, key);
        assert_eq!(txn.source_fingerprint(), fingerprint, "{key}");
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
        let draft = project_voucher(txn, &assignment, None).unwrap();
        assert_eq!(draft.description(), description, "{key}");
    }
}

#[test]
fn a_reversal_is_booked_in_its_own_direction() {
    let stmt = parse_camt(REVERSAL).unwrap();
    assert_eq!(stmt.transactions.len(), 2);
    let original = &stmt.transactions[0];
    let reversal = &stmt.transactions[1];
    assert!(original.reversal.is_none());
    let info = reversal.reversal.as_ref().unwrap();
    assert_eq!(info.kind, ReversalKind::Reversal);
    assert_eq!(info.reason, Some(ReasonCode::Code("AC01".into())));
    assert_eq!(
        info.additional_info,
        vec!["Virheellinen tilinumero".to_string()]
    );
    // The link to the original: the same end-to-end id, its own reference.
    assert_eq!(info.end_to_end_id.as_deref(), Some("OSTO-2026-0021"));
    assert_ne!(reversal.natural_key, original.natural_key);
    // A reversed debit is money in. The reversal keeps the original's roles
    // (the account owner as debtor), so the counterparty is the creditor.
    assert_eq!(reversal.direction, Direction::Credit);
    assert_eq!(reversal.counterparty.as_deref(), Some("Konepaja Esimerkki"));

    // Booked to the original's accounts, the projection undoes the original.
    let assignment = |bank_ref: &str| Assignment {
        bank_ref: bank_ref.to_string(),
        bank_account: AccountId::new("1910").unwrap(),
        contra_postings: vec![ContraPosting {
            account: AccountId::new("2700").unwrap(),
            amount: Money::parse("120.00", Currency::new("EUR").unwrap()).unwrap(),
            vat_code: None,
        }],
        vat_treatment: None,
        document_date: None,
        description: None,
        not_booked_by: Vec::new(),
    };
    let side_of = |draft: &aggountant_model::VoucherDraft, account: &str| {
        draft
            .postings()
            .iter()
            .find(|p| p.account.as_str() == account)
            .unwrap()
            .side
    };
    let booked = project_voucher(original, &assignment(&original.bank_ref), None).unwrap();
    let undone = project_voucher(reversal, &assignment(&reversal.bank_ref), None).unwrap();
    assert_eq!(side_of(&booked, "1910"), Side::Credit);
    assert_eq!(side_of(&undone, "1910"), Side::Debit);
    assert_eq!(side_of(&undone, "2700"), Side::Credit);
}

fn stmt_xml(body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.08">
  <BkToCstmrStmt><Stmt>
    <Acct><Id><IBAN>FI2112345600000785</IBAN></Id><Ccy>EUR</Ccy></Acct>
    {body}
  </Stmt></BkToCstmrStmt>
</Document>"#
    )
}

fn entry(indicator: &str, extra: &str, tx_extra: &str) -> String {
    format!(
        r#"<Ntry>
      <Amt Ccy="EUR">10.00</Amt><CdtDbtInd>{indicator}</CdtDbtInd>
      <Sts><Cd>BOOK</Cd></Sts>
      <BookgDt><Dt>2026-04-03</Dt></BookgDt>
      {extra}
      <NtryDtls><TxDtls>
        <Refs><AcctSvcrRef>REF-1</AcctSvcrRef><EndToEndId>E2E-1</EndToEndId></Refs>
        {tx_extra}
      </TxDtls></NtryDtls>
    </Ntry>"#
    )
}

#[test]
fn charge_records_of_later_versions_are_read() {
    let charges = r#"<Chrgs>
        <TtlChrgsAndTaxAmt Ccy="EUR">2.00</TtlChrgsAndTaxAmt>
        <Rcrd>
          <Amt Ccy="EUR">1.50</Amt><CdtDbtInd>DBIT</CdtDbtInd>
          <ChrgInclInd>true</ChrgInclInd>
          <Tp><Prtry><Id>PALVELUMAKSU</Id></Prtry></Tp><Br>SHAR</Br>
        </Rcrd>
        <Rcrd><Amt Ccy="EUR">0.50</Amt><ChrgInclInd>false</ChrgInclInd></Rcrd>
      </Chrgs>"#;
    // Charges in the transaction details are read when the entry has none.
    let stmt = parse_camt(&stmt_xml(&entry("DBIT", "", charges))).unwrap();
    let charges = stmt.transactions[0].charges.as_ref().unwrap();
    assert_eq!(charges.total.as_ref().unwrap().amount_str(), "2.00");
    assert_eq!(charges.records.len(), 2);
    assert_eq!(charges.records[0].included_in_amount, Some(true));
    assert_eq!(
        charges.records[0].charge_type.as_deref(),
        Some("PALVELUMAKSU")
    );
    assert_eq!(charges.records[0].bearer.as_deref(), Some("SHAR"));
    assert_eq!(charges.records[1].amount.amount_str(), "0.50");
    assert_eq!(charges.records[1].included_in_amount, Some(false));
    assert_eq!(charges.records[1].direction, None);
}

#[test]
fn a_proprietary_return_reason_is_kept_verbatim() {
    let rtr = "<RtrInf><Rsn><Prtry>PALAUTUS</Prtry></Rsn></RtrInf>";
    let stmt = parse_camt(&stmt_xml(&entry("CRDT", "", rtr))).unwrap();
    let info = stmt.transactions[0].reversal.as_ref().unwrap();
    assert_eq!(info.kind, ReversalKind::Return);
    assert_eq!(
        info.reason,
        Some(ReasonCode::Proprietary("PALAUTUS".into()))
    );
}

#[test]
fn a_reversal_without_return_information_still_says_so() {
    let stmt = parse_camt(&stmt_xml(&entry("CRDT", "<RvslInd>true</RvslInd>", ""))).unwrap();
    let info = stmt.transactions[0].reversal.as_ref().unwrap();
    assert_eq!(info.kind, ReversalKind::Reversal);
    assert_eq!(info.reason, None);
    assert_eq!(info.end_to_end_id.as_deref(), Some("E2E-1"));
}

#[test]
fn a_reversal_reusing_the_original_reference_in_one_file_is_refused() {
    let original = entry("DBIT", "", "");
    let reversal = entry("CRDT", "<RvslInd>true</RvslInd>", "");
    let err = parse_camt(&stmt_xml(&format!("{original}{reversal}"))).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT");
    assert!(err.to_string().contains("share the dedup key"), "{err}");
}

#[test]
fn an_invalid_reversal_indicator_is_still_refused() {
    let err = parse_camt(&stmt_xml(&entry("CRDT", "<RvslInd>yes</RvslInd>", ""))).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT");
}

#[test]
fn unreadable_passthrough_elements_are_left_out_not_refused() {
    // The build before these fields accepted such a file, so its retained copy
    // must still read back; only the unreadable part is missing.
    let extra = r#"<BkTxCd><Domn><Cd>PMNT</Cd><Fmly><Cd>CCRD</Cd></Fmly></Domn></BkTxCd>
      <Chrgs><Amt>1.00</Amt></Chrgs>
      <AmtDtls><InstdAmt><Amt Ccy="usd">x</Amt></InstdAmt></AmtDtls>"#;
    let stmt = parse_camt(&stmt_xml(&entry("DBIT", extra, ""))).unwrap();
    let txn = &stmt.transactions[0];
    assert!(txn.transaction_code.is_none());
    assert!(txn.charges.is_none());
    assert!(txn.instructed_amount.is_none());
    let json = serde_json::to_value(txn).unwrap();
    for field in [
        "transaction_code",
        "reversal",
        "charges",
        "instructed_amount",
    ] {
        assert!(json.get(field).is_none(), "{field} serialized: {json}");
    }
}

#[test]
fn a_reversal_whose_bank_swaps_the_roles_still_names_the_other_party() {
    let parties = "<RltdPties><Dbtr><Nm>Konepaja Esimerkki</Nm></Dbtr></RltdPties>";
    let stmt = parse_camt(&stmt_xml(&entry(
        "CRDT",
        "<RvslInd>true</RvslInd>",
        parties,
    )))
    .unwrap();
    assert_eq!(
        stmt.transactions[0].counterparty.as_deref(),
        Some("Konepaja Esimerkki")
    );
}
