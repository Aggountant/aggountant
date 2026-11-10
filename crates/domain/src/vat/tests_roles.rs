use super::*;

const BRIOX_CHART_VERSION: &str = "example-briox-2026@fixture";
const CUSTOM_CHART_VERSION: &str = "synthetic-company@roles-v1";
const SHARED_CHART_VERSION: &str = "shared-company@roles-v1";
const DEFAULT_CHART_VERSION: &str =
    "fi-general-2026@dd5d3f339cebfbc670f55cbd0942c775904c58523d9b07ad8eb48b589c79b63b";
const TEST_ROLE_TABLE_JSON: &str = r#"{
  "table":"test-vat-account-roles",
  "charts":[
    {"chart_version":"fi-general-2026@0","active":true,"accounts":[
      {"account":"1763","role":"input-vat"},{"account":"2939","role":"tagged-only"}
    ],"mistag_guards":[{"account":"7010","dedicated_rate_class":"domestic-electricity"}]},
    {"chart_version":"example-briox-2026@fixture","active":true,"accounts":[
      {"account":"1763","role":"input-vat"},{"account":"1797","role":"input-vat"},
      {"account":"1798","role":"input-vat"},
      {"account":"2982","role":"reverse-charge-output-vat","reverse_charge_scope":"non-eu-services","paired_input_account":"1763"},
      {"account":"2991","role":"reverse-charge-output-vat","reverse_charge_scope":"eu-goods","paired_input_account":"1797"},
      {"account":"2992","role":"reverse-charge-output-vat","reverse_charge_scope":"eu-services","paired_input_account":"1798"},
      {"account":"2939","role":"tagged-only"}
    ]},
    {"chart_version":"synthetic-company@roles-v1","active":true,"accounts":[
      {"account":"1801","role":"input-vat"},
      {"account":"1802","role":"input-vat"},
      {"account":"1803","role":"input-vat"},
      {"account":"2901","role":"reverse-charge-output-vat","reverse_charge_scope":"non-eu-services","paired_input_account":"1801"},
      {"account":"2902","role":"reverse-charge-output-vat","reverse_charge_scope":"eu-goods","paired_input_account":"1802"},
      {"account":"2903","role":"reverse-charge-output-vat","reverse_charge_scope":"eu-services","paired_input_account":"1803"},
      {"account":"2999","role":"tagged-only"}
    ]},
    {"chart_version":"shared-company@roles-v1","active":true,"accounts":[
      {"account":"1763","role":"input-vat"},
      {"account":"2939","role":"tagged-only","generated_role":"output"}
    ],"domestic_input_account":"1763","reverse_charge_targets":[
      {"scope":"eu-services","input_account":"1763","output_account":"2939"},
      {"scope":"non-eu-services","input_account":"1763","output_account":"2939"}
    ]},
    {"chart_version":"unreviewed-import@fixture","active":false,"accounts":[]}
  ]
}"#;

fn test_role_table() -> VatAccountRoleTable {
    VatAccountRoleTable::from_json(TEST_ROLE_TABLE_JSON).unwrap()
}

fn test_sum(period: &PeriodId, vouchers: &[VoucherBody]) -> Result<DerivedVatFigures, DomainError> {
    sum_vat_account_figures_with_role_table(period, vouchers, &test_role_table())
}

fn test_derive(
    period: &PeriodId,
    vouchers: &[VoucherBody],
) -> Result<DerivedVatFigures, DomainError> {
    derive_filing_figures_with_role_table(period, vouchers, &test_role_table())
}

fn test_verify(voucher: &VoucherBody) -> Result<VatRecompute, DomainError> {
    verify_vat_expansion_with_role_table(voucher, &test_role_table())
}

fn custom_expand(input: &VatExpandInput) -> Result<VoucherDraft, DomainError> {
    vat_expand_with_role_table(input, CUSTOM_CHART_VERSION, &test_role_table())
}

fn shared_expand_body(number: u64, input: &VatExpandInput) -> VoucherBody {
    let draft =
        vat_expand_with_role_table(input, SHARED_CHART_VERSION, &test_role_table()).unwrap();
    VoucherBody::from_draft(&draft, number, SHARED_CHART_VERSION.into()).unwrap()
}

fn custom_expand_body(number: u64, input: &VatExpandInput) -> VoucherBody {
    let draft = custom_expand(input).unwrap();
    VoucherBody::from_draft(&draft, number, CUSTOM_CHART_VERSION.into()).unwrap()
}

fn custom_leg<'a>(draft: &'a VoucherDraft, account: &str, side: Side) -> &'a Posting {
    draft
        .postings()
        .iter()
        .find(|posting| posting.account.as_str() == account && posting.side == side)
        .unwrap_or_else(|| panic!("missing {side:?} leg on {account}"))
}

fn test_mistag(
    chart_version: &str,
    document_date: Date,
    postings: &[Posting],
) -> Result<(), DomainError> {
    check_vat_rate_class_mistag_with_role_table(
        &test_role_table(),
        chart_version,
        document_date,
        postings,
    )
}

/// Expand with the existing pure VAT engine, then bind the sealed body to the imported
/// Briox snapshot whose metadata owns the reverse-charge account meanings.
fn company_expand_body(
    number: u64,
    amount: &str,
    net_account: &str,
    contra_account: &str,
    treatment: VatTreatment,
    document_date: &str,
    posting_date: &str,
) -> VoucherBody {
    let mut input = expand_input(
        amount,
        net_account,
        contra_account,
        treatment,
        document_date,
    );
    input.posting_date = date(posting_date);
    let draft = vat_expand(&input).unwrap();
    VoucherBody::from_draft(&draft, number, BRIOX_CHART_VERSION.into()).unwrap()
}

#[test]
fn chart_aware_expansion_uses_synthetic_targets_for_every_treatment() {
    let domestic = custom_expand(&expand_input(
        "125.50",
        "4000",
        "1910",
        domestic_standard(),
        "2026-02-15",
    ))
    .unwrap();
    assert_eq!(
        custom_leg(&domestic, "1801", Side::Debit)
            .amount
            .amount_str(),
        "25.50"
    );

    for (scope, input_account, output_account) in [
        (ReverseChargeScope::EuServices, "1803", "2903"),
        (ReverseChargeScope::EuGoods, "1802", "2902"),
        (ReverseChargeScope::NonEuServices, "1801", "2901"),
    ] {
        let draft = custom_expand(&expand_input(
            "100.00",
            "4000",
            "1910",
            reverse_charge(scope),
            "2026-02-15",
        ))
        .unwrap();
        assert_eq!(
            custom_leg(&draft, input_account, Side::Debit)
                .amount
                .amount_str(),
            "25.50"
        );
        assert_eq!(
            custom_leg(&draft, output_account, Side::Credit)
                .amount
                .amount_str(),
            "25.50"
        );
        assert!(draft.postings().iter().all(|posting| {
            !["1763", "1797", "1798", "2982", "2991", "2992"].contains(&posting.account.as_str())
        }));
    }
}

