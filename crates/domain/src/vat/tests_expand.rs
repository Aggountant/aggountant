use super::*;
use aggountant_model::{GeneratedVatLeg, GeneratedVatRole};

pub(super) fn expand_input(
    amount: &str,
    net_account: &str,
    contra_account: &str,
    treatment: VatTreatment,
    document_date: &str,
) -> VatExpandInput {
    VatExpandInput {
        amount: eur_money(amount),
        net_account: AccountId::new(net_account).unwrap(),
        contra_account: AccountId::new(contra_account).unwrap(),
        treatment,
        document_date: date(document_date),
        posting_date: date(document_date),
        description: "test purchase".into(),
        attachment_ref: None,
    }
}

/// The unique posting on `account`/`side` (panics if absent — the assertion).
pub(super) fn leg_of<'a>(draft: &'a VoucherDraft, account: &str, side: Side) -> &'a Posting {
    let mut hits = draft
        .postings()
        .iter()
        .filter(|p| p.account.as_str() == account && p.side == side);
    let leg = hits.next().unwrap_or_else(|| {
        panic!("expected a {side:?} leg on account {account}");
    });
    assert!(
        hits.next().is_none(),
        "more than one {side:?} leg on {account}"
    );
    leg
}

pub(super) fn domestic_standard() -> VatTreatment {
    VatTreatment::DomesticInput {
        rate_class: "domestic-standard".into(),
    }
}

pub(super) fn reverse_charge(scope: ReverseChargeScope) -> VatTreatment {
    VatTreatment::ReverseCharge {
        rate_class: "domestic-standard".into(),
        scope,
    }
}

pub(super) fn half_deductible() -> VatDeductibility {
    VatDeductibility::new(
        1,
        2,
        VatDeductibilityProvenance::DocumentedUseAllocation {
            reference: "vehicle-logbook-2026".into(),
        },
    )
    .unwrap()
}

pub(super) fn domestic_half_deductible() -> VatTreatment {
    VatTreatment::DomesticInputPartial {
        rate_class: "domestic-standard".into(),
        deductibility: half_deductible(),
    }
}

pub(super) fn reverse_charge_half_deductible(scope: ReverseChargeScope) -> VatTreatment {
    VatTreatment::ReverseChargePartial {
        rate_class: "domestic-standard".into(),
        scope,
        deductibility: half_deductible(),
    }
}

