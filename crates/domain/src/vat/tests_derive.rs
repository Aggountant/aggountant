use super::*;

pub(super) fn date(s: &str) -> Date {
    Date::parse(s).unwrap()
}

/// A balanced two-posting voucher whose `code`-tagged line (on `tagged_side`)
/// carries `base` as the VAT base; the other line squares the entry. The
/// **normal** invoice side is sales→Credit, purchase→Debit; flipping the side
/// models a credit note / return. `document_date` drives the rate; `posting_date`
/// drives the period.
#[allow(clippy::too_many_arguments)]
pub(super) fn vat_voucher_sided(
    number: u64,
    document_date: &str,
    posting_date: &str,
    tagged_account: &str,
    base: &str,
    code: &str,
    tagged_side: Side,
    counter_account: &str,
) -> VoucherBody {
    let counter_side = match tagged_side {
        Side::Debit => Side::Credit,
        Side::Credit => Side::Debit,
    };
    let tagged = Posting {
        account: AccountId::new(tagged_account).unwrap(),
        amount: eur_money(base),
        side: tagged_side,
        vat_code: Some(VatCode::new(code).unwrap()),
    };
    let counter = Posting {
        account: AccountId::new(counter_account).unwrap(),
        amount: eur_money(base),
        side: counter_side,
        vat_code: None,
    };
    let draft = aggountant_model::VoucherDraft::new(
        date(document_date),
        date(posting_date),
        "test voucher".into(),
        vec![tagged, counter],
        None,
    )
    .unwrap();
    VoucherBody::from_draft(&draft, number, "fi-general-2026@0".into()).unwrap()
}

/// A normal invoice: the tagged VAT line sits on its accrual side (sales→Credit,
/// purchase→Debit), so its VAT contributes positively.
pub(super) fn vat_voucher(
    number: u64,
    document_date: &str,
    posting_date: &str,
    tagged_account: &str,
    base: &str,
    code: &str,
    counter_account: &str,
) -> VoucherBody {
    let side = if code.starts_with("sales-") {
        Side::Credit
    } else {
        Side::Debit
    };
    vat_voucher_sided(
        number,
        document_date,
        posting_date,
        tagged_account,
        base,
        code,
        side,
        counter_account,
    )
}

#[test]
fn public_tax_bearing_query_uses_canonical_parsing_and_dated_rates() {
    let taxable = VatCode::new("purchase-domestic-standard").unwrap();
    assert!(vat_code_is_tax_bearing(&taxable, date("2024-08-31")).unwrap());
    assert!(vat_code_is_tax_bearing(&taxable, date("2024-09-01")).unwrap());

    let zero = VatCode::new("purchase-zero-rated").unwrap();
    assert!(!vat_code_is_tax_bearing(&zero, date("2020-01-01")).unwrap());

    let unknown = VatCode::new("purchase-not-a-rate-class").unwrap();
    let error = vat_code_is_tax_bearing(&unknown, date("2026-01-01")).unwrap_err();
    assert_eq!(error.error_code(), "VAT_CODE");
    assert!(error.to_string().contains("not-a-rate-class"));
}

#[test]
fn derives_sales_and_purchase_vat_at_the_standard_rate() {
    // 25,5 % (in force from 2024-09-01): 10000 base → 2550 output;
    // 4000 base → 1020 deductible.
    let vouchers = vec![
        vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "3000",
            "10000.00",
            "sales-domestic-standard",
            "1910",
        ),
        vat_voucher(
            2,
            "2026-02-15",
            "2026-02-15",
            "4000",
            "4000.00",
            "purchase-domestic-standard",
            "1910",
        ),
    ];
    let d = derive_vat_figures(&q1(), &vouchers).unwrap();
    assert_eq!(d.sales_vat.amount_str(), "2550.00");
    assert_eq!(d.purchase_vat.amount_str(), "1020.00");
}

#[test]
fn rate_is_resolved_by_document_date_across_the_boundary() {
    // 1000 base at the historical 24 % (document_date before 2024-09-01) → 240.
    let vouchers = vec![vat_voucher(
        1,
        "2024-06-30",
        "2024-06-30",
        "3000",
        "1000.00",
        "sales-domestic-standard",
        "1910",
    )];
    let q2_2024 = PeriodId::parse("2024-Q2").unwrap();
    let d = derive_vat_figures(&q2_2024, &vouchers).unwrap();
    assert_eq!(d.sales_vat.amount_str(), "240.00", "24 % before 2024-09-01");
}

#[test]
fn only_vouchers_in_the_period_are_aggregated() {
    let vouchers = vec![
        // Q1 2026 — counted.
        vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "3000",
            "1000.00",
            "sales-domestic-standard",
            "1910",
        ),
        // Q2 2026 — excluded by posting_date.
        vat_voucher(
            2,
            "2026-05-15",
            "2026-05-15",
            "3000",
            "1000.00",
            "sales-domestic-standard",
            "1910",
        ),
    ];
    let d = derive_vat_figures(&q1(), &vouchers).unwrap();
    assert_eq!(d.sales_vat.amount_str(), "255.00");
}

#[test]
fn an_empty_period_derives_to_zero() {
    let d = derive_vat_figures(&q1(), &[]).unwrap();
    assert_eq!(d.sales_vat.amount_str(), "0.00");
    assert_eq!(d.purchase_vat.amount_str(), "0.00");
}