#[test]
fn shared_service_pair_preserves_treatment_and_drives_distinct_return_fields() {
    let eu_input = expand_input(
        "100.00",
        "4000",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-02-15",
    );
    let non_eu_input = expand_input(
        "200.00",
        "4000",
        "1910",
        reverse_charge(ReverseChargeScope::NonEuServices),
        "2026-02-15",
    );
    let eu = shared_expand_body(1, &eu_input);
    let non_eu = shared_expand_body(2, &non_eu_input);
    for body in [&eu, &non_eu] {
        assert!(body
            .postings()
            .iter()
            .any(|posting| posting.account.as_str() == "1763" && posting.side == Side::Debit));
        assert!(body
            .postings()
            .iter()
            .any(|posting| posting.account.as_str() == "2939" && posting.side == Side::Credit));
        assert!(matches!(
            test_verify(body),
            Ok(VatRecompute::Verified { .. })
        ));
    }
    assert!(matches!(
        eu.vat_expansion().unwrap().treatment(),
        VatExpansionTreatment::ReverseCharge {
            scope: VatExpansionScope::EuServices,
            ..
        }
    ));
    assert!(matches!(
        non_eu.vat_expansion().unwrap().treatment(),
        VatExpansionTreatment::ReverseCharge {
            scope: VatExpansionScope::NonEuServices,
            ..
        }
    ));

    let filing = compute_derived_vat_filing(&test_derive(&q1(), &[eu, non_eu]).unwrap()).unwrap();
    for (field, amount) in [
        (VatReturnField::Field313, "100.00"),
        (VatReturnField::Field306, "25.50"),
        (VatReturnField::Field301, "51.00"),
        (VatReturnField::Field307, "76.50"),
        (VatReturnField::Field308, "0.00"),
    ] {
        assert_eq!(filing.fields.get(field).unwrap().amount_str(), amount);
    }
    assert!(filing.fields.get(VatReturnField::Field302).is_none());
    assert!(filing.fields.get(VatReturnField::Field303).is_none());
    assert!(filing.fields.get(VatReturnField::Field305).is_none());
    assert!(filing.fields.get(VatReturnField::Field312).is_none());
}

#[test]
fn non_eu_service_uses_each_generic_rate_class_slot() {
    for (rate_class, expected_field) in [
        ("domestic-standard", VatReturnField::Field301),
        ("domestic-reduced-1", VatReturnField::Field302),
        ("domestic-reduced-2", VatReturnField::Field303),
    ] {
        let body = shared_expand_body(
            1,
            &expand_input(
                "100.00",
                "4000",
                "1910",
                VatTreatment::ReverseCharge {
                    rate_class: rate_class.into(),
                    scope: ReverseChargeScope::NonEuServices,
                },
                "2026-02-15",
            ),
        );
        let filing = compute_derived_vat_filing(&test_derive(&q1(), &[body]).unwrap()).unwrap();
        assert!(filing.fields.get(expected_field).is_some());
        assert!(filing.fields.get(VatReturnField::Field313).is_none());
        assert!(filing.fields.get(VatReturnField::Field306).is_none());
    }
}

#[test]
fn shared_non_eu_partial_keeps_full_output_and_only_deductible_share() {
    let body = shared_expand_body(
        1,
        &expand_input(
            "100.00",
            "4000",
            "1910",
            reverse_charge_half_deductible(ReverseChargeScope::NonEuServices),
            "2026-02-15",
        ),
    );
    let filing = compute_derived_vat_filing(&test_derive(&q1(), &[body]).unwrap()).unwrap();
    assert_eq!(
        filing
            .fields
            .get(VatReturnField::Field301)
            .unwrap()
            .amount_str(),
        "25.50"
    );
    assert_eq!(
        filing
            .fields
            .get(VatReturnField::Field307)
            .unwrap()
            .amount_str(),
        "12.75"
    );
    assert_eq!(
        filing
            .fields
            .get(VatReturnField::Field308)
            .unwrap()
            .amount_str(),
        "12.75"
    );
    assert!(filing.fields.get(VatReturnField::Field313).is_none());
    assert!(filing.fields.get(VatReturnField::Field306).is_none());
}

#[test]
fn tagged_and_generated_use_of_shared_output_account_are_disjoint() {
    let generated = shared_expand_body(
        1,
        &expand_input(
            "100.00",
            "4000",
            "1910",
            reverse_charge(ReverseChargeScope::NonEuServices),
            "2026-02-15",
        ),
    );
    let tagged_draft = VoucherDraft::new(
        date("2026-02-15"),
        date("2026-02-15"),
        "tagged domestic sale on shared output account".into(),
        vec![
            posting(
                "2939",
                "100.00",
                Side::Credit,
                Some("sales-domestic-standard"),
            ),
            posting("1910", "100.00", Side::Debit, None),
        ],
        None,
    )
    .unwrap();
    let tagged = VoucherBody::from_draft(&tagged_draft, 2, SHARED_CHART_VERSION.into()).unwrap();
    let figures = test_derive(&q1(), &[generated, tagged]).unwrap();
    assert_eq!(figures.sales_vat.amount_str(), "51.00");
    assert_eq!(figures.purchase_vat.amount_str(), "25.50");
}

#[test]
fn chart_aware_partial_rounding_survives_seal_recompute_and_filing() {
    let input = expand_input(
        "125.50",
        "4000",
        "1910",
        domestic_half_deductible(),
        "2026-02-15",
    );
    let body = custom_expand_body(1, &input);
    assert_eq!(
        body.postings()
            .iter()
            .find(|posting| posting.account.as_str() == "1801")
            .unwrap()
            .amount
            .amount_str(),
        "12.75"
    );
    assert!(matches!(
        test_verify(&body).unwrap(),
        VatRecompute::Verified {
            treatment: VatTreatment::DomesticInputPartial { .. }
        }
    ));
    let figures = test_derive(&q1(), std::slice::from_ref(&body)).unwrap();
    assert_eq!(figures.sales_vat.amount_str(), "0.00");
    assert_eq!(figures.purchase_vat.amount_str(), "12.75");
    let filing = compute_vat_filing(&VatFilingInput {
        period: q1(),
        sales_vat: figures.sales_vat,
        purchase_vat: figures.purchase_vat,
    })
    .unwrap();
    assert_eq!(
        filing
            .fields
            .get(VatReturnField::Field307)
            .unwrap()
            .amount_str(),
        "12.75"
    );
    assert_eq!(
        filing
            .fields
            .get(VatReturnField::Field308)
            .unwrap()
            .amount_str(),
        "-12.75"
    );
}

#[test]
fn chart_aware_reverse_charge_round_trip_uses_one_exact_role_table() {
    for scope in [
        ReverseChargeScope::EuServices,
        ReverseChargeScope::EuGoods,
        ReverseChargeScope::NonEuServices,
    ] {
        let input = expand_input("10.10", "4000", "1910", reverse_charge(scope), "2026-02-15");
        let body = custom_expand_body(1, &input);
        assert!(matches!(
            test_verify(&body).unwrap(),
            VatRecompute::Verified {
                treatment: VatTreatment::ReverseCharge { scope: actual, .. }
            } if actual == scope
        ));
        let figures = test_derive(&q1(), &[body]).unwrap();
        assert_eq!(figures.sales_vat.amount_str(), "2.58");
        assert_eq!(figures.purchase_vat.amount_str(), "2.58");
        let filing = compute_derived_vat_filing(&figures).unwrap();
        assert_eq!(
            filing
                .fields
                .get(VatReturnField::Field307)
                .unwrap()
                .amount_str(),
            "2.58"
        );
        assert_eq!(
            filing
                .fields
                .get(VatReturnField::Field308)
                .unwrap()
                .amount_str(),
            "0.00"
        );
        match scope {
            ReverseChargeScope::EuServices => {
                assert_eq!(
                    filing
                        .fields
                        .get(VatReturnField::Field306)
                        .unwrap()
                        .amount_str(),
                    "2.58"
                );
                assert_eq!(
                    filing
                        .fields
                        .get(VatReturnField::Field313)
                        .unwrap()
                        .amount_str(),
                    "10.10"
                );
            }
            ReverseChargeScope::EuGoods => {
                assert_eq!(
                    filing
                        .fields
                        .get(VatReturnField::Field305)
                        .unwrap()
                        .amount_str(),
                    "2.58"
                );
                assert_eq!(
                    filing
                        .fields
                        .get(VatReturnField::Field312)
                        .unwrap()
                        .amount_str(),
                    "10.10"
                );
            }
            ReverseChargeScope::NonEuServices => {
                assert!(filing.fields.get(VatReturnField::Field305).is_none());
                assert!(filing.fields.get(VatReturnField::Field306).is_none());
            }
        }
    }
}