#[test]
fn expansion_provenance_covers_treatments_and_zero_rounding() {
    let cases = [
        VatTreatment::NoVat,
        domestic_standard(),
        domestic_half_deductible(),
        reverse_charge(ReverseChargeScope::EuServices),
        reverse_charge(ReverseChargeScope::EuGoods),
        reverse_charge(ReverseChargeScope::NonEuServices),
        reverse_charge_half_deductible(ReverseChargeScope::EuServices),
        reverse_charge_half_deductible(ReverseChargeScope::EuGoods),
        reverse_charge_half_deductible(ReverseChargeScope::NonEuServices),
    ];
    for treatment in cases {
        let draft = vat_expand(&expand_input(
            "125.50",
            "7690",
            "1910",
            treatment.clone(),
            "2026-02-15",
        ))
        .unwrap();
        let draft_round_trip: VoucherDraft =
            serde_json::from_slice(&serde_json::to_vec(&draft).unwrap()).unwrap();
        assert_eq!(draft_round_trip.vat_expansion(), draft.vat_expansion());
        let expansion = draft
            .vat_expansion()
            .expect("every VAT-A expansion carries provenance");
        assert_eq!(expansion.version(), 1);
        assert_eq!(expansion.source_amount().amount_str(), "125.50");
        assert_eq!(expansion.net_account().as_str(), "7690");
        assert_eq!(expansion.contra_account().as_str(), "1910");
        assert_eq!(expansion.policy_set(), "fi.core.2026-01@stub");
        if matches!(treatment, VatTreatment::NoVat) {
            assert!(expansion.resolved_percent().is_none());
            assert!(expansion.generated_vat_legs().is_empty());
        } else {
            assert_eq!(expansion.resolved_percent(), Some("25.5"));
            let roles: Vec<_> = expansion
                .generated_vat_legs()
                .iter()
                .map(|leg| leg.role)
                .collect();
            let expected_markers = match &treatment {
                VatTreatment::DomesticInput { .. } | VatTreatment::DomesticInputPartial { .. } => {
                    vec![(1, GeneratedVatRole::Input, "1763")]
                }
                VatTreatment::ReverseCharge { scope, .. }
                | VatTreatment::ReverseChargePartial { scope, .. } => vec![
                    (2, GeneratedVatRole::Input, scope.input_account()),
                    (3, GeneratedVatRole::Output, scope.output_account()),
                ],
                VatTreatment::NoVat => unreachable!(),
            };
            assert_eq!(
                roles,
                expected_markers
                    .iter()
                    .map(|(_, role, _)| *role)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                expansion.generated_vat_legs(),
                expected_markers
                    .iter()
                    .map(|(posting_index, role, _)| GeneratedVatLeg {
                        posting_index: *posting_index,
                        role: *role
                    })
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
            for (posting_index, _, account) in expected_markers {
                assert_eq!(draft.postings()[posting_index].account.as_str(), account);
            }
            assert_eq!(
                expansion.vat_rates_hash().to_hex(),
                "e736ab82100707db6ab806df5b3acffa7285676b0b8401cc0cc45cc46afff4e8"
            );
            let treatment_json = serde_json::to_string(expansion.treatment()).unwrap();
            assert!(treatment_json.contains("domestic-standard"));
            if matches!(
                treatment,
                VatTreatment::ReverseCharge { .. } | VatTreatment::ReverseChargePartial { .. }
            ) {
                assert!(treatment_json.contains(match treatment {
                    VatTreatment::ReverseCharge { scope, .. }
                    | VatTreatment::ReverseChargePartial { scope, .. } => match scope {
                        ReverseChargeScope::EuServices => "eu-services",
                        ReverseChargeScope::EuGoods => "eu-goods",
                        ReverseChargeScope::NonEuServices => "non-eu-services",
                    },
                    _ => unreachable!(),
                }));
            }
            if matches!(
                treatment,
                VatTreatment::DomesticInputPartial { .. }
                    | VatTreatment::ReverseChargePartial { .. }
            ) {
                assert!(treatment_json.contains("\"numerator\":1"));
                assert!(treatment_json.contains("\"denominator\":2"));
                assert!(treatment_json.contains("vehicle-logbook-2026"));
            }
        }
        let body = VoucherBody::from_draft(&draft, 1, "chart@hash".into()).unwrap();
        assert_eq!(
            serde_json::from_slice::<VoucherBody>(&serde_json::to_vec(&body).unwrap())
                .unwrap()
                .vat_expansion(),
            draft.vat_expansion()
        );
    }
}

#[test]
fn expansion_provenance_retains_zero_rounded_marker_shapes() {
    let cases = [
        domestic_standard(),
        domestic_half_deductible(),
        reverse_charge(ReverseChargeScope::EuServices),
        reverse_charge_half_deductible(ReverseChargeScope::EuServices),
    ];
    for treatment in cases {
        let draft = vat_expand(&expand_input(
            "0.01",
            "7690",
            "1910",
            treatment,
            "2026-02-15",
        ))
        .unwrap();
        let expansion = draft.vat_expansion().unwrap();
        assert!(
            expansion.generated_vat_legs().is_empty(),
            "zero VAT has no physical marker"
        );
        let decoded_draft: VoucherDraft =
            serde_json::from_slice(&serde_json::to_vec(&draft).unwrap()).unwrap();
        let body = VoucherBody::from_draft(&draft, 1, "chart@hash".into()).unwrap();
        let decoded_body: VoucherBody =
            serde_json::from_slice(&serde_json::to_vec(&body).unwrap()).unwrap();
        assert_eq!(decoded_draft.vat_expansion(), Some(expansion));
        assert_eq!(decoded_body.vat_expansion(), Some(expansion));
    }
}

#[test]
fn expand_no_vat_is_a_plain_two_leg_entry() {
    // Pattern C: the whole amount books to the net account; no VAT account moves.
    let draft = vat_expand(&expand_input(
        "100.00",
        "7690",
        "1910",
        VatTreatment::NoVat,
        "2026-02-15",
    ))
    .unwrap();
    assert_eq!(draft.postings().len(), 2);
    assert_eq!(
        leg_of(&draft, "7690", Side::Debit).amount.amount_str(),
        "100.00"
    );
    assert_eq!(
        leg_of(&draft, "1910", Side::Credit).amount.amount_str(),
        "100.00"
    );
    // No leg carries a VAT code under Pattern C.
    assert!(draft.postings().iter().all(|p| p.vat_code.is_none()));
}

#[test]
fn expand_domestic_input_splits_gross_into_net_and_1763() {
    // Pattern B at 25,5 %: gross 125,50 → net 100,00 (7660 D) + input VAT 25,50
    // (1763 D), contra 125,50 (1910 K).
    let draft = vat_expand(&expand_input(
        "125.50",
        "7660",
        "1910",
        domestic_standard(),
        "2026-02-15",
    ))
    .unwrap();
    assert_eq!(draft.postings().len(), 3);
    assert_eq!(
        leg_of(&draft, "7660", Side::Debit).amount.amount_str(),
        "100.00"
    );
    assert_eq!(
        leg_of(&draft, "1763", Side::Debit).amount.amount_str(),
        "25.50"
    );
    assert_eq!(
        leg_of(&draft, "1910", Side::Credit).amount.amount_str(),
        "125.50"
    );
    // Every expanded leg is untagged (VAT-A: the euro on the account is the
    // authority; the treatment lives in the account pair, not a code) — so the
    // legacy tag-driven derive_vat_figures cannot half-read a VAT-A voucher.
    assert!(draft.postings().iter().all(|p| p.vat_code.is_none()));
}

#[test]
fn expand_leaves_every_leg_untagged_so_legacy_derivation_ignores_vat_a_vouchers() {
    // The whole point of untagged legs: derive_vat_figures (base×rate over TAGGED
    // postings) sees nothing on a VAT-A voucher, so it can neither double-count nor
    // (for a reverse charge) half-count. Domestic 0,12 gross at 25,5 % is the exact
    // cent case where a stamped net base (0,10) would have derived 0,03 while the
    // sealed explicit VAT leg is 0,02 — untagging removes that contradiction.
    let draft = vat_expand(&expand_input(
        "0.12",
        "7660",
        "1910",
        domestic_standard(),
        "2026-02-15",
    ))
    .unwrap();
    assert_eq!(
        leg_of(&draft, "1763", Side::Debit).amount.amount_str(),
        "0.02"
    );
    assert!(draft.postings().iter().all(|p| p.vat_code.is_none()));
    // Fold the sealed voucher into a period and derive: the tag-blind legacy path
    // reports 0,00 for both directions — it does not see the explicit VAT accounts.
    let body =
        VoucherBody::from_draft(&draft, 1, "fi-general-2026@0".into()).expect("body from draft");
    let derived = derive_vat_figures(&q1(), &[body]).unwrap();
    assert_eq!(derived.sales_vat.amount_str(), "0.00");
    assert_eq!(derived.purchase_vat.amount_str(), "0.00");
}

#[test]
fn expand_refuses_a_sub_cent_amount_rather_than_silently_rescaling() {
    // Source money is never silently altered: a >2dp intake amount is refused.
    let input = VatExpandInput {
        amount: Money::new("100.001".parse().unwrap(), Currency::new("EUR").unwrap()),
        net_account: AccountId::new("7660").unwrap(),
        contra_account: AccountId::new("1910").unwrap(),
        treatment: domestic_standard(),
        document_date: date("2026-02-15"),
        posting_date: date("2026-02-15"),
        description: "sub-cent".into(),
        attachment_ref: None,
    };
    let err = vat_expand(&input).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CURRENCY");
}

#[test]
fn expand_domestic_partial_deductibility_rounds_full_vat_before_the_share() {
    // Gross 0,12 at 25,5 % has full invoice VAT 0,02. Two thirds of that concrete
    // VAT rounds to 0,01; multiplying the unrounded theoretical VAT by 2/3 and only
    // then rounding would produce 0,02.
    let treatment = VatTreatment::DomesticInputPartial {
        rate_class: "domestic-standard".into(),
        deductibility: VatDeductibility::new(
            2,
            3,
            VatDeductibilityProvenance::DocumentedUseAllocation {
                reference: "mixed-use-allocation".into(),
            },
        )
        .unwrap(),
    };
    let draft = vat_expand(&expand_input(
        "0.12",
        "7660",
        "1910",
        treatment,
        "2026-02-15",
    ))
    .unwrap();
    assert_eq!(
        leg_of(&draft, "1763", Side::Debit).amount.amount_str(),
        "0.01"
    );
    assert_eq!(
        leg_of(&draft, "7660", Side::Debit).amount.amount_str(),
        "0.11"
    );
    assert_eq!(
        leg_of(&draft, "1910", Side::Credit).amount.amount_str(),
        "0.12"
    );
}

#[test]
fn expand_half_deductible_vehicle_books_domestic_remainder_to_cost() {
    // 50 % private-use vehicle: gross 125,50 → full VAT 25,50 → deductible 12,75.
    // The other 12,75 remains in vehicle cost: 112,75 + 12,75 = 125,50.
    let draft = vat_expand(&expand_input(
        "125.50",
        "1220",
        "1910",
        domestic_half_deductible(),
        "2026-02-15",
    ))
    .unwrap();
    assert_eq!(
        leg_of(&draft, "1220", Side::Debit).amount.amount_str(),
        "112.75"
    );
    assert_eq!(
        leg_of(&draft, "1763", Side::Debit).amount.amount_str(),
        "12.75"
    );
    assert_eq!(
        leg_of(&draft, "1910", Side::Credit).amount.amount_str(),
        "125.50"
    );
}

#[test]
fn expand_half_deductible_reverse_charge_keeps_full_output_vat_and_balances() {
    // Net 100,00 → full self-assessed output VAT 25,50, deductible input 12,75,
    // and the 12,75 remainder added to cost. Debits and credits both total 125,50.
    let draft = vat_expand(&expand_input(
        "100.00",
        "1220",
        "1910",
        reverse_charge_half_deductible(ReverseChargeScope::EuServices),
        "2026-02-15",
    ))
    .unwrap();
    assert_eq!(
        leg_of(&draft, "1220", Side::Debit).amount.amount_str(),
        "112.75"
    );
    assert_eq!(
        leg_of(&draft, "1798", Side::Debit).amount.amount_str(),
        "12.75"
    );
    assert_eq!(
        leg_of(&draft, "2992", Side::Credit).amount.amount_str(),
        "25.50"
    );
    assert_eq!(
        leg_of(&draft, "1910", Side::Credit).amount.amount_str(),
        "100.00"
    );
}

#[test]
fn expand_reverse_charge_eu_services_seals_the_1798_2992_pair() {
    // Pattern A, EU services, net 100 at 25,5 %: expense 4020 D 100, bank 1910 K
    // 100, self-assessed output 2992 K 25,50, deductible input 1798 D 25,50.
    let draft = vat_expand(&expand_input(
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2026-02-15",
    ))
    .unwrap();
    assert_eq!(draft.postings().len(), 4);
    assert_eq!(
        leg_of(&draft, "4020", Side::Debit).amount.amount_str(),
        "100.00"
    );
    assert_eq!(
        leg_of(&draft, "1910", Side::Credit).amount.amount_str(),
        "100.00"
    );
    assert_eq!(
        leg_of(&draft, "1798", Side::Debit).amount.amount_str(),
        "25.50"
    );
    assert_eq!(
        leg_of(&draft, "2992", Side::Credit).amount.amount_str(),
        "25.50"
    );
}

#[test]
fn expand_reverse_charge_eu_goods_seals_the_1797_2991_pair() {
    let draft = vat_expand(&expand_input(
        "100.00",
        "4135",
        "1910",
        reverse_charge(ReverseChargeScope::EuGoods),
        "2026-02-15",
    ))
    .unwrap();
    assert_eq!(
        leg_of(&draft, "1797", Side::Debit).amount.amount_str(),
        "25.50"
    );
    assert_eq!(
        leg_of(&draft, "2991", Side::Credit).amount.amount_str(),
        "25.50"
    );
}

#[test]
fn expand_reverse_charge_non_eu_seals_the_1763_2982_pair() {
    let draft = vat_expand(&expand_input(
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::NonEuServices),
        "2026-02-15",
    ))
    .unwrap();
    assert_eq!(
        leg_of(&draft, "1763", Side::Debit).amount.amount_str(),
        "25.50"
    );
    assert_eq!(
        leg_of(&draft, "2982", Side::Credit).amount.amount_str(),
        "25.50"
    );
}