#[test]
fn a_code_without_a_direction_prefix_is_refused() {
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "3000",
        "1000.00",
        "domestic-standard",
        "1910",
    )];
    let err = derive_vat_figures(&q1(), &vouchers).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn an_unknown_rate_class_is_refused() {
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "3000",
        "1000.00",
        "sales-mystery-rate",
        "1910",
    )];
    let err = derive_vat_figures(&q1(), &vouchers).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn untagged_postings_contribute_nothing() {
    // A voucher with no vat_code anywhere derives to zero (bank transfer etc.).
    let plain = aggountant_model::VoucherDraft::new(
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
    let body = VoucherBody::from_draft(&plain, 1, "fi-general-2026@0".into()).unwrap();
    let d = derive_vat_figures(&q1(), &[body]).unwrap();
    assert_eq!(d.sales_vat.amount_str(), "0.00");
    assert_eq!(d.purchase_vat.amount_str(), "0.00");
}

#[test]
fn a_sales_credit_note_reduces_output_vat() {
    // Two sales of 10000 (→ 2550 each) and one 4000 credit note (debit to sales,
    // → −1020): output VAT = 2550 + 2550 − 1020 = 4080.
    let vouchers = vec![
        vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "3000",
            "10000.00",
            "sales-domestic-standard",
            "1910",
        ),
        vat_voucher(
            2,
            "2026-02-15",
            "2026-02-15",
            "3000",
            "10000.00",
            "sales-domestic-standard",
            "1910",
        ),
        vat_voucher_sided(
            3,
            "2026-02-15",
            "2026-02-15",
            "3000",
            "4000.00",
            "sales-domestic-standard",
            Side::Debit,
            "1910",
        ),
    ];
    let d = derive_vat_figures(&q1(), &vouchers).unwrap();
    assert_eq!(d.sales_vat.amount_str(), "4080.00");
}

#[test]
fn a_purchase_return_reduces_input_vat() {
    // A 4000 purchase (→ 1020) and a 1000 return (credit to expense, → −255):
    // deductible input VAT = 1020 − 255 = 765.
    let vouchers = vec![
        vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "4000",
            "4000.00",
            "purchase-domestic-standard",
            "1910",
        ),
        vat_voucher_sided(
            2,
            "2026-02-15",
            "2026-02-15",
            "4000",
            "1000.00",
            "purchase-domestic-standard",
            Side::Credit,
            "1910",
        ),
    ];
    let d = derive_vat_figures(&q1(), &vouchers).unwrap();
    assert_eq!(d.purchase_vat.amount_str(), "765.00");
}

/// A self-assessed EU reverse-charge voucher (AVL 9 §): one balanced voucher
/// booking both the output VAT (myynnin ALV, on the credit line, `sales-`) and
/// the equal deductible input VAT (vähennettävä ALV, on the debit line,
/// `purchase-`) for the same acquisition base. `document_date` drives the rate;
/// `posting_date` drives the period.
pub(super) fn reverse_charge_voucher(
    number: u64,
    base: &str,
    input_code: &str,
    output_code: &str,
) -> VoucherBody {
    let deductible_input = Posting {
        account: AccountId::new("1763").unwrap(),
        amount: eur_money(base),
        side: Side::Debit,
        vat_code: Some(VatCode::new(input_code).unwrap()),
    };
    let self_assessed_output = Posting {
        account: AccountId::new("2939").unwrap(),
        amount: eur_money(base),
        side: Side::Credit,
        vat_code: Some(VatCode::new(output_code).unwrap()),
    };
    let draft = aggountant_model::VoucherDraft::new(
        date("2026-02-15"),
        date("2026-02-15"),
        "EU reverse-charge acquisition".into(),
        vec![deductible_input, self_assessed_output],
        None,
    )
    .unwrap();
    VoucherBody::from_draft(&draft, number, "fi-general-2026@0".into()).unwrap()
}

#[test]
fn derives_reverse_charge_output_and_input_into_separate_totals() {
    // The D7 direction case (issue `vat-d7-direction-aware`). A reverse-charge
    // voucher's self-assessed output and equal deductible input must land in
    // *separate* totals (each 10000 × 25,5 % = 2550), NOT be netted into a
    // single figure that a swap could hide. The old netting-based D7 check
    // compared |Σ vat| against max(Σdebits, Σcredits) and saw only the zero net;
    // deriving by direction from the code metadata keeps the two sides distinct.
    let d = derive_vat_figures(
        &q1(),
        &[reverse_charge_voucher(
            1,
            "10000.00",
            "purchase-domestic-standard",
            "sales-domestic-standard",
        )],
    )
    .unwrap();
    assert_eq!(
        d.sales_vat.amount_str(),
        "2550.00",
        "self-assessed output VAT"
    );
    assert_eq!(
        d.purchase_vat.amount_str(),
        "2550.00",
        "deductible input VAT"
    );
}

