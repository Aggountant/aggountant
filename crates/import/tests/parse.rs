//! Golden tests for the deterministic camt.053 parser and lawful projection.
//! The source-fingerprint regression intentionally composes the public Import and
//! Ledger APIs because the parallel CLI worktree boundary forbids changing the
//! production call site in this slice.

use aggountant_domain::{build_draft, PolicySet};
use aggountant_import::{
    assignment_for, check_assignments_unique, parse_camt053, project_draft, project_voucher,
    Assignment, ContraPosting, Direction, ImportError, ReverseChargeScope, VatTreatment,
};
use aggountant_ledger::{Ledger, SealRequest};
use aggountant_model::{AccountId, Currency, Money, Side, VatCode};

const SAMPLE: &str = include_str!("fixtures/holvi-camt053-sample.xml");
const SAMPLE_052: &str = include_str!("fixtures/holvi-camt052-sample.xml");

fn contra(account: &str, amount: &str, vat_code: Option<&str>) -> ContraPosting {
    contra_in(account, amount, "EUR", vat_code)
}

fn contra_in(account: &str, amount: &str, currency: &str, vat_code: Option<&str>) -> ContraPosting {
    ContraPosting {
        account: AccountId::new(account).unwrap(),
        amount: Money::parse(amount, Currency::new(currency).unwrap()).unwrap(),
        vat_code: vat_code.map(|code| VatCode::new(code).unwrap()),
    }
}

fn assignment(bank_ref: &str, contra_account: &str) -> Assignment {
    Assignment {
        bank_ref: bank_ref.to_string(),
        bank_account: AccountId::new("1910").unwrap(),
        contra_postings: vec![contra(contra_account, "124.00", None)],
        vat_treatment: None,
        document_date: None,
        description: None,
    }
}

#[test]
fn parses_only_booked_entries_with_canonical_fields() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    assert_eq!(stmt.account_iban, "FI2112345600000785");
    assert_eq!(stmt.currency.as_deref(), Some("EUR"));

    // The PDNG (pending) entry is skipped — only the two BOOK entries remain.
    assert_eq!(stmt.transactions.len(), 2);

    let out = &stmt.transactions[0];
    assert_eq!(out.direction, Direction::Debit);
    assert_eq!(out.amount.amount_str(), "124.00");
    assert_eq!(out.amount.currency().as_str(), "EUR");
    // KPL 2:4: the accounting date is the booking date, not the value date.
    assert_eq!(out.booking_date.to_string(), "2026-03-14");
    assert_eq!(
        out.value_date.map(|d| d.to_string()).as_deref(),
        Some("2026-03-15")
    );
    assert_eq!(out.accounting_date.to_string(), "2026-03-14");
    // Counterparty for money-out is the creditor.
    assert_eq!(out.counterparty.as_deref(), Some("Hetzner Online GmbH"));
    assert_eq!(
        out.description.as_deref(),
        Some("Palvelinvuokra maaliskuu 2026")
    );
    assert_eq!(out.bank_ref, "HLV-2026-0314-001");
    assert_eq!(
        out.natural_key,
        "bank-import:holvi:FI2112345600000785:HLV-2026-0314-001"
    );
    assert_eq!(
        out.source_fingerprint(),
        "sha256:v1:8aa071dbbfa000a85059b2be11bbb8eae0e40637daf43c5b660751926fe3c13a"
    );

    let inc = &stmt.transactions[1];
    assert_eq!(inc.direction, Direction::Credit);
    assert_eq!(inc.amount.amount_str(), "2480.00");
    // Counterparty for money-in is the debtor.
    assert_eq!(inc.counterparty.as_deref(), Some("Asiakas Oy"));
    // Multiple <Ustrd> are joined.
    assert_eq!(
        inc.description.as_deref(),
        Some("Lasku 2026-0042 Konsultointi")
    );
}