#[test]
fn chart_aware_expansion_fails_for_wrong_inactive_or_incomplete_versions() {
    let input = expand_input(
        "100.00",
        "4000",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-02-15",
    );
    for version in ["synthetic-company@roles-v2", "unreviewed-import@fixture"] {
        let error = vat_expand_with_role_table(&input, version, &test_role_table()).unwrap_err();
        assert_eq!(error.error_code(), "VAT_STRUCTURE");
    }
    let default_error =
        vat_expand_with_role_table(&input, "fi-general-2026@0", &test_role_table()).unwrap_err();
    assert_eq!(default_error.error_code(), "VAT_STRUCTURE");
    assert!(default_error
        .to_string()
        .contains("no reverse-charge role target"));
}

#[test]
fn chart_aware_expansion_rejects_ambiguous_domestic_and_role_collisions() {
    let ambiguous = VatAccountRoleTable::from_json(
        r#"{"table":"ambiguous","charts":[
          {"chart_version":"company@v1","active":true,"accounts":[
            {"account":"1801","role":"input-vat"},{"account":"1802","role":"input-vat"}
          ]},
          {"chart_version":"company@v2","active":true,"accounts":[
            {"account":"1801","role":"input-vat"},{"account":"1802","role":"input-vat"},
            {"account":"2901","role":"reverse-charge-output-vat","reverse_charge_scope":"non-eu-services","paired_input_account":"1801"}
          ]}
        ]}"#,
    )
    .unwrap();
    let domestic = expand_input("125.50", "4000", "1910", domestic_standard(), "2026-02-15");
    for version in ["company@v1", "company@v2"] {
        let error = vat_expand_with_role_table(&domestic, version, &ambiguous).unwrap_err();
        assert_eq!(error.error_code(), "VAT_STRUCTURE");
        assert!(error.to_string().contains("unambiguous domestic"));
    }

    for (net, contra) in [("1802", "1910"), ("4000", "2999")] {
        let input = expand_input(
            "100.00",
            net,
            contra,
            reverse_charge(ReverseChargeScope::EuServices),
            "2026-02-15",
        );
        let error = custom_expand(&input).unwrap_err();
        assert_eq!(error.error_code(), "VAT_STRUCTURE");
    }
}

#[test]
fn custom_wrong_scope_pair_and_corrupted_leg_fail_recompute() {
    let input = expand_input(
        "100.00",
        "4000",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-02-15",
    );
    let good = custom_expand_body(1, &input);

    let mut wrong_pair = serde_json::to_value(&good).unwrap();
    wrong_pair["postings"][3]["account"] = serde_json::json!("2902");
    let wrong_pair: VoucherBody = serde_json::from_value(wrong_pair).unwrap();
    assert!(test_verify(&wrong_pair).is_err());

    let mut corrupted = serde_json::to_value(&good).unwrap();
    corrupted["postings"][2]["amount"]["amount"] = serde_json::json!("30.00");
    corrupted["postings"][3]["amount"]["amount"] = serde_json::json!("30.00");
    let corrupted: VoucherBody = serde_json::from_value(corrupted).unwrap();
    assert!(test_verify(&corrupted).is_err());
}

#[test]
fn default_chart_reverse_charge_fails_loudly_instead_of_filing_zero() {
    let voucher = expand_body(
        1,
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-02-15",
        "2026-02-15",
    );
    let error = test_sum(&q1(), std::slice::from_ref(&voucher)).unwrap_err();
    assert_eq!(error.error_code(), "VAT_STRUCTURE");
    assert!(error.to_string().contains("does not assign that VAT role"));
    assert!(test_verify(&voucher).is_err());
}

#[test]
fn account_sum_reverse_charge_period_no_longer_files_zero() {
    // The regression: a reverse-charge (EU-services) period. `vat_expand` seals the
    // self-assessed 1798/2992 pair at 25,5 % on net 100,00 → 25,50 each. The legacy
    // tagged path is blind to the untagged legs (0,00); the account-sum reads them.
    let voucher = company_expand_body(
        1,
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-02-15",
        "2026-02-15",
    );

    let legacy = derive_vat_figures(&q1(), std::slice::from_ref(&voucher)).unwrap();
    assert_eq!(
        legacy.sales_vat.amount_str(),
        "0.00",
        "legacy is blind to VAT-A"
    );
    assert_eq!(legacy.purchase_vat.amount_str(), "0.00");

    let summed = test_sum(&q1(), std::slice::from_ref(&voucher)).unwrap();
    assert_eq!(
        summed.sales_vat.amount_str(),
        "25.50",
        "self-assessed output VAT"
    );
    assert_eq!(
        summed.purchase_vat.amount_str(),
        "25.50",
        "deductible input VAT"
    );

    // A pure reverse-charge period nets to zero but files real (non-zero) figures.
    let figures = test_derive(&q1(), std::slice::from_ref(&voucher)).unwrap();
    let filing = compute_vat_filing(&VatFilingInput {
        period: q1(),
        sales_vat: figures.sales_vat,
        purchase_vat: figures.purchase_vat,
    })
    .unwrap();
    assert_eq!(filing.sales_vat.amount_str(), "25.50");
    assert_eq!(filing.purchase_vat.amount_str(), "25.50");
    assert_eq!(filing.net_vat.amount_str(), "0.00");
}

#[test]
fn account_sum_domestic_input_period_files_the_deductible_vat() {
    // Pattern B: gross 125,50 at 25,5 % → deductible input VAT 25,50 on 1763.
    let voucher = expand_body(
        1,
        "125.50",
        "7660",
        "1910",
        domestic_standard(),
        "2026-02-15",
        "2026-02-15",
    );
    let summed = test_sum(&q1(), std::slice::from_ref(&voucher)).unwrap();
    assert_eq!(summed.purchase_vat.amount_str(), "25.50");
    assert_eq!(
        summed.sales_vat.amount_str(),
        "0.00",
        "no output leg in Pattern B"
    );

    // The filing is a deduction (refund direction): net = 0 − 25,50.
    let figures = test_derive(&q1(), std::slice::from_ref(&voucher)).unwrap();
    let filing = compute_vat_filing(&VatFilingInput {
        period: q1(),
        sales_vat: figures.sales_vat,
        purchase_vat: figures.purchase_vat,
    })
    .unwrap();
    assert_eq!(filing.net_vat.amount_str(), "-25.50");
}

