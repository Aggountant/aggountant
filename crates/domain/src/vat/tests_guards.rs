use super::*;

#[test]
fn eu_sale_rate_classes_resolve_to_zero_rated() {
    // The codes must be real (resolvable) so a legitimate EU sale can be booked
    // and derived, not just refused.
    assert_eq!(
        resolve_vat_rate("eu-goods", date("2026-05-31")).unwrap(),
        Decimal::ZERO
    );
    assert_eq!(
        resolve_vat_rate("eu-services", date("2026-05-31")).unwrap(),
        Decimal::ZERO
    );
}

#[test]
fn eu_sale_kind_recognises_only_sales_eu_classes() {
    assert_eq!(eu_sale_kind("sales-eu-goods"), Some(EuSaleKind::Goods));
    assert_eq!(
        eu_sale_kind("sales-eu-services"),
        Some(EuSaleKind::Services)
    );
    // A domestic sale, a purchase, and a malformed code are not EU sales.
    assert_eq!(eu_sale_kind("sales-domestic-standard"), None);
    assert_eq!(eu_sale_kind("purchase-eu-goods"), None);
    assert_eq!(eu_sale_kind("nonsense"), None);
}

#[test]
fn eu_sale_without_buyer_vat_id_is_refused() {
    let err = check_eu_sale_buyer_vat_id(&eu_sale_postings("sales-eu-goods"), None).unwrap_err();
    assert_eq!(err.error_code(), "VAT_BUYER_ID");
}

#[test]
fn eu_sale_with_blank_buyer_vat_id_is_refused() {
    let err = check_eu_sale_buyer_vat_id(&eu_sale_postings("sales-eu-services"), Some("   "))
        .unwrap_err();
    assert_eq!(err.error_code(), "VAT_BUYER_ID");
}

#[test]
fn eu_sale_with_a_valid_other_member_state_vat_id_passes() {
    // Swedish and German buyers; spaces and lower-case are normalised away.
    check_eu_sale_buyer_vat_id(&eu_sale_postings("sales-eu-goods"), Some("SE556677889901"))
        .unwrap();
    assert_eq!(
        check_eu_sale_buyer_vat_id(
            &eu_sale_postings("sales-eu-services"),
            Some("de 811 569 869"),
        )
        .unwrap()
        .as_deref(),
        Some("DE811569869")
    );
}

#[test]
fn eu_sale_with_a_finnish_buyer_vat_id_is_refused() {
    // AVL 72 b § needs *another* member state — an FI buyer is domestic.
    let err = check_eu_sale_buyer_vat_id(&eu_sale_postings("sales-eu-goods"), Some("FI01234567"))
        .unwrap_err();
    assert_eq!(err.error_code(), "VAT_BUYER_ID");
}

#[test]
fn eu_sale_with_a_non_eu_country_prefix_is_refused() {
    // A UK (GB) or US number is not an EU member-state registration.
    let err = check_eu_sale_buyer_vat_id(&eu_sale_postings("sales-eu-goods"), Some("GB123456789"))
        .unwrap_err();
    assert_eq!(err.error_code(), "VAT_BUYER_ID");
}

#[test]
fn non_eu_sale_needs_no_buyer_vat_id_but_still_rejects_a_malformed_stray() {
    // A domestic sale needs no buyer VAT-ID. A stray one is still format-checked
    // (a present-but-junk ID signals a mistag — an EU-sale code the operator
    // forgot to set), but the "another member state" rule does not apply, so an
    // FI domestic buyer's own valid registration is fine here.
    let domestic = eu_sale_postings("sales-domestic-standard");
    check_eu_sale_buyer_vat_id(&domestic, None).unwrap();
    check_eu_sale_buyer_vat_id(&domestic, Some("FI01234567")).unwrap();
    let err = check_eu_sale_buyer_vat_id(&domestic, Some("not-a-vat-id")).unwrap_err();
    assert_eq!(err.error_code(), "VAT_BUYER_ID");
}

