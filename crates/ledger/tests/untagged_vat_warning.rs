//! `voucher validate` warns about a VAT-account posting the return would leave
//! out, without refusing it (issue `vat-untagged-posting-omitted`). A VAT payment
//! or refund is recognised by its bank or tax-account leg
//! (`vat-settlement-shape-hides-untagged-reductions`).

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
            report.warnings[0].account.as_deref()
        ),
        (Some(2), Some("2939"))
    );

    let tagged = ledger
        .validate(&draft(Some("sales-domestic-standard")))
        .unwrap();
    assert!(tagged.warnings.is_empty());
}

fn untagged(postings: &[(&str, Side, &str)]) -> VoucherDraft {
    let d = Date::parse("2026-03-05").unwrap();
    VoucherDraft::new(
        d,
        d,
        "Esimerkki Oy".into(),
        postings
            .iter()
            .map(|(account, side, amount)| Posting {
                account: AccountId::new(*account).unwrap(),
                amount: Money::parse(amount, Currency::new("EUR").unwrap()).unwrap(),
                side: *side,
                vat_code: None,
            })
            .collect(),
        None,
    )
    .unwrap()
}

fn vat_payment() -> VoucherDraft {
    untagged(&[
        ("2939", Side::Debit, "400.00"),
        ("1910", Side::Credit, "400.00"),
    ])
}

#[test]
fn validate_warns_about_untagged_reductions_that_are_not_vat_payments() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    for (name, draft, account) in [
        (
            "advance refund",
            untagged(&[
                ("2620", Side::Debit, "100.00"),
                ("2939", Side::Debit, "25.50"),
                ("1910", Side::Credit, "125.50"),
            ]),
            "2939",
        ),
        (
            "fixed-asset purchase return",
            untagged(&[
                ("2700", Side::Debit, "1255.00"),
                ("1150", Side::Credit, "1000.00"),
                ("1763", Side::Credit, "255.00"),
            ]),
            "1763",
        ),
        (
            "private-use correction",
            untagged(&[
                ("2000", Side::Debit, "25.50"),
                ("1763", Side::Credit, "25.50"),
            ]),
            "1763",
        ),
    ] {
        let report = ledger.validate(&draft).unwrap();
        assert_eq!(report.warnings.len(), 1, "{name}");
        let warning = &report.warnings[0];
        assert_eq!(
            (warning.code, warning.account.as_deref()),
            ("VAT_POSTING_UNTAGGED", Some(account)),
            "{name}"
        );
        assert!(warning.message.contains("VAT base"), "{name}");
    }
    assert!(ledger.validate(&vat_payment()).unwrap().warnings.is_empty());
}

#[test]
fn without_settlement_roles_a_vat_payment_warns_about_the_roles_not_the_vat_code() {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    std::fs::remove_file(
        home.path()
            .join("masterdata/reference/settlement-account-roles.json"),
    )
    .unwrap();
    let ledger = Ledger::open(home.path()).unwrap();
    let report = ledger.validate(&vat_payment()).unwrap();
    assert_eq!(report.warnings.len(), 1);
    let message = &report.warnings[0].message;
    assert!(message.contains("aggountant home upgrade"), "{message}");
    assert!(!message.contains("VAT base"), "{message}");
}