#[test]
fn draft_deductible_vat_reads_explicit_input_vat_under_exact_chart() {
    let draft = vat_expand(&expand_input(
        "125.50",
        "7660",
        "1910",
        domestic_standard(),
        "2026-02-15",
    ))
    .unwrap();
    let actual = draft_deductible_vat(&draft, DEFAULT_CHART_VERSION).unwrap();
    assert_eq!(actual.amount_str(), "25.50");

    let err = draft_deductible_vat(&draft, "fi-general-2026@unknown").unwrap_err();
    assert_eq!(err.error_code(), "VAT_STRUCTURE");
}

#[test]
fn draft_deductible_vat_reads_supported_tagged_partial_path() {
    let draft = VoucherDraft::new(
        date("2026-02-15"),
        date("2026-02-15"),
        "fictional shared-use service".into(),
        vec![
            posting(
                "7690",
                "100.00",
                Side::Debit,
                Some("purchase-domestic-standard-deduct50"),
            ),
            posting("1910", "100.00", Side::Credit, None),
        ],
        None,
    )
    .unwrap();
    assert_eq!(
        draft_deductible_vat(&draft, DEFAULT_CHART_VERSION)
            .unwrap()
            .amount_str(),
        "12.75"
    );
}

#[test]
fn account_sum_reverse_charge_respects_the_rate_boundary() {
    // 24 % before 2024-09-01 → 24,00 each; the historical rate is frozen in the legs.
    let before = company_expand_body(
        1,
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2024-08-31",
        "2024-08-31",
    );
    let q3_2024 = PeriodId::parse("2024-Q3").unwrap();
    let summed = test_sum(&q3_2024, std::slice::from_ref(&before)).unwrap();
    assert_eq!(summed.sales_vat.amount_str(), "24.00");
    assert_eq!(summed.purchase_vat.amount_str(), "24.00");
}

#[test]
fn derive_filing_figures_combines_tagged_sales_with_vat_a_purchases() {
    // A realistic mixed period: a domestic sale still on the tagged base×rate path
    // (output VAT 2550 on 10000) plus a reverse-charge purchase as VAT-A legs
    // (self-assessed 25,50 each). Neither path double-counts the other.
    let tagged_sale = vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "3000",
        "10000.00",
        "sales-domestic-standard",
        "1910",
    );
    let rc_purchase = company_expand_body(
        2,
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-02-15",
        "2026-02-15",
    );
    let figures = test_derive(&q1(), &[tagged_sale, rc_purchase]).unwrap();
    assert_eq!(
        figures.sales_vat.amount_str(),
        "2575.50",
        "2550 tagged + 25,50 RC output"
    );
    assert_eq!(
        figures.purchase_vat.amount_str(),
        "25.50",
        "RC deductible input"
    );
}

#[test]
fn account_sum_excludes_an_unmarked_correction() {
    // A manual correction is not a generated VAT-A leg. Filing counts only the
    // generated accrual and leaves correction classification to an explicit flow.
    let accrual = company_expand_body(
        1,
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-02-15",
        "2026-02-15",
    );
    // Hand-build the mirror correction (Debit 2992 / Credit 1798, plus a squaring
    // pair) — an untagged reversal of the self-assessed VAT.
    let reversal = {
        let draft = VoucherDraft::new(
            date("2026-02-20"),
            date("2026-02-20"),
            "reverse the self-assessed VAT".into(),
            vec![
                Posting {
                    account: AccountId::new("1910").unwrap(),
                    amount: eur_money("100.00"),
                    side: Side::Debit,
                    vat_code: None,
                },
                Posting {
                    account: AccountId::new("4020").unwrap(),
                    amount: eur_money("100.00"),
                    side: Side::Credit,
                    vat_code: None,
                },
                Posting {
                    account: AccountId::new("2992").unwrap(),
                    amount: eur_money("25.50"),
                    side: Side::Debit,
                    vat_code: None,
                },
                Posting {
                    account: AccountId::new("1798").unwrap(),
                    amount: eur_money("25.50"),
                    side: Side::Credit,
                    vat_code: None,
                },
            ],
            None,
        )
        .unwrap();
        VoucherBody::from_draft(&draft, 2, BRIOX_CHART_VERSION.into()).unwrap()
    };
    let summed = test_sum(&q1(), &[accrual, reversal]).unwrap();
    assert_eq!(summed.sales_vat.amount_str(), "25.50");
    assert_eq!(summed.purchase_vat.amount_str(), "25.50");
}

#[test]
fn account_sum_only_counts_vouchers_in_the_period() {
    let in_q1 = company_expand_body(
        1,
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-02-15",
        "2026-02-15",
    );
    let in_q2 = company_expand_body(
        2,
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-05-15",
        "2026-05-15",
    );
    let summed = test_sum(&q1(), &[in_q1, in_q2]).unwrap();
    assert_eq!(
        summed.sales_vat.amount_str(),
        "25.50",
        "only the Q1 voucher"
    );
}

#[test]
fn account_sum_skips_a_tagged_voucher_so_the_legacy_path_owns_it() {
    // A legacy reverse-charge shape posts its deductible leg tagged. The account-sum
    // skips the whole voucher, so the legacy path owns it exactly once.
    let vouchers = reverse_charge_postings("100.00", "purchase-eu-services", "sales-eu-services");
    let draft = VoucherDraft::new(
        date("2026-02-15"),
        date("2026-02-15"),
        "legacy tagged reverse charge".into(),
        vouchers,
        None,
    )
    .unwrap();
    let body = VoucherBody::from_draft(&draft, 1, "fi-general-2026@0".into()).unwrap();
    // Account-sum contributes nothing (tagged voucher is skipped, not an error).
    let summed = test_sum(&q1(), std::slice::from_ref(&body)).unwrap();
    assert_eq!(summed.sales_vat.amount_str(), "0.00");
    assert_eq!(summed.purchase_vat.amount_str(), "0.00");
    // The combined projection equals the legacy path alone — counted exactly once.
    let combined = test_derive(&q1(), std::slice::from_ref(&body)).unwrap();
    let legacy = derive_vat_figures(&q1(), std::slice::from_ref(&body)).unwrap();
    assert_eq!(combined.sales_vat, legacy.sales_vat);
    assert_eq!(combined.purchase_vat, legacy.purchase_vat);
}

#[test]
fn account_sum_excludes_an_unmarked_sub_cent_vat_account_movement() {
    // Even an unusual manual VAT-account movement is outside filing unless it carries
    // a generated-leg marker.
    let draft = VoucherDraft::new(
        date("2026-02-15"),
        date("2026-02-15"),
        "sub-cent input VAT".into(),
        vec![
            Posting {
                account: AccountId::new("7690").unwrap(),
                amount: eur_money("100.00"),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("1763").unwrap(),
                amount: Money::parse("25.505", Currency::new("EUR").unwrap()).unwrap(),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("1910").unwrap(),
                amount: Money::parse("125.505", Currency::new("EUR").unwrap()).unwrap(),
                side: Side::Credit,
                vat_code: None,
            },
        ],
        None,
    )
    .unwrap();
    let body = VoucherBody::from_draft(&draft, 1, "fi-general-2026@0".into()).unwrap();
    let figures = test_sum(&q1(), &[body]).unwrap();
    assert_eq!(figures.purchase_vat.amount_str(), "0.00");
}