#[test]
fn swapping_reverse_charge_direction_changes_the_derived_figures() {
    // The direction error the old netting D7 check could not catch: tag the
    // output line as `purchase-` and the deductible input line as `sales-`.
    // Under a netting check both vouchers net to zero and are indistinguishable;
    // deriving by direction makes the swap observable in the per-direction totals.
    //
    // The **net is identical** either way (0 = 0) — so a hypothetical net-only
    // cross-check would still miss this. It is the *per-direction* figures that
    // move, which is why `file_vat.rs` must (and does) compare `sales_vat` and
    // `purchase_vat` independently. The lock-in of that per-direction comparison
    // is a process-layer test tracked as a follow-up (assessment F1).
    let correct = derive_vat_figures(
        &q1(),
        &[reverse_charge_voucher(
            1,
            "10000.00",
            "purchase-domestic-standard",
            "sales-domestic-standard",
        )],
    )
    .unwrap();
    let swapped = derive_vat_figures(
        &q1(),
        &[reverse_charge_voucher(
            1,
            "10000.00",
            "sales-domestic-standard",
            "purchase-domestic-standard",
        )],
    )
    .unwrap();
    // Pin the correct booking here too, so this test stands on its own rather
    // than borrowing the expected tuple from its sibling.
    assert_eq!(
        (
            correct.sales_vat.amount_str(),
            correct.purchase_vat.amount_str()
        ),
        ("2550.00".to_string(), "2550.00".to_string()),
        "correct reverse-charge booking: output and input each +2550"
    );
    // The swap is observable: the per-direction tuple differs from the correct
    // one (a net-only check would not see this — net is 0 in both).
    assert_ne!(
        (
            correct.sales_vat.amount_str(),
            correct.purchase_vat.amount_str()
        ),
        (
            swapped.sales_vat.amount_str(),
            swapped.purchase_vat.amount_str()
        ),
        "a direction swap must move the per-direction figures, not net to an \
         indistinguishable pass"
    );
    // (−2550, −2550) is NOT a valid filing — it is the *malformed* booking made
    // observably wrong. A negative per-direction total against the agent's
    // correct (+2550, +2550) is exactly what the file-vat cross-check rejects
    // (VAT_FIGURES_MISMATCH). Asserted only to characterise the sign flip, never
    // as a desirable output.
    assert_eq!(swapped.sales_vat.amount_str(), "-2550.00");
    assert_eq!(swapped.purchase_vat.amount_str(), "-2550.00");
}

/// The two VAT postings of a reverse-charge acquisition (rate-free), for the
/// record-time [`check_voucher_vat_structure`] tests: a deductible-input line
/// (Debit) tagged `input_code` and a self-assessed-output line (Credit) tagged
/// `output_code`, both on the same `base`.
pub(super) fn reverse_charge_postings(
    base: &str,
    input_code: &str,
    output_code: &str,
) -> Vec<Posting> {
    vec![
        Posting {
            account: AccountId::new("1763").unwrap(),
            amount: eur_money(base),
            side: Side::Debit,
            vat_code: Some(VatCode::new(input_code).unwrap()),
        },
        Posting {
            account: AccountId::new("2939").unwrap(),
            amount: eur_money(base),
            side: Side::Credit,
            vat_code: Some(VatCode::new(output_code).unwrap()),
        },
    ]
}

#[test]
fn same_direction_reverse_charge_both_purchase_is_refused() {
    // F4 — the same-direction cancellation hole. Both VAT lines tagged
    // `purchase-`: the deductible input (Debit, +base) and the mis-tagged
    // self-assessed output (Credit, −base) cancel to zero *within* the purchase
    // direction. Were it sealed, the period aggregator would sum it to (0, 0) —
    // a plausible zero that passes the file-vat cross-check when the agent also
    // supplies 0, silently dropping the self-assessed output VAT. The record-time
    // structural guard refuses it (`VAT_STRUCTURE`) before it can be sealed.
    let err = check_voucher_vat_structure(&reverse_charge_postings(
        "10000.00",
        "purchase-domestic-standard",
        "purchase-domestic-standard",
    ))
    .unwrap_err();
    assert_eq!(err.error_code(), "VAT_STRUCTURE");
    // The message names the offending direction and the cancelled gross base, so
    // the agent can find and re-tag the malformed booking.
    assert!(err.to_string().contains("purchase-"), "got {err}");
    assert!(err.to_string().contains("20000.00"), "got {err}");
}

#[test]
fn same_direction_reverse_charge_both_sales_is_refused() {
    // The mirror malformation: both lines tagged `sales-`, so the sales
    // direction cancels to zero instead of purchase. Equally caught.
    let err = check_voucher_vat_structure(&reverse_charge_postings(
        "10000.00",
        "sales-domestic-standard",
        "sales-domestic-standard",
    ))
    .unwrap_err();
    assert_eq!(err.error_code(), "VAT_STRUCTURE");
    assert!(err.to_string().contains("sales-"), "got {err}");
}

#[test]
fn a_correct_reverse_charge_draft_passes_the_structural_guard() {
    // The well-formed booking (output `sales-` on Credit, deductible `purchase-`
    // on Debit) has each direction with a single +base contribution — no
    // cancellation — so it must pass unrefused.
    check_voucher_vat_structure(&reverse_charge_postings(
        "10000.00",
        "purchase-domestic-standard",
        "sales-domestic-standard",
    ))
    .unwrap();
}