#[test]
fn changed_reexport_is_flagged_through_parse_project_and_seal() {
    let original = parse_camt053(SAMPLE).unwrap();
    // Value date is a retained source fact but is intentionally not projected to
    // the voucher (BookgDt is the accounting date). This isolates the source
    // fingerprint check: both projected vouchers are byte-identical.
    let corrected_xml = SAMPLE.replacen("<Dt>2026-03-15</Dt>", "<Dt>2026-03-16</Dt>", 1);
    let corrected = parse_camt053(&corrected_xml).unwrap();
    let original_txn = &original.transactions[0];
    let corrected_txn = &corrected.transactions[0];
    assert_eq!(original_txn.natural_key, corrected_txn.natural_key);
    assert_eq!(original_txn.booking_date, corrected_txn.booking_date);
    assert_ne!(original_txn.value_date, corrected_txn.value_date);
    assert_eq!(original_txn.accounting_date, corrected_txn.accounting_date);
    assert_eq!(original_txn.direction, corrected_txn.direction);
    assert_eq!(original_txn.amount, corrected_txn.amount);
    assert_eq!(original_txn.counterparty, corrected_txn.counterparty);
    assert_eq!(original_txn.description, corrected_txn.description);
    assert_ne!(
        original_txn.source_fingerprint(),
        corrected_txn.source_fingerprint()
    );

    let assignment = assignment(&original_txn.bank_ref, "7690");
    let original_draft = project_voucher(original_txn, &assignment, None).unwrap();
    let corrected_draft = project_voucher(corrected_txn, &assignment, None).unwrap();
    assert_eq!(
        aggountant_wire::canonical_bytes(&original_draft).unwrap(),
        aggountant_wire::canonical_bytes(&corrected_draft).unwrap(),
        "projected draft bytes must be identical to isolate the source check"
    );

    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let request = SealRequest {
        natural_key: original_txn.natural_key.clone(),
        actor: "import-test".into(),
        policy_set: PolicySet::default(),
    };
    ledger
        .seal_with_source_fingerprint(
            &original_draft,
            &request,
            &original_txn.source_fingerprint(),
        )
        .unwrap();
    let error = ledger
        .seal_with_source_fingerprint(
            &corrected_draft,
            &request,
            &corrected_txn.source_fingerprint(),
        )
        .unwrap_err();
    assert_eq!(error.error_code(), "IDEMPOTENCY_KEY_REUSE");
    assert!(error
        .to_string()
        .contains("projected voucher content is unchanged"));
    assert_eq!(ledger.verify().unwrap().voucher_count, 1);
}

#[test]
fn camt052_and_camt053_yield_equivalent_transactions() {
    // Holvi's self-service export is camt.052 (BkToCstmrAcctRpt -> Rpt), not
    // camt.053. Only the container differs; the per-<Ntry> payload is identical,
    // so the two fixtures (same entries under different containers) must reduce
    // to byte-for-byte equal canonical statements.
    let s053 = parse_camt053(SAMPLE).unwrap();
    let s052 = parse_camt053(SAMPLE_052).unwrap();

    assert_eq!(s052.account_iban, s053.account_iban);
    assert_eq!(s052.currency, s053.currency);
    // The PDNG entry is skipped in both — only the two BOOK entries remain.
    assert_eq!(s052.transactions.len(), 2);
    assert_eq!(s052.transactions, s053.transactions);
}

#[test]
fn camt052_is_accepted_not_rejected_as_wrong_format() {
    // Regression: before this fix the .052 container was rejected with
    // "no <BkToCstmrStmt>". It must now parse cleanly.
    let stmt = parse_camt053(SAMPLE_052).unwrap();
    assert_eq!(stmt.account_iban, "FI2112345600000785");
    assert_eq!(stmt.transactions[0].direction, Direction::Debit);
    assert_eq!(stmt.transactions[0].amount.amount_str(), "124.00");
    assert_eq!(
        stmt.transactions[0].counterparty.as_deref(),
        Some("Hetzner Online GmbH")
    );
    assert_eq!(
        stmt.transactions[0].natural_key,
        "bank-import:holvi:FI2112345600000785:HLV-2026-0314-001"
    );
}

/// A minimal camt.052 report document (`BkToCstmrAcctRpt → Rpt`), parameterized.
fn rpt_xml(body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.052.001.02">
  <BkToCstmrAcctRpt><Rpt>
    <Acct><Id><IBAN>FI2112345600000785</IBAN></Id><Ccy>EUR</Ccy></Acct>
    {body}
  </Rpt></BkToCstmrAcctRpt>
</Document>"#
    )
}

#[test]
fn camt052_reversal_entry_is_refused() {
    // camt.052 (interim) can carry reversals; a reversal is not an independent
    // booking, so it is refused rather than mis-posted / silently deduped.
    let body = r#"<Ntry>
      <Amt Ccy="EUR">1.00</Amt><CdtDbtInd>DBIT</CdtDbtInd>
      <RvslInd>true</RvslInd><Sts>BOOK</Sts>
      <BookgDt><Dt>2026-03-14</Dt></BookgDt>
      <NtryDtls><TxDtls><Refs><AcctSvcrRef>REF-RVSL</AcctSvcrRef></Refs></TxDtls></NtryDtls>
    </Ntry>"#;
    let err = parse_camt053(&rpt_xml(body)).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT053");
    assert!(err.to_string().contains("RvslInd"), "{err}");

    // RvslInd=false is the normal case and parses fine.
    let ok_body = r#"<Ntry>
      <Amt Ccy="EUR">1.00</Amt><CdtDbtInd>DBIT</CdtDbtInd>
      <RvslInd>false</RvslInd><Sts>BOOK</Sts>
      <BookgDt><Dt>2026-03-14</Dt></BookgDt>
      <NtryDtls><TxDtls><Refs><AcctSvcrRef>REF-OK</AcctSvcrRef></Refs></TxDtls></NtryDtls>
    </Ntry>"#;
    assert_eq!(
        parse_camt053(&rpt_xml(ok_body)).unwrap().transactions.len(),
        1
    );
}