#[test]
fn expand_resolves_the_rate_by_document_date_across_the_2024_09_01_boundary() {
    // THE golden boundary vector (ADR-7 §4): the same EU-services reverse charge
    // (net 100) expands to the *historical* rate — 24 % before 2024-09-01, 25,5 %
    // on/after it — because the rate is frozen from the document date, not read-time.
    let before = vat_expand(&expand_input(
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2024-08-31",
    ))
    .unwrap();
    assert_eq!(
        leg_of(&before, "1798", Side::Debit).amount.amount_str(),
        "24.00",
        "24 % before 2024-09-01"
    );
    assert_eq!(
        leg_of(&before, "2992", Side::Credit).amount.amount_str(),
        "24.00"
    );
    assert_eq!(
        before.vat_expansion().unwrap().resolved_percent(),
        Some("24.0")
    );

    let after = vat_expand(&expand_input(
        "100.00",
        "4020",
        "1910",
        reverse_charge(ReverseChargeScope::EuServices),
        "2024-09-01",
    ))
    .unwrap();
    assert_eq!(
        leg_of(&after, "1798", Side::Debit).amount.amount_str(),
        "25.50",
        "25,5 % on/after 2024-09-01"
    );
    assert_eq!(
        leg_of(&after, "2992", Side::Credit).amount.amount_str(),
        "25.50"
    );
    assert_eq!(
        after.vat_expansion().unwrap().resolved_percent(),
        Some("25.5")
    );
}