#[test]
fn an_in_voucher_partial_correction_is_not_flagged() {
    // The guard flags *exact* cancellation only — a partial in-voucher
    // correction (a 4000 purchase base, +4000, plus a 1000 same-voucher
    // reversal, −1000) nets to a nonzero 3000 base and must pass, so a genuine
    // partial correction is never false-flagged.
    let postings = vec![
        Posting {
            account: AccountId::new("4000").unwrap(),
            amount: eur_money("4000.00"),
            side: Side::Debit,
            vat_code: Some(VatCode::new("purchase-domestic-standard").unwrap()),
        },
        Posting {
            account: AccountId::new("4000").unwrap(),
            amount: eur_money("1000.00"),
            side: Side::Credit,
            vat_code: Some(VatCode::new("purchase-domestic-standard").unwrap()),
        },
        Posting {
            account: AccountId::new("1910").unwrap(),
            amount: eur_money("3000.00"),
            side: Side::Credit,
            vat_code: None,
        },
    ];
    check_voucher_vat_structure(&postings).unwrap();
}

#[test]
fn vat_is_rounded_per_posting_not_per_period() {
    // Two 10.10 bases at 25,5 %: 10.10 × 0.255 = 2.5755 → round to 2.58 each.
    // Per-posting: 2.58 + 2.58 = 5.16. (Per-period would be 5.151 → 5.15.)
    let vouchers = vec![
        vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "3000",
            "10.10",
            "sales-domestic-standard",
            "1910",
        ),
        vat_voucher(
            2,
            "2026-02-15",
            "2026-02-15",
            "3000",
            "10.10",
            "sales-domestic-standard",
            "1910",
        ),
    ];
    let d = derive_vat_figures(&q1(), &vouchers).unwrap();
    assert_eq!(d.sales_vat.amount_str(), "5.16");
}

#[test]
fn the_25_5_boundary_is_exact() {
    // 2024-08-31 is the last 24 % day; 2024-09-01 the first 25,5 % day.
    let before = vat_voucher(
        1,
        "2024-08-31",
        "2024-08-31",
        "3000",
        "1000.00",
        "sales-domestic-standard",
        "1910",
    );
    let after = vat_voucher(
        2,
        "2024-09-01",
        "2024-09-01",
        "3000",
        "1000.00",
        "sales-domestic-standard",
        "1910",
    );
    let q3 = PeriodId::parse("2024-Q3").unwrap();
    assert_eq!(
        derive_vat_figures(&q3, &[before])
            .unwrap()
            .sales_vat
            .amount_str(),
        "240.00"
    );
    assert_eq!(
        derive_vat_figures(&q3, &[after])
            .unwrap()
            .sales_vat
            .amount_str(),
        "255.00"
    );
}

// --- Full FI rate history across the 2020–2026 backtest window -----------
//
// Golden vectors for every source-verified statutory boundary in the window
// (issue `vat-policy-full-rate-history`; sources cited in
// `issues/vat-policy-full-rate-history/rate-history.md`). Each crosses a real
// boundary — the same transaction before/after must select the correct
// *historical* rate by `document_date`, never the current one. The standard-rate
// 24 %→25,5 % boundary is covered by [`the_25_5_boundary_is_exact`] above; these
// add the reduced-rate reclassifications and the two temporary measures.

/// Assert the derived VAT euro of a single `base`-EUR purchase tagged `code` on
/// `document_date` — a one-line golden vector over the dated rate table.
pub(super) fn purchase_vat_on(code: &str, document_date: &str, base: &str) -> String {
    let v = vat_voucher(1, document_date, document_date, "7300", base, code, "1910");
    let period = PeriodId::containing(date(document_date));
    derive_vat_figures(&period, &[v])
        .unwrap()
        .purchase_vat
        .amount_str()
}

#[test]
fn passenger_transport_temporary_zero_rate_2023_is_exact() {
    // AVL 85 a §; temporary 0 % 1.1.2023–30.4.2023 (laki 1265/2022). Before the
    // window and after it the reduced 10 % applies; inside it the fare is 0 %.
    let code = "purchase-domestic-passenger-transport";
    assert_eq!(purchase_vat_on(code, "2022-12-31", "1000.00"), "100.00");
    assert_eq!(purchase_vat_on(code, "2023-01-01", "1000.00"), "0.00");
    assert_eq!(purchase_vat_on(code, "2023-04-30", "1000.00"), "0.00");
    assert_eq!(purchase_vat_on(code, "2023-05-01", "1000.00"), "100.00");
}

#[test]
fn passenger_transport_reduced_rate_reclassifications_2025_2026() {
    // Reclassified 10 %→14 % on 1.1.2025 (moved to the reduced-1 band), then the
    // reduced band 14 %→13,5 % on 1.1.2026. A fare crossing each year-boundary
    // must pick the rate in force on its document date.
    let code = "purchase-domestic-passenger-transport";
    assert_eq!(purchase_vat_on(code, "2024-12-31", "1000.00"), "100.00");
    assert_eq!(purchase_vat_on(code, "2025-01-01", "1000.00"), "140.00");
    assert_eq!(purchase_vat_on(code, "2025-12-31", "1000.00"), "140.00");
    assert_eq!(purchase_vat_on(code, "2026-01-01", "1000.00"), "135.00");
}

#[test]
fn electricity_temporary_reduced_rate_2022_2023_is_exact() {
    // Electricity temporarily reduced from the standard rate to 10 %
    // 1.12.2022–30.4.2023 (laki 1264/2022). Outside the window it tracks the
    // standard rate, so it also crosses the 24 %→25,5 % change on 1.9.2024.
    let code = "purchase-domestic-electricity";
    assert_eq!(purchase_vat_on(code, "2022-11-30", "1000.00"), "240.00");
    assert_eq!(purchase_vat_on(code, "2022-12-01", "1000.00"), "100.00");
    assert_eq!(purchase_vat_on(code, "2023-04-30", "1000.00"), "100.00");
    assert_eq!(purchase_vat_on(code, "2023-05-01", "1000.00"), "240.00");
    assert_eq!(purchase_vat_on(code, "2024-08-31", "1000.00"), "240.00");
    assert_eq!(purchase_vat_on(code, "2024-09-01", "1000.00"), "255.00");
}