#[test]
fn vat_expand_refuses_a_vat_account_as_net_or_contra() {
    // A VAT-role account used as the net (or contra) leg would collide with the
    // expanded VAT legs — refuse it before the seal freezes the ambiguity.
    let err = vat_expand(&expand_input(
        "125.50",
        "1763",
        "1910",
        domestic_standard(),
        "2026-02-15",
    ))
    .unwrap_err();
    assert_eq!(err.error_code(), "VAT_STRUCTURE");
    // The legacy expander can still emit 2992 as a VAT leg. Until it is chart-aware,
    // keep it forbidden as a business contra leg even though it is not a default role.
    let err2 = vat_expand(&expand_input(
        "125.50",
        "7660",
        "2992",
        domestic_standard(),
        "2026-02-15",
    ))
    .unwrap_err();
    assert_eq!(err2.error_code(), "VAT_STRUCTURE");
}

// --- ADR-7 (VAT-A) §2 recompute cross-check ---------------------------------

#[test]
fn verify_reverse_charge_recomputes_to_the_sealed_legs() {
    let voucher = company_expand_body(
        1,
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-02-15",
        "2026-02-15",
    );
    let outcome = test_verify(&voucher).unwrap();
    match outcome {
        VatRecompute::Verified {
            treatment: VatTreatment::ReverseCharge { scope, .. },
        } => assert_eq!(scope, ReverseChargeScope::EuServices),
        other => panic!("expected a verified reverse charge, got {other:?}"),
    }
}

#[test]
fn verify_domestic_input_recomputes_to_the_sealed_legs() {
    let voucher = expand_body(
        1,
        "125.50",
        "7660",
        "1910",
        domestic_standard(),
        "2026-02-15",
        "2026-02-15",
    );
    let outcome = test_verify(&voucher).unwrap();
    assert!(matches!(
        outcome,
        VatRecompute::Verified {
            treatment: VatTreatment::DomesticInput { .. }
        }
    ));
}

#[test]
fn verify_full_lower_rate_is_not_masked_as_partial_higher_rate() {
    let voucher = expand_body(
        1,
        "113.50",
        "4000",
        "1910",
        VatTreatment::DomesticInput {
            rate_class: "domestic-reduced-1".into(),
        },
        "2026-02-15",
        "2026-02-15",
    );
    let VatRecompute::Verified {
        treatment: VatTreatment::DomesticInput { rate_class },
    } = test_verify(&voucher).unwrap()
    else {
        panic!("a full lower-rate treatment must win before partial candidates");
    };
    assert_eq!(
        resolve_vat_rate(&rate_class, date("2026-02-15")).unwrap(),
        "13.5".parse::<Decimal>().unwrap()
    );
}

#[test]
fn verify_zero_rounded_vat_preserves_the_exact_rate_class() {
    let treatment = VatTreatment::DomesticInput {
        rate_class: "domestic-reduced-1".into(),
    };
    let voucher = expand_body(
        1,
        "0.01",
        "4000",
        "1910",
        treatment.clone(),
        "2026-02-15",
        "2026-02-15",
    );
    assert!(voucher
        .vat_expansion()
        .unwrap()
        .generated_vat_legs()
        .is_empty());
    assert_eq!(
        test_verify(&voucher).unwrap(),
        VatRecompute::Verified { treatment }
    );
}

#[test]
fn verify_partial_domestic_uses_exact_sealed_treatment() {
    let voucher = expand_body(
        1,
        "125.50",
        "1220",
        "1910",
        domestic_half_deductible(),
        "2026-02-15",
        "2026-02-15",
    );
    assert_eq!(
        test_verify(&voucher).unwrap(),
        VatRecompute::Verified {
            treatment: domestic_half_deductible()
        }
    );
}

#[test]
fn verify_partial_reverse_charge_uses_exact_sealed_treatment() {
    let voucher = company_expand_body(
        1,
        "100.00",
        "1220",
        "1910",
        reverse_charge_half_deductible(ReverseChargeScope::EuServices),
        "2026-02-15",
        "2026-02-15",
    );
    assert_eq!(
        test_verify(&voucher).unwrap(),
        VatRecompute::Verified {
            treatment: reverse_charge_half_deductible(ReverseChargeScope::EuServices)
        }
    );
}

#[test]
fn partial_domestic_expansion_matches_legacy_derive_golden_vector() {
    // Boundary vector: gross 0,13 → full VAT 0,03 → 50 % share 0,02. Applying
    // 50 % before freezing full VAT would produce 0,01, so this also pins parity in
    // the required two-stage rounding order.
    let expanded = expand_body(
        1,
        "0.13",
        "1220",
        "1910",
        domestic_half_deductible(),
        "2026-02-15",
        "2026-02-15",
    );
    let legacy = vat_voucher(
        2,
        "2026-02-15",
        "2026-02-15",
        "1220",
        "0.10",
        "purchase-domestic-standard-deduct50",
        "1910",
    );
    let expanded_figures = test_sum(&q1(), &[expanded]).unwrap();
    let legacy_figures = derive_vat_figures(&q1(), &[legacy]).unwrap();
    assert_eq!(expanded_figures, legacy_figures);
    assert_eq!(expanded_figures.purchase_vat.amount_str(), "0.02");
}

#[test]
fn partial_reverse_charge_expansion_matches_legacy_derive_golden_vector() {
    let expanded = company_expand_body(
        1,
        "100.00",
        "1220",
        "1910",
        reverse_charge_half_deductible(ReverseChargeScope::EuServices),
        "2026-02-15",
        "2026-02-15",
    );
    // Legacy represents the same reverse charge as two tagged 100,00 bases: full
    // output VAT and a 50 % deductible input-VAT code.
    let legacy_draft = VoucherDraft::new(
        date("2026-02-15"),
        date("2026-02-15"),
        "legacy partial reverse charge".into(),
        vec![
            Posting {
                account: AccountId::new("1220").unwrap(),
                amount: eur_money("100.00"),
                side: Side::Debit,
                vat_code: Some(VatCode::new("purchase-domestic-standard-deduct50").unwrap()),
            },
            Posting {
                account: AccountId::new("2992").unwrap(),
                amount: eur_money("100.00"),
                side: Side::Credit,
                vat_code: Some(VatCode::new("sales-domestic-standard").unwrap()),
            },
        ],
        None,
    )
    .unwrap();
    let legacy = VoucherBody::from_draft(&legacy_draft, 2, BRIOX_CHART_VERSION.into()).unwrap();
    let expanded_figures = test_sum(&q1(), &[expanded]).unwrap();
    let legacy_figures = derive_vat_figures(&q1(), &[legacy]).unwrap();
    assert_eq!(expanded_figures.sales_vat, legacy_figures.sales_vat);
    assert_eq!(expanded_figures.purchase_vat, legacy_figures.purchase_vat);
    assert_eq!(expanded_figures.purchase_vat.amount_str(), "12.75");
    assert_eq!(expanded_figures.sales_vat.amount_str(), "25.50");
}

#[test]
fn verify_non_eu_reverse_charge_recomputes_via_the_1763_2982_pair() {
    // The non-EU variant reuses 1763 (input) with 2982 (output) — the presence of
    // the output leg is what distinguishes it from a domestic input.
    let voucher = company_expand_body(
        1,
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::NonEuServices),
        "2026-02-15",
        "2026-02-15",
    );
    let outcome = test_verify(&voucher).unwrap();
    assert!(matches!(
        outcome,
        VatRecompute::Verified {
            treatment: VatTreatment::ReverseCharge {
                scope: ReverseChargeScope::NonEuServices,
                ..
            }
        }
    ));
}