#[test]
fn document_with_both_containers_is_refused() {
    // A document carrying both a .053 and a .052 container must be refused rather
    // than silently processing only one.
    let xml = r#"<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.02">
  <BkToCstmrStmt><Stmt><Acct><Id><IBAN>FI2112345600000785</IBAN></Id></Acct></Stmt></BkToCstmrStmt>
  <BkToCstmrAcctRpt><Rpt><Acct><Id><IBAN>FI2112345600000785</IBAN></Id></Acct></Rpt></BkToCstmrAcctRpt>
</Document>"#;
    let err = parse_camt053(xml).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT053");
    assert!(err.to_string().contains("both"), "{err}");
}

#[test]
fn report_missing_iban_is_refused_not_scoped_under_the_first() {
    // A second <Rpt> without an IBAN must not inherit the first report's account.
    let xml = format!(
        r#"<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.052.001.02">
  <BkToCstmrAcctRpt>
    <Rpt><Acct><Id><IBAN>FI2112345600000785</IBAN></Id><Ccy>EUR</Ccy></Acct>
      {}
    </Rpt>
    <Rpt><Acct><Ccy>EUR</Ccy></Acct>
      {}
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#,
        book_entry("EUR", "1.00", "REF-A"),
        book_entry("EUR", "2.00", "REF-B"),
    );
    let err = parse_camt053(&xml).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT053");
    assert!(err.to_string().contains("no <Acct/Id/IBAN>"), "{err}");
}

#[test]
fn multi_rpt_same_account_concatenates_entries() {
    // Several <Rpt> blocks for the same account concatenate their entries.
    let xml = format!(
        r#"<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.052.001.02">
  <BkToCstmrAcctRpt>
    <Rpt><Acct><Id><IBAN>FI2112345600000785</IBAN></Id><Ccy>EUR</Ccy></Acct>
      {}
    </Rpt>
    <Rpt><Acct><Id><IBAN>FI2112345600000785</IBAN></Id><Ccy>EUR</Ccy></Acct>
      {}
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#,
        book_entry("EUR", "1.00", "REF-A"),
        book_entry("EUR", "2.00", "REF-B"),
    );
    let stmt = parse_camt053(&xml).unwrap();
    assert_eq!(stmt.transactions.len(), 2);
}

#[test]
fn balance_only_report_with_no_entries_parses_to_empty() {
    // An intraday .052 can carry balances and zero <Ntry> — that is a valid,
    // empty statement, not an error.
    let body = r#"<Bal>
      <Tp><CdOrPrtry><Cd>CLBD</Cd></CdOrPrtry></Tp>
      <Amt Ccy="EUR">100.00</Amt><CdtDbtInd>CRDT</CdtDbtInd><Dt><Dt>2026-03-31</Dt></Dt>
    </Bal>"#;
    let stmt = parse_camt053(&rpt_xml(body)).unwrap();
    assert_eq!(stmt.account_iban, "FI2112345600000785");
    assert!(stmt.transactions.is_empty());
}

#[test]
fn entry_with_multiple_ntrydtls_blocks_is_refused() {
    // Two <NtryDtls> blocks each with one <TxDtls> is still a batched entry — the
    // aggregate amount would misattribute the second sub-transaction.
    let body = r#"<Ntry>
      <Amt Ccy="EUR">100.00</Amt><CdtDbtInd>DBIT</CdtDbtInd><Sts>BOOK</Sts>
      <BookgDt><Dt>2026-03-14</Dt></BookgDt>
      <NtryDtls><TxDtls><Refs><AcctSvcrRef>REF-1</AcctSvcrRef></Refs></TxDtls></NtryDtls>
      <NtryDtls><TxDtls><Refs><AcctSvcrRef>REF-2</AcctSvcrRef></Refs></TxDtls></NtryDtls>
    </Ntry>"#;
    let err = parse_camt053(&rpt_xml(body)).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT053");
    assert!(err.to_string().contains("batched"), "{err}");
}

#[test]
fn parse_is_deterministic() {
    let a = parse_camt053(SAMPLE).unwrap();
    let b = parse_camt053(SAMPLE).unwrap();
    assert_eq!(a, b);
}