#[test]
fn the_reduced_1_band_boundary_is_exact() {
    // The generic 14 %→13,5 % reduced band on 1.1.2026 (already in the table) —
    // a golden vector so the boundary is guarded on the band code an ordinary
    // reduced-rate booking uses, not only on the dedicated item classes.
    let code = "purchase-domestic-reduced-1";
    assert_eq!(purchase_vat_on(code, "2025-12-31", "1000.00"), "140.00");
    assert_eq!(purchase_vat_on(code, "2026-01-01", "1000.00"), "135.00");
}

#[test]
fn temporary_zero_rate_applies_on_the_sales_direction_too() {
    // The dated table is direction-agnostic: a passenger-transport *sale* inside
    // the temporary window is 0 % just as the purchase side is (regression guard
    // for the sales- path, whose accrual side is Credit).
    let v = vat_voucher(
        1,
        "2023-02-15",
        "2023-02-15",
        "3000",
        "1000.00",
        "sales-domestic-passenger-transport",
        "1910",
    );
    let period = PeriodId::containing(date("2023-02-15"));
    assert_eq!(
        derive_vat_figures(&period, &[v])
            .unwrap()
            .sales_vat
            .amount_str(),
        "0.00"
    );
}

#[test]
fn new_rate_classes_resolve_and_reject_out_of_range_dates() {
    // The temporary-measure classes are fully dated (regression guard): every
    // edition resolves, and a date before the first edition is refused loudly
    // rather than extrapolated (rates are dated data, never guessed).
    assert!(resolve_vat_rate("domestic-passenger-transport", date("2023-02-15")).is_ok());
    assert!(resolve_vat_rate("domestic-electricity", date("2023-02-15")).is_ok());
    let pt = resolve_vat_rate("domestic-passenger-transport", date("2012-12-31")).unwrap_err();
    assert_eq!(pt.error_code(), "VAT_CODE");
    let el = resolve_vat_rate("domestic-electricity", date("2012-12-31")).unwrap_err();
    assert_eq!(el.error_code(), "VAT_CODE");
}

#[test]
fn a_gap_between_editions_is_rejected_at_load() {
    // A hole between two editions of a class would brick any voucher dated in the
    // gap at query time; `validate_vat_rate_table` must refuse it at load. Here
    // 2023-04-30 → 2023-06-01 leaves May 2023 uncovered.
    let table = VatRateTable {
        table: "t".into(),
        editions: vec![
            VatRateEdition {
                rate_class: "domestic-electricity".into(),
                valid_from: date("2013-01-01"),
                valid_to: Some(date("2023-04-30")),
                percent: "10.0".into(),
            },
            VatRateEdition {
                rate_class: "domestic-electricity".into(),
                valid_from: date("2023-06-01"),
                valid_to: None,
                percent: "24.0".into(),
            },
        ],
    };
    let err = validate_vat_rate_table(&table).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn contiguous_adjacent_editions_are_accepted() {
    // The mirror of the gap test: a day-after adjacency (2023-04-30 → 2023-05-01)
    // is exactly contiguous and must pass.
    let table = VatRateTable {
        table: "t".into(),
        editions: vec![
            VatRateEdition {
                rate_class: "domestic-electricity".into(),
                valid_from: date("2013-01-01"),
                valid_to: Some(date("2023-04-30")),
                percent: "10.0".into(),
            },
            VatRateEdition {
                rate_class: "domestic-electricity".into(),
                valid_from: date("2023-05-01"),
                valid_to: None,
                percent: "24.0".into(),
            },
        ],
    };
    assert!(validate_vat_rate_table(&table).is_ok());
}

#[test]
fn a_non_eur_vat_posting_is_refused() {
    let usd = Posting {
        account: AccountId::new("3000").unwrap(),
        amount: Money::parse("100.00", Currency::new("USD").unwrap()).unwrap(),
        side: Side::Credit,
        vat_code: Some(VatCode::new("sales-domestic-standard").unwrap()),
    };
    let counter = Posting {
        account: AccountId::new("1910").unwrap(),
        amount: Money::parse("100.00", Currency::new("USD").unwrap()).unwrap(),
        side: Side::Debit,
        vat_code: None,
    };
    let draft = aggountant_model::VoucherDraft::new(
        date("2026-02-15"),
        date("2026-02-15"),
        "usd sale".into(),
        vec![usd, counter],
        None,
    )
    .unwrap();
    let body = VoucherBody::from_draft(&draft, 1, "fi-general-2026@0".into()).unwrap();
    let err = derive_vat_figures(&q1(), &[body]).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CURRENCY");
}

#[test]
fn the_bundled_rate_table_loads_and_validates() {
    // Exercises the OnceLock init (parse + validate) and the invariant checker.
    let table = vat_rate_table();
    assert_eq!(table.table, "fi-vat-rates");
    validate_vat_rate_table(table).unwrap();
}

#[test]
fn overlapping_rate_editions_are_rejected() {
    let table = VatRateTable {
        table: "t".into(),
        editions: vec![
            VatRateEdition {
                rate_class: "domestic-standard".into(),
                valid_from: date("2020-01-01"),
                valid_to: None,
                percent: "24.0".into(),
            },
            VatRateEdition {
                rate_class: "domestic-standard".into(),
                valid_from: date("2024-09-01"),
                valid_to: None,
                percent: "25.5".into(),
            },
        ],
    };
    assert!(validate_vat_rate_table(&table).is_err());
}

// --- AVL 117 § partial input VAT deductibility ------------------------

#[test]
fn a_partial_deductibility_purchase_scales_the_input_vat() {
    // AVL 117 § — a car with 50 % private use: 10000 base × 25,5 % = 2550 full
    // input VAT, of which only 50 % (1275) is deductible. The sales side is
    // untouched (there is no marker there — output VAT is never fractional).
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "4000",
        "10000.00",
        "purchase-domestic-standard-deduct50",
        "1910",
    )];
    let d = derive_vat_figures(&q1(), &vouchers).unwrap();
    assert_eq!(d.purchase_vat.amount_str(), "1275.00");
    assert_eq!(d.sales_vat.amount_str(), "0.00");
}

