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

#[test]
fn vat_codes_on_lists_what_a_seal_accepts_on_that_date() {
    let codes = vat_codes_on(date("2026-03-15"));
    let find = |code: &str| codes.iter().find(|o| o.code == code);

    let standard = find("sales-domestic-standard").unwrap();
    assert_eq!(standard.percent, "25.5");
    assert!(!standard.accepts_partial_deduction);
    let reduced = find("purchase-domestic-reduced-1").unwrap();
    assert_eq!(reduced.percent, "13.5");
    assert!(reduced.accepts_partial_deduction);
    assert!(find("sales-eu-goods").unwrap().requires_buyer_vat_id);
    // Refused at record time: an EU acquisition is reverse-charged, not zero-rated.
    assert!(find("purchase-eu-goods").is_none());
    assert!(find("purchase-eu-services").is_none());
    let margin = find("sales-margin-vehicle").unwrap();
    assert_eq!(margin.scheme, "margin");
    assert_eq!(margin.rate_class, MARGIN_RATE_CLASS);
    assert!(find("purchase-margin-vehicle").is_none());

    // Every listed code passes the parser and the guards a seal runs.
    for option in &codes {
        parse_vat_code(&option.code).unwrap();
        refuse_eu_acquisition_code(&option.code).unwrap();
    }

    // The table is dated: the same class has the rate of the date asked for, and a
    // date before the table has no codes at all.
    let old = vat_codes_on(date("2024-08-31"));
    let old_standard = old
        .iter()
        .find(|o| o.code == "sales-domestic-standard")
        .unwrap();
    assert_eq!(old_standard.percent, "24");
    assert!(vat_codes_on(date("2012-12-31")).is_empty());
}

#[test]
fn a_zero_rated_sale_alone_or_with_other_sales_is_accepted() {
    for code in ["sales-zero-rated", "sales-eu-goods", "sales-eu-services"] {
        check_zero_rated_sale_not_mixed(&eu_sale_postings(code)).unwrap();
        let mut with_domestic = eu_sale_postings(code);
        with_domestic.push(Posting {
            account: AccountId::new("3000").unwrap(),
            amount: eur_money("100.00"),
            side: Side::Credit,
            vat_code: Some(VatCode::new("sales-domestic-standard").unwrap()),
        });
        with_domestic.push(Posting {
            account: AccountId::new("1910").unwrap(),
            amount: eur_money("100.00"),
            side: Side::Debit,
            vat_code: None,
        });
        check_zero_rated_sale_not_mixed(&with_domestic).unwrap();
    }
    // A purchase on its own, or a domestic sale mixed with a purchase (the legacy
    // self-assessment shape), is not this guard's concern.
    check_zero_rated_sale_not_mixed(&reverse_charge_postings(
        "100.00",
        "purchase-domestic-standard",
        "sales-domestic-standard",
    ))
    .unwrap();
}

#[test]
fn a_zero_rated_sale_mixed_with_a_purchase_line_is_refused() {
    for code in ["sales-zero-rated", "sales-eu-goods", "sales-eu-services"] {
        let mut postings = eu_sale_postings(code);
        postings.push(Posting {
            account: AccountId::new("4000").unwrap(),
            amount: eur_money("40.00"),
            side: Side::Debit,
            vat_code: Some(VatCode::new("purchase-domestic-standard").unwrap()),
        });
        postings.push(Posting {
            account: AccountId::new("1910").unwrap(),
            amount: eur_money("40.00"),
            side: Side::Credit,
            vat_code: None,
        });
        let err = check_zero_rated_sale_not_mixed(&postings).unwrap_err();
        assert_eq!(err.error_code(), "VAT_STRUCTURE", "{code}");
        let message = err.to_string();
        assert!(message.contains(code), "{message}");
        assert!(message.contains("separate vouchers"), "{message}");
    }
}