#[test]
fn projection_maps_direction_to_sides_and_balances() {
    let stmt = parse_camt053(SAMPLE).unwrap();

    // Money out (Debit): bank credited, contra (expense) debited.
    let out = &stmt.transactions[0];
    let draft =
        build_draft(project_draft(out, &assignment(&out.bank_ref, "7690"), None).unwrap()).unwrap();
    let postings = draft.postings();
    assert_eq!(postings.len(), 2);
    let bank = postings
        .iter()
        .find(|p| p.account.as_str() == "1910")
        .unwrap();
    let contra_leg = postings
        .iter()
        .find(|p| p.account.as_str() == "7690")
        .unwrap();
    assert_eq!(bank.side, Side::Credit);
    assert_eq!(contra_leg.side, Side::Debit);
    assert_eq!(bank.amount.amount_str(), "124.00");

    // Money in (Credit): bank debited, contra (income) credited.
    let inc = &stmt.transactions[1];
    let draft = build_draft(
        project_draft(
            inc,
            &Assignment {
                bank_ref: inc.bank_ref.clone(),
                bank_account: AccountId::new("1910").unwrap(),
                contra_postings: vec![contra("3000", "2480.00", None)],
                vat_treatment: None,
                document_date: None,
                description: None,
            },
            None,
        )
        .unwrap(),
    )
    .unwrap();
    let bank = draft
        .postings()
        .iter()
        .find(|p| p.account.as_str() == "1910")
        .unwrap();
    let contra = draft
        .postings()
        .iter()
        .find(|p| p.account.as_str() == "3000")
        .unwrap();
    assert_eq!(bank.side, Side::Debit);
    assert_eq!(contra.side, Side::Credit);
}

#[test]
fn projection_preserves_a_canonically_zero_rate_vat_code() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let mut a = assignment(&out.bank_ref, "7690");
    a.contra_postings[0].vat_code = Some(VatCode::new("purchase-zero-rated").unwrap());
    let draft = build_draft(project_draft(out, &a, None).unwrap()).unwrap();
    let bank = draft
        .postings()
        .iter()
        .find(|p| p.account.as_str() == "1910")
        .unwrap();
    let contra = draft
        .postings()
        .iter()
        .find(|p| p.account.as_str() == "7690")
        .unwrap();
    assert!(bank.vat_code.is_none());
    assert_eq!(
        contra.vat_code.as_ref().map(VatCode::as_str),
        Some("purchase-zero-rated")
    );
    assert_eq!(draft.document_date(), out.accounting_date);
    assert_eq!(draft.posting_date(), out.accounting_date);
}

#[test]
fn projection_refuses_every_taxable_leg_in_a_split_gross_allocation() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let assignment = Assignment {
        bank_ref: out.bank_ref.clone(),
        bank_account: AccountId::new("1910").unwrap(),
        contra_postings: vec![
            contra("4000", "100.00", Some("purchase-zero-rated")),
            contra("4010", "24.00", Some("purchase-domestic-reduced-1")),
        ],
        vat_treatment: None,
        document_date: Some(aggountant_model::Date::parse("2026-03-01").unwrap()),
        description: None,
    };

    let error = project_voucher(out, &assignment, None).unwrap_err();
    assert_eq!(error.error_code(), "UNSAFE_LEGACY_VAT_CODE");
    assert!(error.to_string().contains("contra_postings[1].vat_code"));
    assert!(error.to_string().contains("gross allocations"));
}

#[test]
fn projection_refuses_taxable_sales_tags_on_money_in() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let incoming = &stmt.transactions[1];
    let mut a = assignment(&incoming.bank_ref, "3000");
    a.contra_postings[0].amount = incoming.amount.clone();
    a.contra_postings[0].vat_code = Some(VatCode::new("sales-domestic-standard").unwrap());
    a.document_date = Some(aggountant_model::Date::parse("2026-03-01").unwrap());

    let error = project_voucher(incoming, &a, None).unwrap_err();
    assert_eq!(error.error_code(), "UNSAFE_LEGACY_VAT_CODE");
    assert!(error
        .to_string()
        .contains("sales VAT expansion is unsupported"));
}

#[test]
fn projection_does_not_treat_an_unknown_legacy_code_as_zero() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let mut a = assignment(&out.bank_ref, "7690");
    a.contra_postings[0].vat_code = Some(VatCode::new("purchase-unknown-class").unwrap());

    let error = project_voucher(out, &a, None).unwrap_err();
    assert_eq!(error.error_code(), "VAT_CODE");
    assert!(error.to_string().contains("unknown-class"));
}