#[test]
fn full_deductibility_is_the_plain_code_with_no_marker() {
    // The plain code (no marker) is 100 % deductible — a partial code and its
    // full-rate sibling must agree when the fraction is 100 %, and the plain
    // code keeps its existing figure unchanged (regression guard).
    let plain = derive_vat_figures(
        &q1(),
        &[vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "4000",
            "4000.00",
            "purchase-domestic-standard",
            "1910",
        )],
    )
    .unwrap();
    assert_eq!(plain.purchase_vat.amount_str(), "1020.00");
}

#[test]
fn the_deduct_marker_does_not_disturb_a_hyphenated_rate_class() {
    // `domestic-reduced-1` already ends in a digit; the marker must strip off the
    // *tail* only, leaving the reduced-rate class intact. 1000 × 13,5 % (in force
    // from 2026-01-01) = 135, × 50 % = 67.50.
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "4000",
        "1000.00",
        "purchase-domestic-reduced-1-deduct50",
        "1910",
    )];
    let d = derive_vat_figures(&q1(), &vouchers).unwrap();
    assert_eq!(d.purchase_vat.amount_str(), "67.50");
}

#[test]
fn partial_deductibility_rounds_the_invoice_vat_then_the_fraction() {
    // Two rounding steps: round the full invoice VAT first, then the deductible
    // share. 10.30 base × 25,5 % = 2.6265 → round = 2.63 (the euro on the invoice);
    // × 50 % = 1.315 → round half away = 1.32. A one-step product would instead be
    // round(10.30 × 0.255 × 0.5) = round(1.31325) = 1.31 — so this pins the
    // two-step policy (apply the fraction to the *actual* invoice VAT, and keep
    // deductible + non-deductible = full VAT exactly).
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "4000",
        "10.30",
        "purchase-domestic-standard-deduct50",
        "1910",
    )];
    let d = derive_vat_figures(&q1(), &vouchers).unwrap();
    assert_eq!(d.purchase_vat.amount_str(), "1.32");
}

#[test]
fn the_deduct_marker_boundaries_1_and_99_are_accepted() {
    // 1 % and 99 % are the extremes of the canonical range. 10000 × 25,5 % = 2550
    // full VAT; × 1 % = 25.50, × 99 % = 2524.50.
    let one = derive_vat_figures(
        &q1(),
        &[vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "4000",
            "10000.00",
            "purchase-domestic-standard-deduct1",
            "1910",
        )],
    )
    .unwrap();
    assert_eq!(one.purchase_vat.amount_str(), "25.50");
    let ninety_nine = derive_vat_figures(
        &q1(),
        &[vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "4000",
            "10000.00",
            "purchase-domestic-standard-deduct99",
            "1910",
        )],
    )
    .unwrap();
    assert_eq!(ninety_nine.purchase_vat.amount_str(), "2524.50");
}