#[test]
fn expand_domestic_input_also_respects_the_rate_boundary() {
    // Pattern B across the boundary: gross 124,00 at 24 % → net 100,00 + VAT 24,00;
    // gross 125,50 at 25,5 % → net 100,00 + VAT 25,50.
    let before = vat_expand(&expand_input(
        "124.00",
        "7660",
        "1910",
        domestic_standard(),
        "2024-08-31",
    ))
    .unwrap();
    assert_eq!(
        leg_of(&before, "7660", Side::Debit).amount.amount_str(),
        "100.00"
    );
    assert_eq!(
        leg_of(&before, "1763", Side::Debit).amount.amount_str(),
        "24.00"
    );

    let after = vat_expand(&expand_input(
        "125.50",
        "7660",
        "1910",
        domestic_standard(),
        "2024-09-01",
    ))
    .unwrap();
    assert_eq!(
        leg_of(&after, "7660", Side::Debit).amount.amount_str(),
        "100.00"
    );
    assert_eq!(
        leg_of(&after, "1763", Side::Debit).amount.amount_str(),
        "25.50"
    );
}

#[test]
fn expand_is_deterministic_for_the_recompute_cross_check() {
    // ADR-7 §2: a verifier re-runs vat_expand and asserts recomputed == sealed.
    let mk = || {
        expand_input(
            "100.00",
            "4020",
            "1910",
            reverse_charge(ReverseChargeScope::EuServices),
            "2026-02-15",
        )
    };
    assert_eq!(vat_expand(&mk()).unwrap(), vat_expand(&mk()).unwrap());
}