#[test]
fn projection_splits_loan_principal_and_interest_to_separate_accounts() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let assignment = Assignment {
        bank_ref: out.bank_ref.clone(),
        bank_account: AccountId::new("1910").unwrap(),
        contra_postings: vec![
            contra("2350", "100.00", None),
            contra("8310", "24.00", None),
        ],
        vat_treatment: None,
        document_date: None,
        description: Some("Loan instalment".to_string()),
    };

    let draft = build_draft(project_draft(out, &assignment, None).unwrap()).unwrap();
    assert_eq!(draft.postings().len(), 3);
    assert!(draft.postings().iter().any(
        |posting| posting.account.as_str() == "2350" && posting.amount.amount_str() == "100.00"
    ));
    assert!(draft.postings().iter().any(
        |posting| posting.account.as_str() == "8310" && posting.amount.amount_str() == "24.00"
    ));
}

#[test]
fn projection_splits_a_money_in_entry_to_multiple_credit_legs() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let incoming = &stmt.transactions[1];
    let assignment = Assignment {
        bank_ref: incoming.bank_ref.clone(),
        bank_account: AccountId::new("1910").unwrap(),
        contra_postings: vec![
            contra("3000", "2400.00", None),
            contra("2930", "80.00", None),
        ],
        vat_treatment: None,
        document_date: None,
        description: None,
    };

    let draft = build_draft(project_draft(incoming, &assignment, None).unwrap()).unwrap();
    assert!(draft
        .postings()
        .iter()
        .filter(|posting| posting.account.as_str() != "1910")
        .all(|posting| posting.side == Side::Credit));
}

#[test]
fn projection_refuses_a_split_that_does_not_equal_the_bank_amount() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let assignment = Assignment {
        bank_ref: out.bank_ref.clone(),
        bank_account: AccountId::new("1910").unwrap(),
        contra_postings: vec![
            contra("2350", "100.00", None),
            contra("8310", "23.99", None),
        ],
        vat_treatment: None,
        document_date: None,
        description: None,
    };

    let error = project_draft(out, &assignment, None).unwrap_err();
    assert_eq!(error.error_code(), "INVALID_ASSIGNMENT");
    assert!(error
        .to_string()
        .contains("123.99 EUR does not equal bank transaction amount 124.00 EUR"));
}

#[test]
fn projection_refuses_an_empty_or_non_positive_split() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let empty = Assignment {
        bank_ref: out.bank_ref.clone(),
        bank_account: AccountId::new("1910").unwrap(),
        contra_postings: vec![],
        vat_treatment: None,
        document_date: None,
        description: None,
    };
    let error = project_draft(out, &empty, None).unwrap_err();
    assert_eq!(error.error_code(), "INVALID_ASSIGNMENT");
    assert!(error.to_string().contains("has no contra_postings"));

    let zero = Assignment {
        bank_ref: out.bank_ref.clone(),
        bank_account: AccountId::new("1910").unwrap(),
        contra_postings: vec![contra("2350", "124.00", None), contra("8310", "0.00", None)],
        vat_treatment: None,
        document_date: None,
        description: None,
    };
    let error = project_draft(out, &zero, None).unwrap_err();
    assert_eq!(error.error_code(), "INVALID_ASSIGNMENT");
    assert!(error.to_string().contains("amount must be > 0"));
}

#[test]
fn projection_refuses_a_split_with_a_different_currency() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let assignment = Assignment {
        bank_ref: out.bank_ref.clone(),
        bank_account: AccountId::new("1910").unwrap(),
        contra_postings: vec![contra_in("2350", "124.00", "USD", None)],
        vat_treatment: None,
        document_date: None,
        description: None,
    };

    let error = project_draft(out, &assignment, None).unwrap_err();
    assert_eq!(error.error_code(), "INVALID_ASSIGNMENT");
    assert!(error
        .to_string()
        .contains("currency USD does not match bank transaction currency EUR"));
}

#[test]
fn missing_assignment_is_refused_not_guessed() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let err = assignment_for(out, &[]).unwrap_err();
    assert!(matches!(err, ImportError::MissingAssignment(_)));
    assert_eq!(err.error_code(), "MISSING_ASSIGNMENT");
}

#[test]
fn description_override_wins_else_falls_back() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let mut a = assignment(&out.bank_ref, "7690");
    a.description = Some("Custom memo".to_string());
    let draft = build_draft(project_draft(out, &a, None).unwrap()).unwrap();
    assert_eq!(draft.description(), "Custom memo");

    // Without an override, the description is built from the transaction.
    let draft =
        build_draft(project_draft(out, &assignment(&out.bank_ref, "7690"), None).unwrap()).unwrap();
    assert_eq!(
        draft.description(),
        "Hetzner Online GmbH — Palvelinvuokra maaliskuu 2026"
    );
}

#[test]
fn malformed_xml_is_an_invalid_camt053_error() {
    let err = parse_camt053("<not-a-statement/>").unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT053");

    let err = parse_camt053("<Document>{ broken").unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT053");
}

/// A minimal single-entry statement, parameterized, for the fix-coverage tests.
fn stmt_xml(iban: &str, body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.02">
  <BkToCstmrStmt><Stmt>
    <Acct><Id><IBAN>{iban}</IBAN></Id><Ccy>EUR</Ccy></Acct>
    {body}
  </Stmt></BkToCstmrStmt>
</Document>"#
    )
}