#[test]
fn a_duplicated_deduct_marker_is_refused() {
    // `rsplit_once` takes the rightmost marker; a second one left in the class part
    // is a malformed duplicate, refused rather than passed to the rate lookup.
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "4000",
        "1000.00",
        "purchase-domestic-standard-deduct50-deduct25",
        "1910",
    )];
    let err = derive_vat_figures(&q1(), &vouchers).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn a_negative_signed_deduct_marker_is_refused() {
    // `-deduct-50` splits to a pct_str of "-50" (a hyphen is not an ASCII digit),
    // so it is refused rather than parsed as a negative percent.
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "4000",
        "1000.00",
        "purchase-domestic-standard-deduct-50",
        "1910",
    )];
    let err = derive_vat_figures(&q1(), &vouchers).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn a_reserved_deduct_word_in_a_rate_class_is_rejected_at_load() {
    // The `-deduct<NN>` marker reserves the word `deduct`; a rate class containing
    // it would misparse, so the table validator rejects it at load time.
    let table = VatRateTable {
        table: "t".into(),
        editions: vec![VatRateEdition {
            rate_class: "special-deduction".into(),
            valid_from: date("2020-01-01"),
            valid_to: None,
            percent: "24.0".into(),
        }],
    };
    let err = validate_vat_rate_table(&table).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn a_reverse_charge_acquisition_can_be_partially_deductible() {
    // A mixed-use EU acquisition, reverse-charged (AVL 9 §) with 50 % deductible
    // input (AVL 117 §): the self-assessed output VAT stays **full** (2550) while
    // only half the input is deductible (1275). Net VAT owed to Vero is the non-
    // deductible half — the correct law outcome, and the case with no prior test.
    let d = derive_vat_figures(
        &q1(),
        &[reverse_charge_voucher(
            1,
            "10000.00",
            "purchase-domestic-standard-deduct50",
            "sales-domestic-standard",
        )],
    )
    .unwrap();
    assert_eq!(
        d.sales_vat.amount_str(),
        "2550.00",
        "full self-assessed output"
    );
    assert_eq!(
        d.purchase_vat.amount_str(),
        "1275.00",
        "half-deductible input"
    );
}

#[test]
fn a_partial_deductibility_purchase_return_reduces_input_vat_proportionally() {
    // A 4000 purchase at deduct50 (→ 510) and a 1000 same-code return (credit,
    // → −127.50): deductible input VAT = 510 − 127.50 = 382.50. The fraction
    // applies to the reversal exactly as to the accrual.
    let vouchers = vec![
        vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "4000",
            "4000.00",
            "purchase-domestic-standard-deduct50",
            "1910",
        ),
        vat_voucher_sided(
            2,
            "2026-02-15",
            "2026-02-15",
            "4000",
            "1000.00",
            "purchase-domestic-standard-deduct50",
            Side::Credit,
            "1910",
        ),
    ];
    let d = derive_vat_figures(&q1(), &vouchers).unwrap();
    assert_eq!(d.purchase_vat.amount_str(), "382.50");
}

#[test]
fn a_deduct_marker_on_a_sales_code_is_refused() {
    // Output VAT on a sale is never fractional (AVL 117 § is an input concept),
    // so a `sales-*-deduct*` code is refused rather than silently scaling output.
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "3000",
        "1000.00",
        "sales-domestic-standard-deduct50",
        "1910",
    )];
    let err = derive_vat_figures(&q1(), &vouchers).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
    assert!(err.to_string().contains("purchase-"), "got {err}");
}

#[test]
fn non_canonical_deduct_markers_are_refused() {
    // 0 % (not a deduction — book into cost), 100 % (use the plain code), a
    // leading zero (`deduct05`), an over-100 value, and a non-numeric marker are
    // all refused so a code has exactly one canonical spelling per meaning.
    for code in [
        "purchase-domestic-standard-deduct0",
        "purchase-domestic-standard-deduct00",
        "purchase-domestic-standard-deduct05",
        "purchase-domestic-standard-deduct100",
        "purchase-domestic-standard-deduct999",
        "purchase-domestic-standard-deductfoo",
    ] {
        let vouchers = vec![vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "4000",
            "1000.00",
            code,
            "1910",
        )];
        let err =
            derive_vat_figures(&q1(), &vouchers).expect_err(&format!("{code:?} should be refused"));
        assert_eq!(err.error_code(), "VAT_CODE", "for {code:?}");
    }
}

#[test]
fn a_deduct_marker_with_an_empty_rate_class_is_refused() {
    // `purchase-deduct50` has the marker but no rate class to scale.
    let vouchers = vec![vat_voucher(
        1,
        "2026-02-15",
        "2026-02-15",
        "4000",
        "1000.00",
        "purchase-deduct50",
        "1910",
    )];
    let err = derive_vat_figures(&q1(), &vouchers).unwrap_err();
    assert_eq!(err.error_code(), "VAT_CODE");
}

#[test]
fn a_partial_deductibility_purchase_passes_the_structural_guard() {
    // A partial-deductibility purchase is a single positive purchase-direction
    // contribution — no same-direction cancellation — so it builds normally.
    let postings = vec![
        Posting {
            account: AccountId::new("4000").unwrap(),
            amount: eur_money("10000.00"),
            side: Side::Debit,
            vat_code: Some(VatCode::new("purchase-domestic-standard-deduct50").unwrap()),
        },
        Posting {
            account: AccountId::new("1910").unwrap(),
            amount: eur_money("10000.00"),
            side: Side::Credit,
            vat_code: None,
        },
    ];
    check_voucher_vat_structure(&postings).unwrap();
}

// --- AVL 72 b § EU-sale buyer-VAT-ID gate -----------------------------

/// A single `sales-eu-goods` line (balanced by a bank leg) — the shape a real
/// EU-goods sale takes. The buyer VAT-ID gate keys off the tagged line.
pub(super) fn eu_sale_postings(code: &str) -> Vec<Posting> {
    vec![
        Posting {
            account: AccountId::new("1910").unwrap(),
            amount: eur_money("10000.00"),
            side: Side::Debit,
            vat_code: None,
        },
        Posting {
            account: AccountId::new("3000").unwrap(),
            amount: eur_money("10000.00"),
            side: Side::Credit,
            vat_code: Some(VatCode::new(code).unwrap()),
        },
    ]
}