#[test]
fn verify_plain_entry_is_not_applicable() {
    let draft = VoucherDraft::new(
        date("2026-02-15"),
        date("2026-02-15"),
        "bank transfer".into(),
        vec![
            Posting {
                account: AccountId::new("1910").unwrap(),
                amount: eur_money("500.00"),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("1900").unwrap(),
                amount: eur_money("500.00"),
                side: Side::Credit,
                vat_code: None,
            },
        ],
        None,
    )
    .unwrap();
    let body = VoucherBody::from_draft(&draft, 1, "fi-general-2026@0".into()).unwrap();
    assert_eq!(test_verify(&body).unwrap(), VatRecompute::NotApplicable);
}

#[test]
fn verify_rejects_a_tampered_vat_amount() {
    // Keep the sealed provenance but corrupt both balanced VAT legs. Exact
    // recomputation must reject even if another rate could round to those cents.
    let good = company_expand_body(
        1,
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-02-15",
        "2026-02-15",
    );
    let mut tampered = serde_json::to_value(&good).unwrap();
    let postings = tampered["postings"].as_array_mut().unwrap();
    postings[2]["amount"]["amount"] = serde_json::json!("30.00");
    postings[3]["amount"]["amount"] = serde_json::json!("30.00");
    let tampered: VoucherBody = serde_json::from_value(tampered).unwrap();
    let err = test_verify(&tampered).unwrap_err();
    assert_eq!(err.error_code(), "VAT_STRUCTURE");
}

#[test]
fn verify_ignores_an_unmarked_output_account_movement() {
    let draft = VoucherDraft::new(
        date("2026-02-15"),
        date("2026-02-15"),
        "orphan output VAT".into(),
        vec![
            Posting {
                account: AccountId::new("4020").unwrap(),
                amount: eur_money("100.00"),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("1910").unwrap(),
                amount: eur_money("74.50"),
                side: Side::Credit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("2992").unwrap(),
                amount: eur_money("25.50"),
                side: Side::Credit,
                vat_code: None,
            },
        ],
        None,
    )
    .unwrap();
    let body = VoucherBody::from_draft(&draft, 1, BRIOX_CHART_VERSION.into()).unwrap();
    assert_eq!(test_verify(&body).unwrap(), VatRecompute::NotApplicable);
}

#[test]
fn verify_treats_a_reversal_as_not_applicable_not_malformed() {
    // A correction that reverses a self-assessed pair (VAT legs on the opposite
    // side) is not a forward expansion — the verifier reports NotApplicable, never
    // a misleading "malformed" error.
    let draft = VoucherDraft::new(
        date("2026-02-20"),
        date("2026-02-20"),
        "reverse the self-assessed VAT".into(),
        vec![
            Posting {
                account: AccountId::new("1910").unwrap(),
                amount: eur_money("100.00"),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("4020").unwrap(),
                amount: eur_money("100.00"),
                side: Side::Credit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("2992").unwrap(),
                amount: eur_money("25.50"),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("1798").unwrap(),
                amount: eur_money("25.50"),
                side: Side::Credit,
                vat_code: None,
            },
        ],
        None,
    )
    .unwrap();
    let body = VoucherBody::from_draft(&draft, 1, BRIOX_CHART_VERSION.into()).unwrap();
    assert_eq!(test_verify(&body).unwrap(), VatRecompute::NotApplicable);
}

// --- F25 role masterdata + loud-failure guards ------------------------------

/// One posting, optionally VAT-tagged (helper for the F25 guard tests).
fn posting(account: &str, amount: &str, side: Side, code: Option<&str>) -> Posting {
    Posting {
        account: AccountId::new(account).unwrap(),
        amount: eur_money(amount),
        side,
        vat_code: code.map(|c| VatCode::new(c).unwrap()),
    }
}

/// Freeze a set of (already-balanced) postings into a `VoucherBody` recorded
/// against `chart_version` — used to exercise the chart-keyed role resolution.
fn untagged_body(number: u64, chart_version: &str, postings: Vec<Posting>) -> VoucherBody {
    let draft = aggountant_model::VoucherDraft::new(
        date("2026-02-15"),
        date("2026-02-15"),
        "f25 test voucher".into(),
        postings,
        None,
    )
    .unwrap();
    VoucherBody::from_draft(&draft, number, chart_version.into()).unwrap()
}

#[test]
fn bundled_default_snapshot_resolves_by_its_real_content_hash() {
    let chart = vat_account_role_table()
        .resolve(DEFAULT_CHART_VERSION)
        .unwrap();
    assert_eq!(chart.role_of("1763"), Some(VatRoleTag::InputVat));
    assert_eq!(chart.role_of("2992"), None);
}

#[test]
fn bundled_active_default_selects_the_shared_service_pair_explicitly() {
    let chart = vat_account_role_table()
        .resolve(DEFAULT_CHART_VERSION)
        .unwrap();
    assert_eq!(chart.domestic_input_target(), Some("1763"));
    assert_eq!(
        chart.reverse_charge_target(ReverseChargeScope::EuServices),
        Some(("1763", "2939"))
    );
    assert_eq!(
        chart.reverse_charge_target(ReverseChargeScope::NonEuServices),
        Some(("1763", "2939"))
    );
    assert_eq!(chart.role_of("2939"), Some(VatRoleTag::TaggedOnly));
    assert_eq!(
        chart.generated_role_of("2939"),
        Some(GeneratedVatRole::Output)
    );
}

#[test]
fn default_chart_roles_only_name_accounts_in_the_default_chart() {
    // The bundled chart has domestic deductible/output accounts, but no dedicated
    // reverse-charge accounts. In particular, Briox's 1798/2992 pair must not leak in
    // as a default-chart assumption.
    let table = test_role_table();
    let chart = table.resolve("fi-general-2026@0").unwrap();
    assert_eq!(
        chart.role_of(ACCT_INPUT_VAT_DOMESTIC),
        Some(VatRoleTag::InputVat)
    );
    assert_eq!(chart.role_of("2939"), Some(VatRoleTag::TaggedOnly));
    for account in ["1797", "1798", "2982", "2991", "2992"] {
        assert_eq!(chart.role_of(account), None, "account {account}");
    }
}

#[test]
fn imported_briox_snapshot_owns_reverse_charge_pairing() {
    let table = test_role_table();
    let chart = table.resolve(BRIOX_CHART_VERSION).unwrap();
    assert_eq!(
        reverse_charge_pair(chart, "2992"),
        Some((ReverseChargeScope::EuServices, "1798"))
    );
    assert_eq!(
        chart.reverse_charge_target(ReverseChargeScope::EuServices),
        Some(("1798", "2992"))
    );
    assert_eq!(
        reverse_charge_pair(chart, "2991"),
        Some((ReverseChargeScope::EuGoods, "1797"))
    );
    assert_eq!(
        reverse_charge_pair(chart, "2982"),
        Some((ReverseChargeScope::NonEuServices, "1763"))
    );
}