fn book_entry(amt_ccy: &str, amt: &str, reference: &str) -> String {
    format!(
        r#"<Ntry>
      <Amt Ccy="{amt_ccy}">{amt}</Amt>
      <CdtDbtInd>DBIT</CdtDbtInd>
      <Sts>BOOK</Sts>
      <BookgDt><Dt>2026-03-14</Dt></BookgDt>
      <NtryDtls><TxDtls><Refs><AcctSvcrRef>{reference}</AcctSvcrRef></Refs></TxDtls></NtryDtls>
    </Ntry>"#
    )
}

#[test]
fn multi_account_document_is_refused() {
    // Two <Stmt> for different IBANs must not collapse under the first IBAN.
    let xml = format!(
        r#"<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.02">
  <BkToCstmrStmt>
    <Stmt><Acct><Id><IBAN>FI2112345600000785</IBAN></Id><Ccy>EUR</Ccy></Acct>
      {}
    </Stmt>
    <Stmt><Acct><Id><IBAN>FI5542345670000081</IBAN></Id><Ccy>EUR</Ccy></Acct>
      {}
    </Stmt>
  </BkToCstmrStmt>
</Document>"#,
        book_entry("EUR", "1.00", "REF-A"),
        book_entry("EUR", "2.00", "REF-B"),
    );
    let err = parse_camt053(&xml).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT053");
    assert!(err.to_string().contains("mixes accounts"), "{err}");
}

#[test]
fn batched_entry_with_multiple_txdtls_is_refused() {
    let body = r#"<Ntry>
      <Amt Ccy="EUR">100.00</Amt><CdtDbtInd>DBIT</CdtDbtInd><Sts>BOOK</Sts>
      <BookgDt><Dt>2026-03-14</Dt></BookgDt>
      <NtryDtls>
        <TxDtls><Refs><AcctSvcrRef>REF-1</AcctSvcrRef></Refs></TxDtls>
        <TxDtls><Refs><AcctSvcrRef>REF-2</AcctSvcrRef></Refs></TxDtls>
      </NtryDtls>
    </Ntry>"#;
    let err = parse_camt053(&stmt_xml("FI2112345600000785", body)).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT053");
    assert!(err.to_string().contains("batched"), "{err}");
}

#[test]
fn placeholder_acctsvcrref_falls_through_to_endtoendid() {
    // Entry-level AcctSvcrRef is the "NOTPROVIDED" sentinel; a usable EndToEndId
    // must still be selected rather than shadowed then refused.
    let body = r#"<Ntry>
      <Amt Ccy="EUR">1.00</Amt><CdtDbtInd>DBIT</CdtDbtInd><Sts>BOOK</Sts>
      <BookgDt><Dt>2026-03-14</Dt></BookgDt>
      <AcctSvcrRef>NOTPROVIDED</AcctSvcrRef>
      <NtryDtls><TxDtls><Refs><EndToEndId>E2E-USABLE</EndToEndId></Refs></TxDtls></NtryDtls>
    </Ntry>"#;
    let stmt = parse_camt053(&stmt_xml("FI2112345600000785", body)).unwrap();
    assert_eq!(stmt.transactions[0].bank_ref, "E2E-USABLE");
}

#[test]
fn duplicate_dedup_key_within_a_statement_is_refused() {
    // Two entries with the same reference would collapse to one on seal.
    let body = format!(
        "{}\n{}",
        book_entry("EUR", "1.00", "DUP-REF"),
        book_entry("EUR", "2.00", "DUP-REF")
    );
    let err = parse_camt053(&stmt_xml("FI2112345600000785", &body)).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT053");
    assert!(err.to_string().contains("dedup key"), "{err}");
}

#[test]
fn foreign_currency_entry_is_refused() {
    let body = book_entry("USD", "1.00", "REF-USD");
    let err = parse_camt053(&stmt_xml("FI2112345600000785", &body)).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT053");
    assert!(err.to_string().contains("currency"), "{err}");
}

#[test]
fn duplicate_assignments_are_refused() {
    let dup = vec![
        assignment("HLV-2026-0314-001", "7690"),
        assignment("HLV-2026-0314-001", "7700"),
    ];
    let err = check_assignments_unique(&dup).unwrap_err();
    // A duplicate is an *invalid* assignment, not a *missing* one.
    assert_eq!(err.error_code(), "INVALID_ASSIGNMENT");
    assert!(check_assignments_unique(&[assignment("A", "7690")]).is_ok());
}