#[test]
fn explicitly_selected_month_does_not_include_adjacent_quarter_months() {
    let vouchers = vec![
        vat_voucher(
            1,
            "2026-06-30",
            "2026-06-30",
            "3000",
            "100.00",
            "sales-domestic-standard",
            "1910",
        ),
        vat_voucher(
            2,
            "2026-07-01",
            "2026-07-01",
            "3000",
            "200.00",
            "sales-domestic-standard",
            "1910",
        ),
        vat_voucher(
            3,
            "2026-07-31",
            "2026-07-31",
            "3000",
            "300.00",
            "sales-domestic-standard",
            "1910",
        ),
        vat_voucher(
            4,
            "2026-08-01",
            "2026-08-01",
            "3000",
            "400.00",
            "sales-domestic-standard",
            "1910",
        ),
    ];
    let july = PeriodId::parse("2026-07").unwrap();
    let figures = derive_vat_figures(&july, &vouchers).unwrap();
    assert_eq!(figures.sales_vat.amount_str(), "127.50");
    assert_eq!(figures.period, july);
}

#[test]
fn domestic_tagged_sale_populates_numbered_tax_from_the_net_base() {
    let posting = |account: &str, amount: &str, side, code: Option<&str>| Posting {
        account: AccountId::new(account).unwrap(),
        amount: eur_money(amount),
        side,
        vat_code: code.map(|code| VatCode::new(code).unwrap()),
    };
    let draft = VoucherDraft::new(
        date("2026-02-15"),
        date("2026-02-15"),
        "fictional domestic sale".into(),
        vec![
            posting("1700", "125.50", Side::Debit, None),
            posting(
                "3000",
                "100.00",
                Side::Credit,
                Some("sales-domestic-standard"),
            ),
            posting("2939", "25.50", Side::Credit, None),
        ],
        None,
    )
    .unwrap();
    let body = VoucherBody::from_draft(&draft, 1, "fi-general-2026@0".into()).unwrap();
    let figures = derive_vat_figures(&q1(), &[body]).unwrap();
    assert_eq!(figures.sales_vat.amount_str(), "25.50");
    assert_eq!(
        figures
            .fields
            .get(VatReturnField::Field301)
            .unwrap()
            .amount_str(),
        "25.50"
    );
    let filing = compute_derived_vat_filing(&figures).unwrap();
    assert_eq!(
        filing
            .fields
            .get(VatReturnField::Field308)
            .unwrap()
            .amount_str(),
        "25.50"
    );
    assert_eq!(filing.fields.iter().count(), 3);
}

#[test]
fn domestic_tagged_sales_slots_share_signed_rounded_aggregate_contributions() {
    for (class, field, tax) in [
        ("domestic-standard", VatReturnField::Field301, "25.50"),
        ("domestic-reduced-1", VatReturnField::Field302, "13.50"),
        ("domestic-reduced-2", VatReturnField::Field303, "10.00"),
    ] {
        let code = format!("sales-{class}");
        let sale = vat_voucher(
            1,
            "2026-02-15",
            "2026-02-15",
            "3000",
            "100.00",
            &code,
            "1700",
        );
        let credit = vat_voucher_sided(
            2,
            "2026-02-15",
            "2026-02-15",
            "3000",
            "20.00",
            &code,
            Side::Debit,
            "1700",
        );
        let outside = vat_voucher(
            3,
            "2026-02-15",
            "2026-04-01",
            "3000",
            "900.00",
            &code,
            "1700",
        );
        let full = derive_vat_figures(&q1(), std::slice::from_ref(&sale)).unwrap();
        assert_eq!(full.fields.get(field).unwrap().amount_str(), tax);
        let reversal = derive_vat_figures(&q1(), std::slice::from_ref(&credit)).unwrap();
        assert!(reversal.fields.get(field).unwrap().amount() < Decimal::ZERO);
        assert_eq!(reversal.fields.get(field).unwrap(), &reversal.sales_vat);
        let net = derive_vat_figures(&q1(), &[sale, credit, outside]).unwrap();
        assert_eq!(net.fields.get(field).unwrap(), &net.sales_vat);
        assert_eq!(
            net.sales_vat.amount(),
            full.sales_vat.amount() * Decimal::new(80, 2)
        );
        assert_eq!(net.fields.iter().count(), 1);
    }
    let historical = vat_voucher(
        1,
        "2024-08-31",
        "2026-02-15",
        "3000",
        "100.00",
        "sales-domestic-standard",
        "1700",
    );
    let figures = derive_vat_figures(&q1(), &[historical]).unwrap();
    assert_eq!(
        figures
            .fields
            .get(VatReturnField::Field301)
            .unwrap()
            .amount_str(),
        "24.00"
    );
}

#[test]
fn legacy_tagged_self_assessment_keeps_aggregates_without_guessing_sales_category() {
    for (input_code, purchase_vat) in [
        ("purchase-domestic-standard", "25.50"),
        ("purchase-domestic-standard-deduct50", "12.75"),
    ] {
        let voucher = reverse_charge_voucher(1, "100.00", input_code, "sales-domestic-standard");
        let figures = derive_vat_figures(&q1(), &[voucher]).unwrap();
        assert_eq!(figures.sales_vat.amount_str(), "25.50");
        assert_eq!(figures.purchase_vat.amount_str(), purchase_vat);
        // Scope is not encoded by these legacy tags. Do not newly assign 301,
        // infer 305/306, or refuse a previously sealed voucher.
        assert_eq!(figures.fields.iter().count(), 0);
        let filing = compute_derived_vat_filing(&figures).unwrap();
        assert!(filing.fields.get(VatReturnField::Field301).is_none());
        assert_eq!(filing.fields.iter().count(), 2);
    }
}
