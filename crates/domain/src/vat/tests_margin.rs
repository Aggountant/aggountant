use super::*;

#[test]
fn compute_margin_vat_extracts_vat_from_within_the_margin() {
    // A used-goods resale: bought (from a private person, no deductible input VAT)
    // for 3745, sold for 5000. Margin = 1255, VAT-inclusive at 25,5 %: the VAT is
    // extracted from *within* it — 1255 × 25,5/125,5 = 255.00 — and the taxable
    // base (margin without the tax portion, AVL 79 k §) is 1255 − 255 = 1000.00.
    let m = compute_margin_vat(
        "sales-margin-used-goods",
        &eur_money("5000.00"),
        &eur_money("3745.00"),
        date("2026-02-15"),
    )
    .unwrap();
    assert_eq!(m.category, MarginCategory::UsedGoods);
    assert_eq!(m.margin.amount_str(), "1255.00");
    assert_eq!(m.taxable_base.amount_str(), "1000.00");
    assert_eq!(m.vat.amount_str(), "255.00");
}

#[test]
fn a_negative_per_item_margin_yields_no_vat() {
    // AVL 79 k § per-item method: a loss item (sold 1000, bought 1500) has a
    // negative margin, which is not creditable — no VAT, zero base — but the
    // signed margin is still reported so the caller sees the loss.
    let m = compute_margin_vat(
        "sales-margin-vehicle",
        &eur_money("1000.00"),
        &eur_money("1500.00"),
        date("2026-02-15"),
    )
    .unwrap();
    assert_eq!(m.margin.amount_str(), "-500.00");
    assert_eq!(m.taxable_base.amount_str(), "0.00");
    assert_eq!(m.vat.amount_str(), "0.00");
}

#[test]
fn a_zero_margin_yields_no_vat() {
    let m = compute_margin_vat(
        "sales-margin-art",
        &eur_money("1000.00"),
        &eur_money("1000.00"),
        date("2026-02-15"),
    )
    .unwrap();
    assert_eq!(m.margin.amount_str(), "0.00");
    assert_eq!(m.taxable_base.amount_str(), "0.00");
    assert_eq!(m.vat.amount_str(), "0.00");
}

#[test]
fn compute_margin_vat_resolves_the_rate_by_document_date() {
    // Before 2024-09-01 the standard rate was 24 %: a 1240 margin (bought 1000,
    // sold 2240) extracts 1240 × 24/124 = 240.00, base 1000.00.
    let m = compute_margin_vat(
        "sales-margin-antiques",
        &eur_money("2240.00"),
        &eur_money("1000.00"),
        date("2024-06-30"),
    )
    .unwrap();
    assert_eq!(m.vat.amount_str(), "240.00");
    assert_eq!(m.taxable_base.amount_str(), "1000.00");
}

#[test]
fn compute_margin_vat_at_the_2024_09_01_boundary() {
    // The extraction rate itself changes at the boundary: the same 1255 margin
    // (bought 3745, sold 5000) extracts 1255 × 24/124 = 242.90 on the last 24 %
    // day and 1255 × 25,5/125,5 = 255.00 on the first 25,5 % day.
    let sold = eur_money("5000.00");
    let bought = eur_money("3745.00");
    let before = compute_margin_vat(
        "sales-margin-used-goods",
        &sold,
        &bought,
        date("2024-08-31"),
    )
    .unwrap();
    let after = compute_margin_vat(
        "sales-margin-used-goods",
        &sold,
        &bought,
        date("2024-09-01"),
    )
    .unwrap();
    assert_eq!(before.vat.amount_str(), "242.90", "1255 × 24/124");
    assert_eq!(after.vat.amount_str(), "255.00", "1255 × 25,5/125,5");
}

#[test]
fn compute_margin_vat_rounds_the_extracted_vat_half_away() {
    // A 100.00 margin (bought 1000, sold 1100) × 25,5/125,5 = 20.31872… → round
    // half away = 20.32; base is the exact complement 100.00 − 20.32 = 79.68
    // (base + vat = margin always).
    let m = compute_margin_vat(
        "sales-margin-collectibles",
        &eur_money("1100.00"),
        &eur_money("1000.00"),
        date("2026-02-15"),
    )
    .unwrap();
    assert_eq!(m.vat.amount_str(), "20.32");
    assert_eq!(m.taxable_base.amount_str(), "79.68");
}