#[test]
fn interior_whitespace_cannot_smuggle_a_finnish_buyer_past_the_gate() {
    // Regression: `validate_eu_buyer_vat_id` strips *all* whitespace, so both the
    // format check and the FI check must agree on the normalised form. A naive
    // `trim_start` FI check let `"F I01234567"` (→ FI01234567) slip through.
    let err = check_eu_sale_buyer_vat_id(&eu_sale_postings("sales-eu-goods"), Some("F I0 1234567"))
        .unwrap_err();
    assert_eq!(err.error_code(), "VAT_BUYER_ID");
}

#[test]
fn a_non_ascii_buyer_vat_id_is_rejected_without_panicking() {
    // Regression: byte-slicing `split_at(2)` on a multibyte char would panic.
    assert!(validate_eu_buyer_vat_id("AÄ1234").is_err());
    let err = check_eu_sale_buyer_vat_id(&eu_sale_postings("sales-eu-goods"), Some("Ä1234567"))
        .unwrap_err();
    assert_eq!(err.error_code(), "VAT_BUYER_ID");
}

#[test]
fn northern_ireland_buyer_is_goods_only() {
    // XI is valid for an intra-Community supply of goods…
    check_eu_sale_buyer_vat_id(&eu_sale_postings("sales-eu-goods"), Some("XI123456789")).unwrap();
    // …but not services.
    let err =
        check_eu_sale_buyer_vat_id(&eu_sale_postings("sales-eu-services"), Some("XI123456789"))
            .unwrap_err();
    assert_eq!(err.error_code(), "VAT_BUYER_ID");
}

#[test]
fn purchase_eu_code_is_refused_as_a_zero_rated_acquisition() {
    // The eu-* rate classes exist for sales; a `purchase-eu-goods` acquisition
    // must be reverse-charged, not booked at 0 %.
    let postings = vec![
        Posting {
            account: AccountId::new("1763").unwrap(),
            amount: eur_money("10000.00"),
            side: Side::Debit,
            vat_code: Some(VatCode::new("purchase-eu-goods").unwrap()),
        },
        Posting {
            account: AccountId::new("2939").unwrap(),
            amount: eur_money("10000.00"),
            side: Side::Credit,
            vat_code: None,
        },
    ];
    let err = check_no_eu_acquisition_code(&postings).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
    // A sales EU code and a plain purchase are untouched.
    check_no_eu_acquisition_code(&eu_sale_postings("sales-eu-goods")).unwrap();
}

#[test]
fn a_partial_deductibility_marker_cannot_smuggle_an_eu_acquisition_through() {
    // The `-deduct50` marker is stripped before the eu-goods/eu-services match, so
    // `purchase-eu-goods-deduct50` is still caught as a zero-rated EU acquisition
    // that must be reverse-charged — the marker offers no bypass.
    let postings = vec![
        Posting {
            account: AccountId::new("1763").unwrap(),
            amount: eur_money("10000.00"),
            side: Side::Debit,
            vat_code: Some(VatCode::new("purchase-eu-goods-deduct50").unwrap()),
        },
        Posting {
            account: AccountId::new("2939").unwrap(),
            amount: eur_money("10000.00"),
            side: Side::Credit,
            vat_code: None,
        },
    ];
    let err = check_no_eu_acquisition_code(&postings).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn validate_eu_buyer_vat_id_checks_shape_and_returns_the_normalised_form() {
    // Recognised prefix but an empty / non-alphanumeric / over-long body.
    assert!(validate_eu_buyer_vat_id("SE").is_err());
    assert!(validate_eu_buyer_vat_id("SE12-34").is_err());
    assert!(validate_eu_buyer_vat_id("SE1234567890123").is_err());
    assert!(validate_eu_buyer_vat_id("EL123456789").is_ok());
    // The returned value is the canonical (whitespace-stripped, upper-cased) ID.
    assert_eq!(
        validate_eu_buyer_vat_id("se 556677889901").unwrap(),
        "SE556677889901"
    );
}

// --- AVL 79 a–k § second-hand margin scheme --------------------------