#[test]
fn datetime_booking_date_uses_the_calendar_part() {
    let body = r#"<Ntry>
      <Amt Ccy="EUR">1.00</Amt><CdtDbtInd>DBIT</CdtDbtInd><Sts>BOOK</Sts>
      <BookgDt><DtTm>2026-03-14T09:30:00+02:00</DtTm></BookgDt>
      <NtryDtls><TxDtls><Refs><AcctSvcrRef>REF-DTTM</AcctSvcrRef></Refs></TxDtls></NtryDtls>
    </Ntry>"#;
    let stmt = parse_camt053(&stmt_xml("FI2112345600000785", body)).unwrap();
    assert_eq!(
        stmt.transactions[0].accounting_date.to_string(),
        "2026-03-14"
    );
}

#[test]
fn entry_without_a_reference_is_refused() {
    // An Ntry with no AcctSvcrRef / EndToEndId cannot yield a stable dedup key.
    let xml = r#"<?xml version="1.0"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.02">
  <BkToCstmrStmt><Stmt>
    <Acct><Id><IBAN>FI2112345600000785</IBAN></Id></Acct>
    <Ntry>
      <Amt Ccy="EUR">1.00</Amt>
      <CdtDbtInd>DBIT</CdtDbtInd>
      <Sts>BOOK</Sts>
      <BookgDt><Dt>2026-03-14</Dt></BookgDt>
    </Ntry>
  </Stmt></BkToCstmrStmt>
</Document>"#;
    let err = parse_camt053(xml).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_CAMT053");
}

// --- ADR-7 (VAT-A) project_voucher expansion --------------------------------

/// Build an assignment whose contra account is ADR-7's net account, carrying a
/// VAT treatment that drives the Domain expansion.
fn treated(bank_ref: &str, net: &str, treatment: VatTreatment) -> Assignment {
    treated_with_amount(bank_ref, net, "124.00", treatment)
}

fn treated_with_amount(
    bank_ref: &str,
    net: &str,
    amount: &str,
    treatment: VatTreatment,
) -> Assignment {
    Assignment {
        bank_ref: bank_ref.to_string(),
        bank_account: AccountId::new("1910").unwrap(),
        contra_postings: vec![contra(net, amount, None)],
        vat_treatment: Some(treatment),
        document_date: Some(aggountant_model::Date::parse("2026-03-14").unwrap()),
        description: None,
    }
}

#[test]
fn domestic_treatment_uses_explicit_document_date_and_preserves_bank_posting_date() {
    let xml = SAMPLE.replacen(">124.00</Amt>", ">129.00</Amt>", 1);
    let xml = xml.replacen(">2026-03-14</Dt>", ">2024-09-02</Dt>", 1);
    let stmt = parse_camt053(&xml).unwrap();
    let out = &stmt.transactions[0];
    let mut a = treated_with_amount(
        &out.bank_ref,
        "7690",
        "129.00",
        VatTreatment::DomesticInput {
            rate_class: "domestic-standard".into(),
        },
    );
    a.document_date = Some(aggountant_model::Date::parse("2024-08-31").unwrap());

    let draft = project_voucher(out, &a, None).unwrap();
    assert_eq!(draft.document_date().to_string(), "2024-08-31");
    assert_eq!(draft.posting_date().to_string(), "2024-09-02");
    let amount = |account: &str| {
        draft
            .postings()
            .iter()
            .find(|posting| posting.account.as_str() == account)
            .unwrap()
            .amount
            .amount_str()
    };
    assert_eq!(amount("7690"), "104.03");
    assert_eq!(amount("1763"), "24.97");
    assert_eq!(amount("1910"), "129.00");
}

#[test]
fn domestic_treatment_at_current_rate_splits_129_euros_exactly() {
    let xml = SAMPLE.replacen(">124.00</Amt>", ">129.00</Amt>", 1);
    let stmt = parse_camt053(&xml).unwrap();
    let out = &stmt.transactions[0];
    let a = treated_with_amount(
        &out.bank_ref,
        "7690",
        "129.00",
        VatTreatment::DomesticInput {
            rate_class: "domestic-standard".into(),
        },
    );
    let draft = project_voucher(out, &a, None).unwrap();
    let amount = |account: &str| {
        draft
            .postings()
            .iter()
            .find(|posting| posting.account.as_str() == account)
            .unwrap()
            .amount
            .amount_str()
    };
    assert_eq!(amount("7690"), "102.79");
    assert_eq!(amount("1763"), "26.21");
    assert_eq!(amount("1910"), "129.00");
}

#[test]
fn project_voucher_refuses_treatment_without_source_document_date() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let mut a = treated(
        &out.bank_ref,
        "7660",
        VatTreatment::DomesticInput {
            rate_class: "domestic-standard".into(),
        },
    );
    a.document_date = None;

    let error = project_voucher(out, &a, None).unwrap_err();
    assert_eq!(error.error_code(), "MISSING_DOCUMENT_DATE");
    assert!(error.to_string().contains("field document_date"));
    assert!(error.to_string().contains("2026-03-14"));
}