#[test]
fn projection_excludes_unmarked_imported_vat_account_movements() {
    let json = r#"{
      "table":"imported",
      "charts":[{
        "chart_version":"company@custom",
        "active":true,
        "accounts":[
          {"account":"1801","role":"input-vat"},
          {"account":"2901","role":"reverse-charge-output-vat","reverse_charge_scope":"eu-services","paired_input_account":"1801"}
        ]
      }]
    }"#;
    let table = VatAccountRoleTable::from_json(json).unwrap();
    let body = untagged_body(
        1,
        "company@custom",
        vec![
            posting("4000", "100.00", Side::Debit, None),
            posting("1910", "100.00", Side::Credit, None),
            posting("1801", "25.50", Side::Debit, None),
            posting("2901", "25.50", Side::Credit, None),
        ],
    );
    let figures = sum_vat_account_figures_with_role_table(&q1(), &[body], &table).unwrap();
    assert_eq!(figures.purchase_vat.amount_str(), "0.00");
    assert_eq!(figures.sales_vat.amount_str(), "0.00");
}

#[test]
fn resolver_is_version_exact_and_refuses_unactivated_imports() {
    let table = test_role_table();
    assert!(table.resolve(BRIOX_CHART_VERSION).is_ok());
    assert!(table.resolve("example-briox-2026@another-version").is_err());
    let error = table.resolve("unreviewed-import@fixture").unwrap_err();
    assert_eq!(error.error_code(), "VAT_STRUCTURE");
    assert!(error.to_string().contains("not activated"));
}

#[test]
fn validator_refuses_active_roleless_snapshot_and_unpaired_output() {
    let roleless = VatAccountRoleTable {
        table: "import".into(),
        charts: vec![ChartVatRoles {
            chart_version: "company@one".into(),
            active: true,
            accounts: vec![],
            domestic_input_account: None,
            reverse_charge_targets: vec![],
            mistag_guards: vec![],
        }],
    };
    assert!(validate_vat_account_role_table(&roleless).is_err());

    let unpaired = VatAccountRoleTable {
        table: "import".into(),
        charts: vec![ChartVatRoles {
            chart_version: "company@two".into(),
            active: true,
            accounts: vec![ChartVatAccount {
                account: "2992".into(),
                role: VatRoleTag::ReverseChargeOutputVat,
                generated_role: None,
                reverse_charge_scope: None,
                paired_input_account: None,
            }],
            domestic_input_account: None,
            reverse_charge_targets: vec![],
            mistag_guards: vec![],
        }],
    };
    assert!(validate_vat_account_role_table(&unpaired).is_err());
}

#[test]
fn explicit_targets_require_an_explicit_domestic_input_target() {
    let error = VatAccountRoleTable::from_json(
        r#"{"table":"t","charts":[{"chart_version":"x@1","active":true,"accounts":[
          {"account":"1763","role":"input-vat"},
          {"account":"2939","role":"tagged-only","generated_role":"output"}
        ],"reverse_charge_targets":[
          {"scope":"non-eu-services","input_account":"1763","output_account":"2939"}
        ]}]}"#,
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("selects no domestic input account"));
}

#[test]
fn role_table_validator_rejects_an_unknown_dedicated_rate_class() {
    // A mistag guard whose dedicated_rate_class has no edition in the dated table
    // would silently never fire — reject it at load time.
    let bad = VatAccountRoleTable {
        table: "t".into(),
        charts: vec![ChartVatRoles {
            chart_version: "x@1".into(),
            active: false,
            accounts: vec![],
            domestic_input_account: None,
            reverse_charge_targets: vec![],
            mistag_guards: vec![MistagGuard {
                account: "7010".into(),
                dedicated_rate_class: "domestic-nonsense".into(),
            }],
        }],
    };
    assert!(validate_vat_account_role_table(&bad).is_err());
}

#[test]
fn role_table_validator_rejects_a_duplicated_account() {
    let bad = VatAccountRoleTable {
        table: "t".into(),
        charts: vec![ChartVatRoles {
            chart_version: "x@1".into(),
            active: true,
            accounts: vec![
                ChartVatAccount {
                    account: "1763".into(),
                    role: VatRoleTag::InputVat,
                    generated_role: None,
                    reverse_charge_scope: None,
                    paired_input_account: None,
                },
                ChartVatAccount {
                    account: "1763".into(),
                    role: VatRoleTag::InputVat,
                    generated_role: None,
                    reverse_charge_scope: None,
                    paired_input_account: None,
                },
            ],
            domestic_input_account: None,
            reverse_charge_targets: vec![],
            mistag_guards: vec![],
        }],
    };
    assert!(validate_vat_account_role_table(&bad).is_err());
}

#[test]
fn tagged_voucher_on_unknown_chart_cannot_bypass_activation() {
    let tagged = vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "3000",
        "100.00",
        "sales-domestic-standard",
        "1910",
    );
    let draft = VoucherDraft::new(
        tagged.document_date(),
        tagged.posting_date(),
        tagged.description().to_string(),
        tagged.postings().to_vec(),
        tagged.attachment_ref(),
    )
    .unwrap();
    let unknown = VoucherBody::from_draft(&draft, 1, "unknown@chart".into()).unwrap();
    let error = test_derive(&q1(), &[unknown]).unwrap_err();
    assert_eq!(error.error_code(), "VAT_STRUCTURE");
    assert!(error.to_string().contains("voucher 1"));
}

#[test]
fn account_sum_refuses_an_unknown_chart_instead_of_filing_zero() {
    // Guard A: a pure VAT-A voucher recorded against a chart the role masterdata
    // does not recognise must FAIL LOUDLY, not silently classify no VAT legs and
    // file 0,00 (the worst business outcome). Same untagged legs as a normal
    // reverse charge, only the chart is different.
    let input = expand_input(
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-02-15",
    );
    let draft = vat_expand(&input).unwrap();
    let body = VoucherBody::from_draft(&draft, 1, "acme-oy-2027@deadbeef".into()).unwrap();
    let err = test_sum(&q1(), std::slice::from_ref(&body)).unwrap_err();
    assert_eq!(err.error_code(), "VAT_STRUCTURE");
    // The whole filing (both projections) fails, not just this projection.
    assert!(test_derive(&q1(), std::slice::from_ref(&body)).is_err());
}

#[test]
fn account_sum_excludes_unmarked_movement_on_a_tagged_only_vat_account() {
    // A manual movement on a VAT account is not a generated VAT-A leg.
    let body = untagged_body(
        1,
        "fi-general-2026@0",
        vec![
            posting("2939", "100.00", Side::Debit, None),
            posting("1910", "100.00", Side::Credit, None),
        ],
    );
    let figures = test_sum(&q1(), std::slice::from_ref(&body)).unwrap();
    assert_eq!(figures.sales_vat.amount_str(), "0.00");
    assert_eq!(figures.purchase_vat.amount_str(), "0.00");
}

#[test]
fn mistag_refuses_generic_code_on_electricity_in_the_temp_window() {
    // Guard C regression: electricity (7010) tagged with the generic band
    // `purchase-domestic-standard` (24 %) during the temporary-measure window
    // (2022-12-01…2023-04-30, dedicated `domestic-electricity` = 10 %) silently
    // files the WRONG rate. Refuse it and force the dedicated class.
    let err = test_mistag(
        "fi-general-2026@0",
        date("2023-02-15"),
        &[posting(
            "7010",
            "100.00",
            Side::Debit,
            Some("purchase-domestic-standard"),
        )],
    )
    .unwrap_err();
    assert_eq!(err.error_code(), "VAT_STRUCTURE");
    assert!(err.to_string().contains("domestic-electricity"));

    // The dedicated class is accepted on the same date.
    assert!(test_mistag(
        "fi-general-2026@0",
        date("2023-02-15"),
        &[posting(
            "7010",
            "100.00",
            Side::Debit,
            Some("purchase-domestic-electricity"),
        )],
    )
    .is_ok());
}