#[test]
fn a_one_cent_margin_extracts_no_vat() {
    // A positive but sub-rate margin: 0.01 × 25,5/125,5 = 0.00203… → rounds to
    // 0.00. The base keeps the whole cent (base + vat = margin). Documents that a
    // micro-margin yields no VAT rather than a spurious cent.
    let m = compute_margin_vat(
        "sales-margin-used-goods",
        &eur_money("1000.01"),
        &eur_money("1000.00"),
        date("2026-02-15"),
    )
    .unwrap();
    assert_eq!(m.margin.amount_str(), "0.01");
    assert_eq!(m.vat.amount_str(), "0.00");
    assert_eq!(m.taxable_base.amount_str(), "0.01");
}

#[test]
fn compute_margin_vat_refuses_a_mixed_currency_pair() {
    // The selling and purchase prices must share the EUR currency; a USD purchase
    // is refused rather than silently treated as EUR.
    let usd = Money::parse("3745.00", Currency::new("USD").unwrap()).unwrap();
    let err = compute_margin_vat(
        "sales-margin-used-goods",
        &eur_money("5000.00"),
        &usd,
        date("2026-02-15"),
    )
    .unwrap_err();
    assert_eq!(err.error_code(), "VAT_CURRENCY");
}

#[test]
fn the_margin_rate_class_resolves_in_the_bundled_table() {
    // Guard the MARGIN_RATE_CLASS -> table coupling: if the standard rate class
    // were renamed or removed, every margin code would break at derive time. This
    // pins that the constant resolves against the bundled data on a current date.
    assert!(resolve_vat_rate(MARGIN_RATE_CLASS, date("2026-02-15")).is_ok());
}

#[test]
fn compute_margin_vat_needs_a_margin_code() {
    // A normal (non-margin) sales code is refused — there is no margin to split.
    let err = compute_margin_vat(
        "sales-domestic-standard",
        &eur_money("5000.00"),
        &eur_money("3745.00"),
        date("2026-02-15"),
    )
    .unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn compute_margin_vat_refuses_a_purchase_margin_code() {
    // The margin scheme is a reseller's output VAT — a purchase- code is refused
    // in parse_vat_code before any computation.
    let err = compute_margin_vat(
        "purchase-margin-used-goods",
        &eur_money("5000.00"),
        &eur_money("3745.00"),
        date("2026-02-15"),
    )
    .unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn compute_margin_vat_refuses_a_negative_price() {
    let eur = Currency::new("EUR").unwrap();
    let err = compute_margin_vat(
        "sales-margin-used-goods",
        &Money::new(Decimal::from(-5000), eur.clone()),
        &eur_money("3745.00"),
        date("2026-02-15"),
    )
    .unwrap_err();
    assert_eq!(err.error_code(), "VAT_CURRENCY");
}

#[test]
fn derive_extracts_margin_vat_from_a_sealed_voucher() {
    // The tagged line carries the VAT-inclusive margin (1255); deriving a sealed
    // margin sale extracts the 255.00 output VAT from within it, into sales_vat.
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "3000",
        "1255.00",
        "sales-margin-used-goods",
        "1910",
    )];
    let d = derive_vat_figures(&q1(), &vouchers).unwrap();
    assert_eq!(d.sales_vat.amount_str(), "255.00");
    assert_eq!(d.purchase_vat.amount_str(), "0.00");
}

#[test]
fn a_margin_credit_note_reduces_the_output_vat() {
    // A margin sale (1255 margin → +255) and a 1255 margin credit note (debit
    // side → −255): the period's margin output VAT nets to 0.00.
    let vouchers = vec![
        vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "3000",
            "1255.00",
            "sales-margin-used-goods",
            "1910",
        ),
        vat_voucher_sided(
            2,
            "2026-02-15",
            "2026-02-15",
            "3000",
            "1255.00",
            "sales-margin-used-goods",
            Side::Debit,
            "1910",
        ),
    ];
    let d = derive_vat_figures(&q1(), &vouchers).unwrap();
    assert_eq!(d.sales_vat.amount_str(), "0.00");
}

