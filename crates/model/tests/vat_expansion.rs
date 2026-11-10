use aggountant_model::*;

fn eur(s: &str) -> Money {
    Money::parse(s, Currency::new("EUR").unwrap()).unwrap()
}
fn posting(account: &str, amount: &str, side: Side) -> Posting {
    Posting {
        account: AccountId::new(account).unwrap(),
        amount: eur(amount),
        side,
        vat_code: None,
    }
}
fn expansion() -> VatExpansion {
    VatExpansion::new(
        1,
        VatExpansionTreatment::DomesticInputPartial {
            rate_class: "domestic-standard".into(),
            deductibility: VatExpansionDeductibility::new(
                2,
                4,
                VatExpansionDeductibilityProvenance::DocumentedUseAllocation {
                    reference: "logbook".into(),
                },
            )
            .unwrap(),
        },
        eur("125.50"),
        AccountId::new("7660").unwrap(),
        AccountId::new("1910").unwrap(),
        Some("25.5".into()),
        "fi.core.2026-01@stub".into(),
        Hash::from_bytes([3; 32]),
        vec![GeneratedVatLeg {
            posting_index: 1,
            role: GeneratedVatRole::Input,
        }],
    )
    .unwrap()
}

#[test]
fn provenance_round_trips_and_rejects_bad_metadata() {
    let date = Date::parse("2026-02-15").unwrap();
    let draft = VoucherDraft::new(
        date,
        date,
        "VAT purchase".into(),
        vec![
            posting("7660", "100.00", Side::Debit),
            posting("1763", "25.50", Side::Debit),
            posting("1910", "125.50", Side::Credit),
        ],
        None,
    )
    .unwrap()
    .with_vat_expansion(expansion())
    .unwrap();
    let body = VoucherBody::from_draft(&draft, 1, "chart@hash".into()).unwrap();
    assert_eq!(body.vat_expansion(), draft.vat_expansion());
    let treatment_json = serde_json::to_string(draft.vat_expansion().unwrap().treatment()).unwrap();
    assert!(treatment_json.contains("\"numerator\":1"));
    assert!(treatment_json.contains("\"denominator\":2"));
    assert!(treatment_json.contains("logbook"));
    let bytes = serde_json::to_vec(&body).unwrap();
    assert_eq!(serde_json::from_slice::<VoucherBody>(&bytes).unwrap(), body);
    let bad_percent = String::from_utf8(bytes)
        .unwrap()
        .replace("\"25.5\"", "\"025.5\"");
    assert!(serde_json::from_str::<VoucherBody>(&bad_percent).is_err());
    let duplicate = VatExpansion::new(
        1,
        VatExpansionTreatment::DomesticInput {
            rate_class: "domestic-standard".into(),
        },
        eur("1.00"),
        AccountId::new("7660").unwrap(),
        AccountId::new("1910").unwrap(),
        Some("25.5".into()),
        "policy".into(),
        Hash::zero(),
        vec![
            GeneratedVatLeg {
                posting_index: 1,
                role: GeneratedVatRole::Input,
            },
            GeneratedVatLeg {
                posting_index: 1,
                role: GeneratedVatRole::Input,
            },
        ],
    );
    assert!(
        duplicate.is_err(),
        "constructor rejects duplicate marker indices"
    );
    let output_marker = r#"{"version":1,"treatment":{"type":"domestic-input","rate_class":"domestic-standard"},"source_amount":{"amount":"100.00","currency":"EUR"},"net_account":"7660","contra_account":"1910","resolved_percent":"25.5","policy_set":"policy","vat_rates_hash":"0000000000000000000000000000000000000000000000000000000000000000","generated_vat_legs":[{"posting_index":1,"role":"output"}]}"#;
    assert!(serde_json::from_str::<VatExpansion>(output_marker).is_err());
    let scale_three = VatExpansion::new(
        1,
        VatExpansionTreatment::DomesticInput {
            rate_class: "domestic-standard".into(),
        },
        Money::new("100.000".parse().unwrap(), Currency::new("EUR").unwrap()),
        AccountId::new("7660").unwrap(),
        AccountId::new("1910").unwrap(),
        Some("25.5".into()),
        "policy".into(),
        Hash::zero(),
        vec![],
    )
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<VatExpansion>(&serde_json::to_vec(&scale_three).unwrap())
            .unwrap()
            .source_amount()
            .amount_str(),
        "100.000",
        "numerically exact cents retain their source scale"
    );
}