#[test]
fn imported_role_table_drives_mistag_guard() {
    let table = VatAccountRoleTable::from_json(
        r#"{"table":"import","charts":[{"chart_version":"company@guarded","active":true,"accounts":[{"account":"1801","role":"input-vat"}],"mistag_guards":[{"account":"7101","dedicated_rate_class":"domestic-electricity"}]}]}"#,
    )
    .unwrap();
    let error = check_vat_rate_class_mistag_with_role_table(
        &table,
        "company@guarded",
        date("2023-02-15"),
        &[posting(
            "7101",
            "100.00",
            Side::Debit,
            Some("purchase-domestic-standard"),
        )],
    )
    .unwrap_err();
    assert_eq!(error.error_code(), "VAT_STRUCTURE");
}

#[test]
fn mistag_allows_generic_code_on_electricity_when_the_rate_coincides() {
    // Outside the temp windows electricity == standard (both 25,5 % from
    // 2024-09-01), so either spelling files the same euro — the guard must not
    // block an ordinary electricity booking.
    assert!(test_mistag(
        "fi-general-2026@0",
        date("2026-03-01"),
        &[posting(
            "7010",
            "100.00",
            Side::Debit,
            Some("purchase-domestic-standard"),
        )],
    )
    .is_ok());
}

#[test]
fn bundled_vat_role_masterdata_loads_and_validates() {
    // Force the OnceLock init (parse + validate) so `cargo test` catches malformed
    // bundled data rather than a first-use panic during a quarterly filing.
    let table = vat_account_role_table();
    assert!(table
        .charts
        .iter()
        .any(|c| c.active && chart_name(&c.chart_version) == DEFAULT_CHART));
}

#[test]
fn role_table_validator_rejects_an_invalid_account_id() {
    let bad = VatAccountRoleTable {
        table: "t".into(),
        charts: vec![ChartVatRoles {
            chart_version: format!("{DEFAULT_CHART}@bad"),
            active: true,
            accounts: vec![ChartVatAccount {
                account: "17 63".into(),
                role: VatRoleTag::InputVat,
                generated_role: None,
                reverse_charge_scope: None,
                paired_input_account: None,
            }],
            domestic_input_account: None,
            reverse_charge_targets: vec![],
            mistag_guards: vec![],
        }],
    };
    assert!(validate_vat_account_role_table(&bad).is_err());
}

#[test]
fn bundled_role_table_validator_requires_the_default_chart() {
    let bad = VatAccountRoleTable {
        table: "t".into(),
        charts: vec![ChartVatRoles {
            chart_version: "some-other-chart@1".into(),
            active: false,
            accounts: vec![],
            domestic_input_account: None,
            reverse_charge_targets: vec![],
            mistag_guards: vec![],
        }],
    };
    assert!(validate_vat_account_role_table(&bad).is_ok());
    assert!(validate_bundled_default(&bad).is_err());
}

#[test]
fn vat_expand_refuses_a_tagged_only_vat_account_as_net_or_contra() {
    // Regression: 2939 (domestic-output, tagged-only) must be refused as net/contra
    // at expansion time — not accepted only to be refused later by Guard B at
    // filing. Fail at the cheaper record-time boundary.
    let err = vat_expand(&expand_input(
        "100.00",
        "2939",
        "1910",
        domestic_standard(),
        "2026-02-15",
    ))
    .unwrap_err();
    assert_eq!(err.error_code(), "VAT_STRUCTURE");
}

#[test]
fn mistag_does_not_refuse_a_reverse_charge_or_eu_code_on_a_guarded_account() {
    // A structurally different treatment (EU reverse-charge acquisition) booked to
    // the electricity account is NOT a rate mistag and may be legitimate — the
    // guard must only police the generic *domestic bands*, not refuse this. In the
    // electricity temp window, eu-services (0%) diverges from electricity (10%); the
    // old percentage-only guard would have wrongly refused it.
    assert!(test_mistag(
        "fi-general-2026@0",
        date("2023-02-15"),
        &[posting(
            "7010",
            "100.00",
            Side::Debit,
            Some("purchase-eu-services"),
        )],
    )
    .is_ok());
}

#[test]
fn mistag_ignores_accounts_without_a_rule_and_unknown_charts() {
    // An account with no dedicated treatment, and any account on a chart with no
    // mistag masterdata, are not blocked (the loud unknown-chart refusal is at
    // filing time, Guard A).
    assert!(test_mistag(
        "fi-general-2026@0",
        date("2023-02-15"),
        &[posting(
            "7690",
            "100.00",
            Side::Debit,
            Some("purchase-domestic-standard"),
        )],
    )
    .is_ok());
    assert!(test_mistag(
        "acme-oy-2027@deadbeef",
        date("2023-02-15"),
        &[posting(
            "7010",
            "100.00",
            Side::Debit,
            Some("purchase-domestic-standard"),
        )],
    )
    .is_ok());
}

#[test]
fn account_sum_respects_an_explicit_month() {
    let july = PeriodId::parse("2026-07").unwrap();
    let july_voucher = company_expand_body(
        1,
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-07-31",
        "2026-07-31",
    );
    let august_voucher = company_expand_body(
        2,
        "200.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-08-01",
        "2026-08-01",
    );
    let figures = test_sum(&july, &[july_voucher, august_voucher]).unwrap();
    assert_eq!(figures.sales_vat.amount_str(), "25.50");
    assert_eq!(figures.purchase_vat.amount_str(), "25.50");
}

#[test]
fn domestic_tagged_sale_and_provenance_purchases_merge_once_per_tax_slot() {
    let sale = vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "3000",
        "100.00",
        "sales-domestic-standard",
        "1700",
    );
    let credit = vat_voucher_sided(
        2,
        "2026-02-15",
        "2026-02-15",
        "3000",
        "20.00",
        "sales-domestic-standard",
        Side::Debit,
        "1700",
    );
    let mut vouchers = vec![sale, credit];
    for (number, scope) in [
        (3, ReverseChargeScope::NonEuServices),
        (4, ReverseChargeScope::EuServices),
    ] {
        vouchers.push(shared_expand_body(
            number,
            &expand_input("10.00", "4000", "1910", reverse_charge(scope), "2026-02-15"),
        ));
    }
    let figures = test_derive(&q1(), &vouchers).unwrap();
    assert_eq!(figures.sales_vat.amount_str(), "25.50");
    assert_eq!(figures.purchase_vat.amount_str(), "5.10");
    let filing = compute_derived_vat_filing(&figures).unwrap();
    for (field, amount) in [
        (VatReturnField::Field301, "22.95"),
        (VatReturnField::Field306, "2.55"),
        (VatReturnField::Field313, "10.00"),
        (VatReturnField::Field307, "5.10"),
        (VatReturnField::Field308, "20.40"),
    ] {
        assert_eq!(filing.fields.get(field).unwrap().amount_str(), amount);
    }
    assert_eq!(filing.fields.iter().count(), 5);
    assert_eq!(
        filing
            .fields
            .get(VatReturnField::Field301)
            .unwrap()
            .amount()
            + filing
                .fields
                .get(VatReturnField::Field306)
                .unwrap()
                .amount(),
        filing.sales_vat.amount()
    );
}