#[test]
fn margin_vat_is_strictly_less_than_normal_vat_on_the_same_base() {
    // The whole point of the scheme: the same 1255 base yields less VAT under the
    // margin scheme (255.00, extracted) than under the normal scheme (320.03, on
    // top), because the margin is VAT-inclusive.
    let base = "1255.00";
    let normal = derive_vat_figures(
        &q1(),
        &[vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "3000",
            base,
            "sales-domestic-standard",
            "1910",
        )],
    )
    .unwrap();
    let margin = derive_vat_figures(
        &q1(),
        &[vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "3000",
            base,
            "sales-margin-used-goods",
            "1910",
        )],
    )
    .unwrap();
    assert_eq!(normal.sales_vat.amount_str(), "320.03");
    assert_eq!(margin.sales_vat.amount_str(), "255.00");
    assert!(margin.sales_vat.amount() < normal.sales_vat.amount());
}

#[test]
fn every_margin_category_derives_at_the_standard_rate() {
    // All five AVL 79 a § categories are standard-rated on the margin, so each
    // extracts the same 255.00 from a 1255 margin.
    for cat in [
        "sales-margin-used-goods",
        "sales-margin-art",
        "sales-margin-antiques",
        "sales-margin-collectibles",
        "sales-margin-vehicle",
    ] {
        let d = derive_vat_figures(
            &q1(),
            &[vat_voucher(
                1,
                "2026-02-15",
                "2026-02-15",
                "3000",
                "1255.00",
                cat,
                "1910",
            )],
        )
        .unwrap();
        assert_eq!(d.sales_vat.amount_str(), "255.00", "for {cat}");
    }
}

#[test]
fn an_unknown_margin_category_is_refused() {
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "3000",
        "1255.00",
        "sales-margin-jewelry",
        "1910",
    )];
    let err = derive_vat_figures(&q1(), &vouchers).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn a_purchase_margin_code_is_refused_in_derivation() {
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "4000",
        "1255.00",
        "purchase-margin-used-goods",
        "1910",
    )];
    let err = derive_vat_figures(&q1(), &vouchers).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn a_margin_code_with_a_deduct_marker_is_refused() {
    // Margin output VAT is never fractional — combining the scheme with a
    // partial-deductibility marker is a contradiction, refused at parse.
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "3000",
        "1255.00",
        "sales-margin-used-goods-deduct50",
        "1910",
    )];
    let err = derive_vat_figures(&q1(), &vouchers).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn a_reserved_margin_word_in_a_rate_class_is_rejected_at_load() {
    // `margin` is reserved like `deduct`: a rate class containing it would be
    // misparsed as the scheme infix, so the table validator rejects it.
    let table = VatRateTable {
        table: "t".into(),
        editions: vec![VatRateEdition {
            rate_class: "margin-foo".into(),
            valid_from: date("2020-01-01"),
            valid_to: None,
            percent: "24.0".into(),
        }],
    };
    let err = validate_vat_rate_table(&table).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn a_margin_sale_passes_the_structural_guard() {
    // A single margin sales line is one positive sales-direction contribution —
    // no same-direction cancellation — so it builds normally.
    let postings = vec![
        Posting {
            account: AccountId::new("1910").unwrap(),
            amount: eur_money("1255.00"),
            side: Side::Debit,
            vat_code: None,
        },
        Posting {
            account: AccountId::new("3000").unwrap(),
            amount: eur_money("1255.00"),
            side: Side::Credit,
            vat_code: Some(VatCode::new("sales-margin-used-goods").unwrap()),
        },
    ];
    check_voucher_vat_structure(&postings).unwrap();
}

// --- ADR-7 (VAT-A) vat_expand golden vectors --------------------------------
