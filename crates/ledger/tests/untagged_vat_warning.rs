//! `voucher validate` warns about a VAT-account posting the return would leave
//! out, without refusing it (issue `vat-untagged-posting-omitted`).

use aggountant_ledger::Ledger;
use aggountant_model::{AccountId, Currency, Date, Money, Posting, Side, VatCode, VoucherDraft};

fn draft(code: Option<&str>) -> VoucherDraft {
    let line = |account: &str, side: Side, amount: &str, code: Option<&str>| Posting {
        account: AccountId::new(account).unwrap(),
        amount: Money::parse(amount, Currency::new("EUR").unwrap()).unwrap(),
        side,
        vat_code: code.map(|c| VatCode::new(c).unwrap()),
    };
    let d = Date::parse("2026-03-05").unwrap();
    VoucherDraft::new(
        d,
        d,
        "myyntilasku".into(),
        vec![
            line("1700", Side::Debit, "2480.00", None),
            line("3000", Side::Credit, "1976.10", code),
            line("2939", Side::Credit, "503.90", None),
        ],
        None,
    )
    .unwrap()
}

#[test]
fn validate_warns_about_an_untagged_vat_leg_but_accepts_the_draft() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();

    let report = ledger.validate(&draft(None)).unwrap();
    assert!(report.ok);
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(report.warnings[0].code, "VAT_POSTING_UNTAGGED");
    assert_eq!(
        (
            report.warnings[0].posting_index,
            report.warnings[0].account.as_str()
        ),
        (2, "2939")
    );

    let tagged = ledger
        .validate(&draft(Some("sales-domestic-standard")))
        .unwrap();
    assert!(tagged.warnings.is_empty());
}
