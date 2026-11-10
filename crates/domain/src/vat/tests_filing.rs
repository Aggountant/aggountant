use super::*;

pub(super) fn eur_money(s: &str) -> Money {
    Money::parse(s, Currency::new("EUR").unwrap()).unwrap()
}
pub(super) fn q1() -> PeriodId {
    PeriodId::parse("2026-Q1").unwrap()
}

#[test]
fn generic_rate_classes_own_the_three_domestic_return_slots() {
    assert_eq!(
        vat_rate_class_field("domestic-standard").unwrap(),
        VatReturnField::Field301
    );
    assert_eq!(
        vat_rate_class_field("domestic-reduced-1").unwrap(),
        VatReturnField::Field302
    );
    assert_eq!(
        vat_rate_class_field("domestic-reduced-2").unwrap(),
        VatReturnField::Field303
    );
    assert!(vat_rate_class_field("domestic-electricity").is_err());
}

#[test]
fn net_vat_is_sales_minus_purchases_payable() {
    let filing = compute_vat_filing(&VatFilingInput {
        period: q1(),
        sales_vat: eur_money("2400.00"),
        purchase_vat: eur_money("900.00"),
    })
    .unwrap();
    assert_eq!(filing.net_vat.amount_str(), "1500.00");
}

#[test]
fn net_vat_can_be_a_refund() {
    let filing = compute_vat_filing(&VatFilingInput {
        period: q1(),
        sales_vat: eur_money("100.00"),
        purchase_vat: eur_money("450.00"),
    })
    .unwrap();
    assert_eq!(filing.net_vat.amount_str(), "-350.00");
}

#[test]
fn reconcile_applies_existing_credit() {
    let filing = compute_vat_filing(&VatFilingInput {
        period: q1(),
        sales_vat: eur_money("2400.00"),
        purchase_vat: eur_money("900.00"),
    })
    .unwrap();
    // Net payable 1500; account already holds a 200 credit → pay 1300.
    let rec = reconcile_verotili(
        &filing,
        &TaxAccountSnapshot {
            balance: eur_money("200.00"),
            as_of: "2026-04-12T00:00:00Z".into(),
        },
    )
    .unwrap();
    assert_eq!(rec.bank_payment_required.amount_str(), "1300.00");
}

#[test]
fn reconcile_never_goes_negative_for_a_refund() {
    let filing = compute_vat_filing(&VatFilingInput {
        period: q1(),
        sales_vat: eur_money("100.00"),
        purchase_vat: eur_money("450.00"),
    })
    .unwrap();
    let rec = reconcile_verotili(
        &filing,
        &TaxAccountSnapshot {
            balance: eur_money("0.00"),
            as_of: "2026-04-12T00:00:00Z".into(),
        },
    )
    .unwrap();
    assert_eq!(rec.bank_payment_required.amount_str(), "0.00");
}

#[test]
fn existing_debt_does_not_increase_the_filing_payment() {
    // A pre-existing debt on the tax account is a separate matter; the filing's
    // own bank payment is just its net (credit offset only, never debt add-on).
    let filing = compute_vat_filing(&VatFilingInput {
        period: q1(),
        sales_vat: eur_money("1000.00"),
        purchase_vat: eur_money("0.00"),
    })
    .unwrap();
    let rec = reconcile_verotili(
        &filing,
        &TaxAccountSnapshot {
            balance: eur_money("-500.00"),
            as_of: "2026-04-12T00:00:00Z".into(),
        },
    )
    .unwrap();
    assert_eq!(rec.bank_payment_required.amount_str(), "1000.00");
}

// --- F4 derivation from sealed vouchers ---------------------------------