#[test]
fn project_voucher_expands_a_reverse_charge_treatment_in_the_domain() {
    // The money-out entry (124,00 on 2026-03-14, 25,5 %) with an EU-services reverse
    // charge expands to the four sealed legs: net 4020 D, bank 1910 K, deductible
    // input 1798 D 31,62, self-assessed output 2992 K 31,62.
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let a = treated(
        &out.bank_ref,
        "4020",
        VatTreatment::ReverseCharge {
            rate_class: "domestic-standard".into(),
            scope: ReverseChargeScope::EuServices,
        },
    );
    let draft = project_voucher(out, &a, None).unwrap();
    assert_eq!(draft.postings().len(), 4);
    let find = |acct: &str, side: Side| {
        draft
            .postings()
            .iter()
            .find(|p| p.account.as_str() == acct && p.side == side)
            .unwrap_or_else(|| panic!("expected {side:?} leg on {acct}"))
            .amount
            .amount_str()
    };
    assert_eq!(find("4020", Side::Debit), "124.00");
    assert_eq!(find("1910", Side::Credit), "124.00");
    assert_eq!(find("1798", Side::Debit), "31.62");
    assert_eq!(find("2992", Side::Credit), "31.62");
    let provenance = draft
        .vat_expansion()
        .expect("import must return Domain provenance");
    assert_eq!(provenance.source_amount().amount_str(), "124.00");
    assert_eq!(provenance.resolved_percent(), Some("25.5"));
    assert_eq!(provenance.net_account().as_str(), "4020");
    assert_eq!(provenance.contra_account().as_str(), "1910");
    assert_eq!(provenance.generated_vat_legs().len(), 2);
    assert_eq!(
        provenance.vat_rates_hash().to_hex(),
        "e736ab82100707db6ab806df5b3acffa7285676b0b8401cc0cc45cc46afff4e8"
    );
}

#[test]
fn project_voucher_refuses_a_treatment_on_a_money_in_entry() {
    // Deductible input / reverse-charge VAT is purchase-side; a Credit (money-in)
    // entry with a treatment is refused rather than mis-booked.
    let stmt = parse_camt053(SAMPLE).unwrap();
    let inc = &stmt.transactions[1];
    assert_eq!(inc.direction, Direction::Credit);
    let a = treated_with_amount(
        &inc.bank_ref,
        "4020",
        "2480.00",
        VatTreatment::ReverseCharge {
            rate_class: "domestic-standard".into(),
            scope: ReverseChargeScope::EuServices,
        },
    );
    let err = project_voucher(inc, &a, None).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_ASSIGNMENT");
    assert!(err.to_string().contains("money-in"));
}

#[test]
fn project_draft_refuses_vat_treatment() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let a = treated(
        &out.bank_ref,
        "7660",
        VatTreatment::DomesticInput {
            rate_class: "domestic-standard".into(),
        },
    );
    let error = project_draft(out, &a, None).unwrap_err();
    assert_eq!(error.error_code(), "INVALID_ASSIGNMENT");
    assert!(error.to_string().contains("use project_voucher"));
}

#[test]
fn project_voucher_refuses_a_treatment_with_multiple_contra_postings() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let mut a = treated(
        &out.bank_ref,
        "7660",
        VatTreatment::DomesticInput {
            rate_class: "domestic-standard".into(),
        },
    );
    a.contra_postings = vec![
        contra("7660", "100.00", None),
        contra("8310", "24.00", None),
    ];
    let error = project_voucher(out, &a, None).unwrap_err();
    assert_eq!(error.error_code(), "INVALID_ASSIGNMENT");
    assert!(error
        .to_string()
        .contains("VAT expansion supports exactly one net account"));
}

#[test]
fn project_voucher_refuses_both_vat_code_and_vat_treatment() {
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let mut a = treated(
        &out.bank_ref,
        "7660",
        VatTreatment::DomesticInput {
            rate_class: "domestic-standard".into(),
        },
    );
    a.contra_postings[0].vat_code = Some(VatCode::new("purchase-domestic-standard").unwrap());
    let err = project_voucher(out, &a, None).unwrap_err();
    assert_eq!(err.error_code(), "INVALID_ASSIGNMENT");
}

#[test]
fn project_voucher_without_a_treatment_is_the_simple_projection() {
    // No treatment → the unchanged 2-leg bank ↔ contra projection.
    let stmt = parse_camt053(SAMPLE).unwrap();
    let out = &stmt.transactions[0];
    let draft = project_voucher(out, &assignment(&out.bank_ref, "7690"), None).unwrap();
    assert_eq!(draft.postings().len(), 2);
}