#[test]
fn marker_shapes_and_metadata_are_rejected_by_constructor_and_serde() {
    let treatment = VatExpansionTreatment::ReverseCharge {
        rate_class: "domestic-standard".into(),
        scope: VatExpansionScope::EuServices,
    };
    let make = |treatment, legs| {
        VatExpansion::new(
            1,
            treatment,
            eur("100.00"),
            AccountId::new("4020").unwrap(),
            AccountId::new("1910").unwrap(),
            Some("25.5".into()),
            "policy".into(),
            Hash::zero(),
            legs,
        )
    };
    let input = GeneratedVatLeg {
        posting_index: 2,
        role: GeneratedVatRole::Input,
    };
    let output = GeneratedVatLeg {
        posting_index: 3,
        role: GeneratedVatRole::Output,
    };
    assert!(make(treatment.clone(), vec![]).is_ok());
    assert!(make(treatment.clone(), vec![input.clone(), output.clone()]).is_ok());
    assert!(make(treatment.clone(), vec![input.clone()]).is_err());
    assert!(make(treatment.clone(), vec![output.clone()]).is_err());
    assert!(make(
        treatment.clone(),
        vec![
            input.clone(),
            GeneratedVatLeg {
                posting_index: 4,
                role: GeneratedVatRole::Input
            },
            output.clone()
        ]
    )
    .is_err());

    let partial = VatExpansionTreatment::ReverseChargePartial {
        rate_class: "domestic-standard".into(),
        scope: VatExpansionScope::EuServices,
        deductibility: VatExpansionDeductibility::new(
            1,
            2,
            VatExpansionDeductibilityProvenance::DocumentedUseAllocation {
                reference: "allocation".into(),
            },
        )
        .unwrap(),
    };
    assert!(make(partial.clone(), vec![]).is_ok());
    assert!(make(partial.clone(), vec![output.clone()]).is_ok());
    assert!(make(partial.clone(), vec![input.clone(), output.clone()]).is_ok());
    assert!(make(partial, vec![input.clone()]).is_err());
    assert!(make(
        VatExpansionTreatment::DomesticInput {
            rate_class: "domestic-standard".into()
        },
        vec![]
    )
    .is_ok());
    assert!(make(
        VatExpansionTreatment::DomesticInput {
            rate_class: "domestic-standard".into()
        },
        vec![input.clone()]
    )
    .is_ok());
    assert!(make(
        VatExpansionTreatment::DomesticInput {
            rate_class: "domestic-standard".into()
        },
        vec![output.clone()]
    )
    .is_err());

    let valid = make(treatment.clone(), vec![input, output]).unwrap();
    let json = serde_json::to_string(&valid).unwrap();
    for (from, to) in [
        ("\"version\":1", "\"version\":2"),
        ("domestic-standard", " domestic-standard"),
        ("\"25.5\"", "\"025.5\""),
        ("\"policy\"", "\" policy\""),
        ("\"EUR\"", "\"USD\""),
        ("\"100.00\"", "\"100.001\""),
        ("\"100.00\"", "\"0.00\""),
        ("\"100.00\"", "\"-1.00\""),
        ("\"role\":\"input\"", "\"role\":\"output\""),
    ] {
        assert!(
            serde_json::from_str::<VatExpansion>(&json.replacen(from, to, 1)).is_err(),
            "serde must reject {from} -> {to}"
        );
    }
    let body = VoucherBody::from_draft(
        &VoucherDraft::new(
            Date::parse("2026-02-15").unwrap(),
            Date::parse("2026-02-15").unwrap(),
            "VAT RC".into(),
            vec![
                posting("4020", "100.00", Side::Debit),
                posting("1910", "100.00", Side::Credit),
                posting("1798", "25.50", Side::Debit),
                posting("2992", "25.50", Side::Credit),
            ],
            None,
        )
        .unwrap()
        .with_vat_expansion(
            make(
                treatment.clone(),
                vec![
                    GeneratedVatLeg {
                        posting_index: 2,
                        role: GeneratedVatRole::Input,
                    },
                    GeneratedVatLeg {
                        posting_index: 3,
                        role: GeneratedVatRole::Output,
                    },
                ],
            )
            .unwrap(),
        )
        .unwrap(),
        1,
        "chart@hash".into(),
    )
    .unwrap();
    let body_json = serde_json::to_string(&body).unwrap();
    assert!(serde_json::from_str::<VoucherBody>(&body_json.replacen(
        "\"posting_index\":2",
        "\"posting_index\":99",
        1
    ))
    .is_err());
    let wrong_sides = VoucherDraft::new(
        Date::parse("2026-02-15").unwrap(),
        Date::parse("2026-02-15").unwrap(),
        "VAT RC wrong sides".into(),
        vec![
            posting("4020", "100.00", Side::Debit),
            posting("1910", "100.00", Side::Credit),
            posting("1798", "25.50", Side::Credit),
            posting("2992", "25.50", Side::Debit),
        ],
        None,
    )
    .unwrap();
    let mut tagged_postings = vec![
        posting("7660", "100.00", Side::Debit),
        posting("1763", "25.50", Side::Debit),
        posting("1910", "125.50", Side::Credit),
    ];
    tagged_postings[0].vat_code = Some(VatCode::new("purchase-domestic-standard").unwrap());
    let tagged = VoucherDraft::new(
        Date::parse("2026-02-15").unwrap(),
        Date::parse("2026-02-15").unwrap(),
        "tagged and expanded".into(),
        tagged_postings,
        None,
    )
    .unwrap();
    assert!(tagged.with_vat_expansion(expansion()).is_err());

    assert!(
        wrong_sides
            .with_vat_expansion(
                make(
                    treatment.clone(),
                    vec![
                        GeneratedVatLeg {
                            posting_index: 2,
                            role: GeneratedVatRole::Input
                        },
                        GeneratedVatLeg {
                            posting_index: 3,
                            role: GeneratedVatRole::Output
                        }
                    ]
                )
                .unwrap()
            )
            .is_err(),
        "balanced VAT markers on the wrong side are rejected"
    );

    let bad_fraction = json.replace(
        "\"type\":\"reverse-charge\"",
        "\"type\":\"reverse-charge-partial\",\"deductibility\":{\"numerator\":2,\"denominator\":2,\"provenance\":{\"source\":\"documented-use-allocation\",\"reference\":\"allocation\"}}",
    );
    assert!(serde_json::from_str::<VatExpansion>(&bad_fraction).is_err());

    assert!(VatExpansion::new(
        1,
        VatExpansionTreatment::DomesticInput {
            rate_class: "domestic-standard".into()
        },
        eur("0.00"),
        AccountId::new("4020").unwrap(),
        AccountId::new("1910").unwrap(),
        Some("25.5".into()),
        "policy".into(),
        Hash::zero(),
        vec![]
    )
    .is_err());
    assert!(VatExpansion::new(
        1,
        VatExpansionTreatment::DomesticInput {
            rate_class: "domestic-standard".into()
        },
        Money::new("-1.00".parse().unwrap(), Currency::new("EUR").unwrap()),
        AccountId::new("4020").unwrap(),
        AccountId::new("1910").unwrap(),
        Some("25.5".into()),
        "policy".into(),
        Hash::zero(),
        vec![]
    )
    .is_err());
    assert!(VatExpansion::new(
        1,
        VatExpansionTreatment::NoVat,
        eur("100.00"),
        AccountId::new("4020").unwrap(),
        AccountId::new("1910").unwrap(),
        Some("25.5".into()),
        "policy".into(),
        Hash::zero(),
        vec![]
    )
    .is_err());
    assert!(VatExpansion::new(
        1,
        VatExpansionTreatment::NoVat,
        eur("100.00"),
        AccountId::new("4020").unwrap(),
        AccountId::new("1910").unwrap(),
        None,
        "policy".into(),
        Hash::zero(),
        vec![GeneratedVatLeg {
            posting_index: 0,
            role: GeneratedVatRole::Input
        }]
    )
    .is_err());

    let sek_draft = VoucherDraft::new(
        Date::parse("2026-02-15").unwrap(),
        Date::parse("2026-02-15").unwrap(),
        "SEK".into(),
        vec![
            Posting {
                account: AccountId::new("4020").unwrap(),
                amount: Money::parse("100.00", Currency::new("SEK").unwrap()).unwrap(),
                side: Side::Debit,
                vat_code: None,
            },
            Posting {
                account: AccountId::new("1910").unwrap(),
                amount: Money::parse("100.00", Currency::new("SEK").unwrap()).unwrap(),
                side: Side::Credit,
                vat_code: None,
            },
        ],
        None,
    )
    .unwrap();
    let no_leg = VatExpansion::new(
        1,
        VatExpansionTreatment::NoVat,
        eur("100.00"),
        AccountId::new("4020").unwrap(),
        AccountId::new("1910").unwrap(),
        None,
        "policy".into(),
        Hash::zero(),
        vec![],
    )
    .unwrap();
    assert!(
        sek_draft
            .clone()
            .with_vat_expansion(no_leg.clone())
            .is_err(),
        "a no-marker draft checks source currency before marker validation"
    );
    let no_leg_json = serde_json::to_string(&no_leg).unwrap();
    assert!(serde_json::from_str::<VatExpansion>(&no_leg_json.replacen(
        "\"resolved_percent\":null",
        "\"resolved_percent\":\"25.5\"",
        1
    ))
    .is_err());
    let mut no_leg_value = serde_json::to_value(&no_leg).unwrap();
    no_leg_value.as_object_mut().unwrap().insert(
        "generated_vat_legs".into(),
        serde_json::json!([{"posting_index": 0, "role": "input"}]),
    );
    assert!(serde_json::from_value::<VatExpansion>(no_leg_value).is_err());
    let mut body_value =
        serde_json::to_value(VoucherBody::from_draft(&sek_draft, 1, "chart@hash".into()).unwrap())
            .unwrap();
    body_value.as_object_mut().unwrap().insert(
        "vat_expansion".into(),
        serde_json::to_value(&no_leg).unwrap(),
    );
    assert!(
        serde_json::from_value::<VoucherBody>(body_value).is_err(),
        "body serde checks currency even with no VAT markers"
    );
}

#[test]
fn non_eu_service_scope_has_an_explicit_wire_name() {
    let treatment: VatExpansionTreatment = serde_json::from_str(
        r#"{"type":"reverse-charge","rate_class":"domestic-standard","scope":"non-eu-services"}"#,
    )
    .unwrap();
    assert!(matches!(
        treatment,
        VatExpansionTreatment::ReverseCharge {
            scope: VatExpansionScope::NonEuServices,
            ..
        }
    ));
    assert_eq!(
        serde_json::to_string(&treatment).unwrap(),
        r#"{"type":"reverse-charge","rate_class":"domestic-standard","scope":"non-eu-services"}"#
    );
}

#[test]
fn imported_goods_cannot_select_the_non_eu_service_treatment() {
    for scope in ["non-eu", "non-eu-goods", "imported-goods", "imports"] {
        let json = format!(
            r#"{{"type":"reverse-charge","rate_class":"domestic-standard","scope":"{scope}"}}"#
        );
        assert!(serde_json::from_str::<VatExpansionTreatment>(&json).is_err());
    }
}
