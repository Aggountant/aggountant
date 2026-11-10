use aggountant_domain::PolicySet;
use aggountant_ledger::{
    render_monthly_report_html, FiscalYearState, Ledger, MonthlyReportOptions, SealRequest,
};
use aggountant_model::{AccountId, Currency, Date, Money, PeriodId, Posting, Side, VoucherDraft};

fn eur() -> Currency {
    Currency::new("EUR").unwrap()
}

fn seal(ledger: &Ledger, date: &str, debit: &str, credit: &str, amount: &str, key: &str) {
    let date = Date::parse(date).unwrap();
    let draft = VoucherDraft::new(
        date,
        date,
        key.into(),
        vec![
            Posting {
                account: AccountId::new(debit).unwrap(),
                amount: Money::parse(amount, eur()).unwrap(),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new(credit).unwrap(),
                amount: Money::parse(amount, eur()).unwrap(),
                side: Side::Credit,
                vat_code: None,
            },
        ],
        None,
    )
    .unwrap();
    ledger
        .seal(
            &draft,
            &SealRequest {
                natural_key: key.into(),
                actor: "test".into(),
                policy_set: PolicySet::default(),
            },
        )
        .unwrap();
}

fn seal_year(ledger: &Ledger, year: i32) {
    for quarter in 1..=4 {
        let period = PeriodId::new(year, quarter).unwrap();
        let authority = ledger
            .begin_close(&period, &format!("close-{period}"), "test")
            .unwrap();
        ledger.seal_period(&period, &authority, "test").unwrap();
    }
    let view = ledger
        .seal_year(
            year,
            &format!("{year:064x}"),
            &format!("year-{year}"),
            "test",
        )
        .unwrap();
    assert_eq!(view.state, FiscalYearState::Sealed);
}

fn row_amount(report: &aggountant_ledger::IncomeSection, id: &str) -> String {
    report
        .rows
        .iter()
        .find(|row| row.row_id == id)
        .unwrap()
        .amount
        .clone()
}

#[test]
fn month_ytd_balance_opening_and_future_cutoff_share_one_native_result() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    seal(&ledger, "2025-01-02", "1910", "2000", "1000.00", "capital");
    seal_year(&ledger, 2025);
    seal(&ledger, "2026-01-10", "1910", "3000", "100.00", "jan-sale");
    seal(&ledger, "2026-07-05", "1910", "3000", "200.00", "july-sale");
    seal(
        &ledger,
        "2026-07-20",
        "7690",
        "1910",
        "50.00",
        "july-expense",
    );
    seal(
        &ledger,
        "2026-08-01",
        "1910",
        "3000",
        "999.00",
        "future-sale",
    );

    let july = PeriodId::parse("2026-07").unwrap();
    let authority = ledger.begin_close(&july, "close-july", "test").unwrap();
    ledger.seal_period(&july, &authority, "test").unwrap();
    let report = ledger
        .monthly_statements_report(
            &july,
            "Esimerkki Oy",
            MonthlyReportOptions {
                history_start: Some(Date::parse("2025-01-01").unwrap()),
                limitation: Some("Aiempi aineisto ilmoitettu, ei varmennettu".into()),
            },
        )
        .unwrap();

    assert_eq!(report.status.presentation, "closed");
    assert_eq!(report.status.user_approval, "not_asserted");
    assert_eq!(report.completeness.state, "unverified");
    assert_eq!(
        row_amount(&report.sections.month_income, "IS-LIIKEVAIHTO"),
        "200.00"
    );
    assert_eq!(
        row_amount(&report.sections.month_income, "IS-MUUT-KULUT"),
        "50.00"
    );
    assert_eq!(report.sections.month_income.result, "150.00");
    assert_eq!(
        row_amount(&report.sections.year_to_date_income, "IS-LIIKEVAIHTO"),
        "300.00"
    );
    assert_eq!(report.sections.year_to_date_income.result, "250.00");
    assert_eq!(report.sections.balance.assets_total, "1250.00");
    assert_eq!(report.sections.balance.equity_liabilities_total, "1250.00");
    assert!(report.sections.balance.balanced);
    assert_eq!(
        report.sources.len(),
        3,
        "August must be excluded from the pinned set"
    );
    assert!(
        report
            .sources
            .iter()
            .all(|source| source.source_fingerprint.is_none()),
        "legacy/direct seals without source fingerprints remain readable"
    );
    assert!(report.provenance.opening_balance_source.is_some());
    assert_eq!(report.provenance.source_set_hash.len(), 64);

    let result = report
        .sections
        .balance
        .rows
        .iter()
        .find(|row| row.row_id == "BS-TILIKAUDEN-TULOS")
        .unwrap();
    assert_eq!(result.amount, "250.00");
    assert!(result
        .accounts
        .iter()
        .any(|account| account.account == "3000"));
    assert!(result
        .accounts
        .iter()
        .any(|account| account.account == "7690" && account.amount == "-50.00"));
    let july_trace = report
        .sections
        .month_income
        .rows
        .iter()
        .find(|row| row.row_id == "IS-LIIKEVAIHTO")
        .unwrap()
        .accounts[0]
        .sources
        .iter()
        .find(|source| {
            matches!(
                source,
                aggountant_ledger::ReportSourceRef::SealedVoucher {
                    voucher_number: 3,
                    ..
                }
            )
        })
        .unwrap();
    assert!(
        matches!(july_trace, aggountant_ledger::ReportSourceRef::SealedVoucher { original_document_date, original_posting_date, effective_posting_date, chart_version, policy_set, .. } if original_document_date == original_posting_date && original_posting_date == effective_posting_date && !chart_version.is_empty() && !policy_set.is_empty())
    );
}