#[test]
fn expand_refuses_a_non_positive_amount() {
    let err = vat_expand(&expand_input(
        "0.00",
        "7690",
        "1910",
        VatTreatment::NoVat,
        "2026-02-15",
    ))
    .unwrap_err();
    assert_eq!(err.error_code(), "VAT_CURRENCY");
}

#[test]
fn expand_refuses_a_foreign_currency_amount() {
    let sek = Money::parse("100.00", Currency::new("SEK").unwrap()).unwrap();
    let input = VatExpandInput {
        amount: sek,
        net_account: AccountId::new("7660").unwrap(),
        contra_account: AccountId::new("1910").unwrap(),
        treatment: domestic_standard(),
        document_date: date("2026-02-15"),
        posting_date: date("2026-02-15"),
        description: "foreign".into(),
        attachment_ref: None,
    };
    let err = vat_expand(&input).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CURRENCY");
}

#[test]
fn expand_refuses_an_unresolvable_rate_date() {
    // No dated edition covers 1990 → a loud VAT_CODE, never an extrapolated rate.
    let err = vat_expand(&expand_input(
        "125.50",
        "7660",
        "1910",
        domestic_standard(),
        "1990-01-01",
    ))
    .unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn expand_treatment_deserializes_from_the_assignment_json_shape() {
    // The import Assignment carries the treatment verbatim as an internally-tagged
    // object; confirm the wire shape the skill emits round-trips.
    let rc: VatTreatment = serde_json::from_str(
        r#"{"type":"reverse-charge","rate_class":"domestic-standard","scope":"eu-goods"}"#,
    )
    .unwrap();
    assert_eq!(rc, reverse_charge(ReverseChargeScope::EuGoods));
    let di: VatTreatment =
        serde_json::from_str(r#"{"type":"domestic-input","rate_class":"domestic-standard"}"#)
            .unwrap();
    assert_eq!(di, domestic_standard());
    let none: VatTreatment = serde_json::from_str(r#"{"type":"no-vat"}"#).unwrap();
    assert_eq!(none, VatTreatment::NoVat);

    let partial: VatTreatment = serde_json::from_str(
        r#"{"type":"domestic-input-partial","rate_class":"domestic-standard","deductibility":{"numerator":1,"denominator":2,"provenance":{"source":"documented-use-allocation","reference":"vehicle-logbook-2026"}}}"#,
    )
    .unwrap();
    assert_eq!(partial, domestic_half_deductible());
    let invalid = serde_json::from_str::<VatTreatment>(
        r#"{"type":"domestic-input-partial","rate_class":"domestic-standard","deductibility":{"numerator":2,"denominator":2,"provenance":{"source":"documented-use-allocation","reference":"vehicle-logbook-2026"}}}"#,
    );
    assert!(invalid.is_err(), "full/invalid fractions must be refused");
    let blank_reference = serde_json::from_str::<VatTreatment>(
        r#"{"type":"domestic-input-partial","rate_class":"domestic-standard","deductibility":{"numerator":1,"denominator":2,"provenance":{"source":"documented-use-allocation","reference":" "}}}"#,
    );
    assert!(blank_reference.is_err(), "blank provenance must be refused");
    let forged_reconstruction = serde_json::from_str::<VatTreatment>(
        r#"{"type":"domestic-input-partial","rate_class":"domestic-standard","deductibility":{"numerator":1,"denominator":2,"provenance":{"source":"reconstructed-from-vat-legs"}}}"#,
    );
    assert!(
        forged_reconstruction.is_err(),
        "callers cannot claim verifier-only provenance"
    );
}

// --- ADR-7 (VAT-A) §3 account-sum filing projection -------------------------

/// Seal a VAT-A voucher the way import does: expand the intake through
/// [`vat_expand`], then freeze the draft into a [`VoucherBody`] (untagged legs on
/// the VAT accounts, exactly what the account-sum projection must read).
pub(super) fn expand_body(
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
    VoucherBody::from_draft(&draft, number, "fi-general-2026@0".into()).unwrap()
}

#[test]
fn non_eu_service_with_an_unmappable_rate_class_refuses_before_draft() {
    let error = vat_expand(&expand_input(
        "100.00",
        "4000",
        "1910",
        VatTreatment::ReverseCharge {
            rate_class: "domestic-passenger-transport".into(),
            scope: ReverseChargeScope::NonEuServices,
        },
        "2026-02-15",
    ))
    .unwrap_err();
    assert_eq!(error.error_code(), "VAT_STRUCTURE");
    assert!(error.to_string().contains("no Finnish return rate slot"));
}

#[test]
fn treatment_input_accepts_non_eu_services_but_refuses_imported_goods() {
    let service: VatTreatment = serde_json::from_str(
        r#"{"type":"reverse-charge","rate_class":"domestic-standard","scope":"non-eu-services"}"#,
    )
    .unwrap();
    assert!(matches!(
        service,
        VatTreatment::ReverseCharge {
            scope: ReverseChargeScope::NonEuServices,
            ..
        }
    ));
    for scope in ["non-eu", "non-eu-goods", "imported-goods", "imports"] {
        let json = format!(
            r#"{{"type":"reverse-charge","rate_class":"domestic-standard","scope":"{scope}"}}"#
        );
        assert!(serde_json::from_str::<VatTreatment>(&json).is_err());
    }
}

#[test]
fn treatment_refuses_members_of_another_variant_for_every_variant() {
    // A member that belongs to another variant must be refused, not silently dropped:
    // a `deductibility` on `domestic-input` would otherwise seal a full deduction.
    const DEDUCTIBILITY: &str = r#""deductibility":{"numerator":1,"denominator":2,"provenance":{"source":"documented-use-allocation","reference":"vehicle-logbook-2026"}}"#;
    let valid = [
        (r#""type":"no-vat""#.to_string(), VatTreatment::NoVat),
        (
            r#""type":"domestic-input","rate_class":"domestic-standard""#.to_string(),
            domestic_standard(),
        ),
        (
            format!(
                r#""type":"domestic-input-partial","rate_class":"domestic-standard",{DEDUCTIBILITY}"#
            ),
            domestic_half_deductible(),
        ),
        (
            r#""type":"reverse-charge","rate_class":"domestic-standard","scope":"eu-goods""#
                .to_string(),
            reverse_charge(ReverseChargeScope::EuGoods),
        ),
        (
            format!(
                r#""type":"reverse-charge-partial","rate_class":"domestic-standard","scope":"eu-goods",{DEDUCTIBILITY}"#
            ),
            reverse_charge_half_deductible(ReverseChargeScope::EuGoods),
        ),
    ];
    for (members, expected) in &valid {
        // The `type` tag still selects the variant.
        let parsed: VatTreatment = serde_json::from_str(&format!("{{{members}}}"))
            .unwrap_or_else(|e| panic!("{members} must parse: {e}"));
        assert_eq!(&parsed, expected);
        // Serialization still emits exactly the members deserialization accepts.
        let round_trip: VatTreatment =
            serde_json::from_value(serde_json::to_value(expected).unwrap()).unwrap();
        assert_eq!(&round_trip, expected);
        for extra in [
            DEDUCTIBILITY.to_string(),
            r#""scope":"eu-services""#.to_string(),
            r#""rate_class":"domestic-reduced-a""#.to_string(),
            r#""unknown":1"#.to_string(),
        ] {
            let json = format!("{{{members},{extra}}}");
            assert!(
                serde_json::from_str::<VatTreatment>(&json).is_err(),
                "an unknown or duplicate member must be refused: {json}"
            );
        }
    }

    // The issue's reproduction: a 50 % deductibility misplaced on the full type.
    let misplaced =
        format!(r#"{{"type":"domestic-input","rate_class":"domestic-standard",{DEDUCTIBILITY}}}"#);
    let error = serde_json::from_str::<VatTreatment>(&misplaced).unwrap_err();
    assert!(
        error.to_string().contains("deductibility"),
        "the refusal names the member: {error}"
    );
}

#[test]
fn every_rate_class_is_either_a_purchase_class_or_a_zero_rated_sale_class() {
    // A class added to the dated table must be classified before a purchase can
    // use it; a class in both lists would be admitted and refused at once.
    for class in vat_rate_classes() {
        let purchase = PURCHASE_RATE_CLASSES.contains(&class);
        let sale = ZERO_RATED_SALE_RATE_CLASSES.contains(&class);
        assert!(
            purchase != sale,
            "rate class {class:?} must be in exactly one of PURCHASE_RATE_CLASSES and ZERO_RATED_SALE_RATE_CLASSES"
        );
    }
    for class in PURCHASE_RATE_CLASSES
        .iter()
        .chain(ZERO_RATED_SALE_RATE_CLASSES.iter())
    {
        assert!(
            vat_rate_classes().contains(class),
            "{class:?} is not in the dated table"
        );
    }
}

fn purchase_treatments(rate_class: &str) -> Vec<VatTreatment> {
    let mut treatments = vec![
        VatTreatment::DomesticInput {
            rate_class: rate_class.into(),
        },
        VatTreatment::DomesticInputPartial {
            rate_class: rate_class.into(),
            deductibility: half_deductible(),
        },
    ];
    for scope in [
        ReverseChargeScope::EuGoods,
        ReverseChargeScope::EuServices,
        ReverseChargeScope::NonEuServices,
    ] {
        treatments.push(VatTreatment::ReverseCharge {
            rate_class: rate_class.into(),
            scope,
        });
        treatments.push(VatTreatment::ReverseChargePartial {
            rate_class: rate_class.into(),
            scope,
            deductibility: half_deductible(),
        });
    }
    treatments
}

#[test]
fn purchase_treatment_with_a_zero_rated_sale_class_is_refused_before_a_draft() {
    // The issue's reproduction: domestic-input + eu-goods resolved to 0 % and would
    // have sealed an EU acquisition as a domestic purchase without VAT legs.
    for class in ZERO_RATED_SALE_RATE_CLASSES {
        for treatment in purchase_treatments(class) {
            if treatment == exempt_eu_goods() {
                continue; // the one admitted pair, pinned below
            }
            let input = expand_input("10.00", "7690", "1910", treatment.clone(), "2026-07-15");
            for error in [
                vat_expand(&input).unwrap_err(),
                vat_expand_with_role_table(&input, DEFAULT_TEST_CHART, vat_account_role_table())
                    .unwrap_err(),
            ] {
                assert_eq!(
                    error.error_code(),
                    "VAT_TREATMENT_CLASS_MISMATCH",
                    "{treatment:?}: {error}"
                );
                assert!(error.to_string().contains(class), "{error}");
            }
        }
    }
}

const DEFAULT_TEST_CHART: &str =
    "fi-general-2026@dd5d3f339cebfbc670f55cbd0942c775904c58523d9b07ad8eb48b589c79b63b";

#[test]
fn treatment_class_refusal_names_the_treatment_that_fits() {
    let message = |treatment: VatTreatment| {
        vat_expand(&expand_input(
            "10.00",
            "7690",
            "1910",
            treatment,
            "2026-07-15",
        ))
        .unwrap_err()
        .to_string()
    };

    let eu_goods = message(VatTreatment::DomesticInput {
        rate_class: "eu-goods".into(),
    });
    assert!(
        eu_goods.contains(
            r#"{"type":"reverse-charge","rate_class":"domestic-standard","scope":"eu-goods"}"#
        ),
        "{eu_goods}"
    );
    let eu_services = message(VatTreatment::DomesticInputPartial {
        rate_class: "eu-services".into(),
        deductibility: half_deductible(),
    });
    assert!(
        eu_services.contains(
            r#"{"type":"reverse-charge-partial","rate_class":"domestic-standard","scope":"eu-services"} plus the same deductibility member"#
        ),
        "{eu_services}"
    );
    let zero = message(VatTreatment::DomesticInput {
        rate_class: "zero-rated".into(),
    });
    assert!(zero.contains(r#"{"type":"no-vat"}"#), "{zero}");
    assert!(!eu_goods.contains("deductibility"), "{eu_goods}");
    let class_as_scope = message(VatTreatment::ReverseCharge {
        rate_class: "eu-services".into(),
        scope: ReverseChargeScope::EuServices,
    });
    assert!(
        class_as_scope.contains(
            r#"{"type":"reverse-charge","rate_class":"domestic-standard","scope":"eu-services"}"#
        ),
        "{class_as_scope}"
    );
    // An exempt intra-Community goods acquisition is still reported in the return, so
    // every goods refusal names its one booking and never advises no-vat; an exempt
    // EU service is not reported, so its refusal does.
    let goods_class_as_scope = message(VatTreatment::ReverseCharge {
        rate_class: "eu-goods".into(),
        scope: ReverseChargeScope::EuGoods,
    });
    let zero_partial_goods = message(VatTreatment::ReverseChargePartial {
        rate_class: "zero-rated".into(),
        scope: ReverseChargeScope::EuGoods,
        deductibility: half_deductible(),
    });
    for goods in [&eu_goods, &zero, &goods_class_as_scope, &zero_partial_goods] {
        assert!(goods.contains(EXEMPT_EU_GOODS_ACQUISITION), "{goods}");
    }
    assert!(
        zero_partial_goods.contains("no tax to deduct") && !zero_partial_goods.contains("no-vat"),
        "{zero_partial_goods}"
    );
    assert!(
        !goods_class_as_scope.contains("no-vat"),
        "{goods_class_as_scope}"
    );
    assert!(
        !eu_services.contains(EXEMPT_EU_GOODS_ACQUISITION),
        "{eu_services}"
    );
    let zero_non_eu = message(VatTreatment::ReverseCharge {
        rate_class: "zero-rated".into(),
        scope: ReverseChargeScope::NonEuServices,
    });
    assert!(
        zero_non_eu.contains(r#"{"type":"no-vat"}"#)
            && !zero_non_eu.contains(EXEMPT_EU_GOODS_ACQUISITION),
        "{zero_non_eu}"
    );
    let zero_reverse_services = message(VatTreatment::ReverseCharge {
        rate_class: "zero-rated".into(),
        scope: ReverseChargeScope::EuServices,
    });
    assert!(
        zero_reverse_services.contains(r#"{"type":"no-vat"}"#)
            && zero_reverse_services.contains("not reported"),
        "{zero_reverse_services}"
    );
    let zero_partial_services = message(VatTreatment::ReverseChargePartial {
        rate_class: "zero-rated".into(),
        scope: ReverseChargeScope::EuServices,
        deductibility: half_deductible(),
    });
    assert!(
        zero_partial_services.contains(r#"{"type":"no-vat"} with no other member"#),
        "{zero_partial_services}"
    );
    assert!(
        zero.contains("intra-Community acquisition of goods"),
        "{zero}"
    );
}

#[test]
fn purchase_treatments_keep_every_domestic_class_including_a_dated_zero_rate() {
    // Domestic passenger transport was 0 % from January to April 2023. The class
    // names a domestic supply, so domestic-input still expands (with no VAT leg).
    let draft = vat_expand(&expand_input(
        "50.00",
        "7690",
        "1910",
        VatTreatment::DomesticInput {
            rate_class: "domestic-passenger-transport".into(),
        },
        "2023-02-15",
    ))
    .unwrap();
    assert!(draft
        .vat_expansion()
        .unwrap()
        .generated_vat_legs()
        .is_empty());
    assert_eq!(
        draft.vat_expansion().unwrap().resolved_percent(),
        Some("0.0")
    );

    for class in PURCHASE_RATE_CLASSES {
        for treatment in purchase_treatments(class) {
            let non_eu_slotless = matches!(
                &treatment,
                VatTreatment::ReverseCharge {
                    scope: ReverseChargeScope::NonEuServices,
                    ..
                } | VatTreatment::ReverseChargePartial {
                    scope: ReverseChargeScope::NonEuServices,
                    ..
                }
            ) && vat_rate_class_field(class).is_err();
            let result = vat_expand(&expand_input(
                "100.00",
                "4000",
                "1910",
                treatment.clone(),
                "2026-02-15",
            ));
            if non_eu_slotless {
                // Unchanged: non-EU service tax fits only the three generic slots.
                assert_eq!(result.unwrap_err().error_code(), "VAT_STRUCTURE");
            } else {
                result.unwrap_or_else(|e| panic!("{treatment:?} must expand: {e}"));
            }
        }
    }
}

fn exempt_eu_goods() -> VatTreatment {
    VatTreatment::ReverseCharge {
        rate_class: "zero-rated".into(),
        scope: ReverseChargeScope::EuGoods,
    }
}

#[test]
fn exempt_eu_goods_acquisition_is_one_documented_treatment_with_no_vat_legs() {
    // The constant the refusals quote is the treatment the expansion admits.
    assert_eq!(
        serde_json::from_str::<VatTreatment>(EXEMPT_EU_GOODS_ACQUISITION).unwrap(),
        exempt_eu_goods()
    );
    let input = expand_input("480.00", "4000", "1910", exempt_eu_goods(), "2026-07-15");
    for draft in [
        vat_expand(&input).unwrap(),
        vat_expand_with_role_table(&input, DEFAULT_TEST_CHART, vat_account_role_table()).unwrap(),
    ] {
        let expansion = draft.vat_expansion().unwrap();
        assert!(expansion.generated_vat_legs().is_empty());
        assert_eq!(expansion.resolved_percent(), Some("0.0"));
        assert_eq!(
            expansion.treatment(),
            &VatExpansionTreatment::ReverseCharge {
                rate_class: "zero-rated".into(),
                scope: VatExpansionScope::EuGoods,
            }
        );
        // The net amount goes to the net account in full; no VAT account moves.
        let legs: Vec<_> = draft
            .postings()
            .iter()
            .map(|p| {
                (
                    p.account.as_str().to_string(),
                    p.side,
                    p.amount.amount_str(),
                )
            })
            .collect();
        assert_eq!(
            legs,
            vec![
                ("4000".to_string(), Side::Debit, "480.00".to_string()),
                ("1910".to_string(), Side::Credit, "480.00".to_string()),
            ]
        );
    }
}