#[test]
fn report_refuses_a_locally_consistent_but_broken_voucher_chain() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    seal(&ledger, "2026-07-01", "1910", "3000", "10.00", "sale");

    let connection = rusqlite::Connection::open(home.path().join("fiscal.sqlite")).unwrap();
    let canonical: Vec<u8> = connection
        .query_row(
            "SELECT canonical_bytes FROM vouchers WHERE number = 1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let false_previous = aggountant_model::Hash::from_bytes([7; 32]);
    let locally_consistent_hash = aggountant_wire::chain_hash(&false_previous, &canonical);
    connection
        .execute(
            "UPDATE vouchers SET prev_hash = ?1, hash = ?2 WHERE number = 1",
            rusqlite::params![
                false_previous.as_bytes().as_slice(),
                locally_consistent_hash.as_bytes().as_slice()
            ],
        )
        .unwrap();

    let error = ledger
        .monthly_statements_report(
            &PeriodId::parse("2026-07").unwrap(),
            "Test Oy",
            MonthlyReportOptions::default(),
        )
        .unwrap_err();
    assert_eq!(error.error_code(), "HASH_CHAIN_BROKEN");
}

#[test]
fn attributed_projection_preserves_original_sealed_identity() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    seal(
        &ledger,
        "2027-01-15",
        "1910",
        "3000",
        "75.00",
        "late-adjustment",
    );
    ledger
        .attribute_adjustment(1, 2026, "close-year-2026", "test")
        .unwrap();
    let report = ledger
        .monthly_statements_report(
            &PeriodId::parse("2026-12").unwrap(),
            "Test Oy",
            MonthlyReportOptions::default(),
        )
        .unwrap();
    assert_eq!(report.sections.year_to_date_income.result, "75.00");
    assert_eq!(report.sources.len(), 1);
    let source = &report.sources[0];
    assert_eq!(
        source.original_posting_date,
        Date::parse("2027-01-15").unwrap()
    );
    assert_eq!(
        source.effective_posting_date,
        Date::parse("2026-12-31").unwrap()
    );
    assert_eq!(source.voucher_number, 1);
    assert_eq!(source.voucher_hash.len(), 64);
}

#[test]
fn empty_report_keeps_zero_rows_and_html_escapes_long_content() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    let mut report = ledger
        .monthly_statements_report(
            &PeriodId::parse("2026-07").unwrap(),
            "Pitkä & <Yritys> \"Oy\" nimeltään erittäin erittäin erittäin pitkä",
            MonthlyReportOptions::default(),
        )
        .unwrap();
    assert!(report
        .sections
        .month_income
        .rows
        .iter()
        .all(|row| row.amount == "0.00"));
    assert_eq!(report.sections.month_income.result, "0.00");
    assert!(report.sources.is_empty());
    report.sections.month_income.rows[0]
        .accounts
        .push(aggountant_ledger::ReportAccount {
            account: "3000".into(),
            account_name: "Hyvin pitkä & <tilinimi> joka ei saa rikkoa taulukkoa".repeat(4),
            amount: "0.00".into(),
            sources: vec![],
        });
    let html = render_monthly_report_html(&report);
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("Pitkä &amp; &lt;Yritys&gt; &quot;Oy&quot;"));
    assert!(html.contains("Hyvin pitkä &amp; &lt;tilinimi&gt;"));
    assert!(html.contains("0,00 EUR"));
    assert!(html.contains("Kattavuus: varmistamaton"));
    assert!(!html.contains("<Yritys>"));

    let native = serde_json::to_string(&report).unwrap();
    let decoded: aggountant_ledger::MonthlyStatementsReport =
        serde_json::from_str(&native).unwrap();
    assert_eq!(
        decoded.sections.month_income.result,
        report.sections.month_income.result
    );
}
