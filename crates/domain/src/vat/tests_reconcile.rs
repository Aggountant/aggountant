use super::*;

const RECONCILE_ROLES_JSON: &str = r#"{
  "table":"test-reconcile-roles",
  "charts":[
    {"chart_version":"fi-general-2026@0","active":true,"accounts":[
      {"account":"1763","role":"input-vat"},
      {"account":"2939","role":"tagged-only","generated_role":"output"}
    ],"domestic_input_account":"1763","reverse_charge_targets":[
      {"scope":"eu-services","input_account":"1763","output_account":"2939"},
      {"scope":"non-eu-services","input_account":"1763","output_account":"2939"}
    ]},
    {"chart_version":"legacy-company@roles-v1","active":true,"accounts":[
      {"account":"1763","role":"input-vat"},
      {"account":"2939","role":"tagged-only"}
    ]}
  ]
}"#;

fn roles() -> VatAccountRoleTable {
    VatAccountRoleTable::from_json(RECONCILE_ROLES_JSON).unwrap()
}

/// The chart kinds the tests need; any other account is unknown to the chart.
fn kind_of(account: &str) -> Option<AccountKind> {
    Some(match account {
        "1700" | "1763" | "1910" => AccountKind::Asset,
        "2700" | "2740" | "2939" => AccountKind::Liability,
        "3000" | "9160" => AccountKind::Income,
        "4000" | "7660" | "8350" | "8380" | "8450" => AccountKind::Expense,
        _ => return None,
    })
}

fn line(account: &str, side: Side, amount: &str, code: Option<&str>) -> Posting {
    Posting {
        account: AccountId::new(account).unwrap(),
        amount: eur_money(amount),
        side,
        vat_code: code.map(|c| VatCode::new(c).unwrap()),
    }
}

fn voucher(number: u64, chart_version: &str, postings: Vec<Posting>) -> VoucherBody {
    let draft = VoucherDraft::new(
        date("2026-03-05"),
        date("2026-03-05"),
        "test voucher".into(),
        postings,
        None,
    )
    .unwrap();
    VoucherBody::from_draft(&draft, number, chart_version.into()).unwrap()
}

fn march() -> PeriodId {
    PeriodId::parse("2026-03").unwrap()
}

fn reconcile(vouchers: &[VoucherBody]) -> VatAccountReconciliation {
    let table = roles();
    let derived = derive_filing_figures_with_role_table(&march(), vouchers, &table).unwrap();
    reconcile_vat_accounts(&march(), vouchers, &derived, &table, &kind_of).unwrap()
}

/// The sale from the issue: 2 480,00 gross, 503,90 VAT on 2939, no tag anywhere.
fn untagged_sale(number: u64) -> VoucherBody {
    voucher(
        number,
        "fi-general-2026@0",
        vec![
            line("1700", Side::Debit, "2480.00", None),
            line("3000", Side::Credit, "1976.10", None),
            line("2939", Side::Credit, "503.90", None),
        ],
    )
}

#[test]
fn an_untagged_sale_vat_leg_is_reported_as_left_out_of_the_return() {
    let r = reconcile(&[untagged_sale(1)]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.output.derived.amount_str(), "0.00");
    assert_eq!(r.output.ledger_movement.amount_str(), "503.90");
    assert_eq!(r.output.untagged.amount_str(), "503.90");
    assert_eq!(r.output.difference.amount_str(), "503.90");
    assert_eq!(r.untagged_vouchers(), vec![1]);
    let posting = &r.untagged_postings[0];
    assert_eq!(
        (posting.account.as_str(), posting.posting_index),
        ("2939", 2)
    );
    assert_eq!(posting.direction, Some(GeneratedVatRole::Output));
    assert_eq!(r.accounts.len(), 1);
    assert_eq!(r.accounts[0].credit.amount_str(), "503.90");
}

#[test]
fn a_tagged_sale_reconciles_and_shows_only_the_per_line_rounding() {
    let sale = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("1700", Side::Debit, "2480.00", None),
            line(
                "3000",
                Side::Credit,
                "1976.10",
                Some("sales-domestic-standard"),
            ),
            line("2939", Side::Credit, "503.90", None),
        ],
    );
    let r = reconcile(&[sale]);
    assert_eq!(r.status, VatReconciliationStatus::Reconciled);
    assert!(r.untagged_postings.is_empty());
    // round(1976.10 × 25,5 %) = 503,91 is what the return files.
    assert_eq!(r.output.derived.amount_str(), "503.91");
    assert_eq!(r.output.difference.amount_str(), "-0.01");
    assert_eq!(r.output.untagged.amount_str(), "0.00");
}

#[test]
fn generated_input_vat_on_the_input_vat_account_reconciles_exactly() {
    let purchase = expand_body(
        1,
        "125.50",
        "7660",
        "1910",
        domestic_standard(),
        "2026-03-05",
        "2026-03-05",
    );
    let r = reconcile(&[purchase]);
    assert_eq!(r.status, VatReconciliationStatus::Reconciled);
    assert_eq!(r.input.derived.amount_str(), "25.50");
    assert_eq!(r.input.ledger_movement.amount_str(), "25.50");
    assert_eq!(r.input.difference.amount_str(), "0.00");
}

#[test]
fn untagged_input_vat_booked_with_an_expense_is_left_out() {
    let purchase = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("4000", Side::Debit, "100.00", None),
            line("1763", Side::Debit, "25.50", None),
            line("2700", Side::Credit, "125.50", None),
        ],
    );
    let r = reconcile(&[purchase]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.input.untagged.amount_str(), "25.50");
}

#[test]
fn vat_payments_and_clearings_are_settlements_not_return_figures() {
    let payment = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("2939", Side::Debit, "400.00", None),
            line("1910", Side::Credit, "400.00", None),
        ],
    );
    let clearing = voucher(
        2,
        "fi-general-2026@0",
        vec![
            line("2939", Side::Debit, "25.50", None),
            line("1763", Side::Credit, "25.50", None),
        ],
    );
    let r = reconcile(&[payment, clearing]);
    assert_eq!(r.status, VatReconciliationStatus::Reconciled);
    assert_eq!(r.settlement_postings.len(), 3);
    assert_eq!(r.output.ledger_movement.amount_str(), "-425.50");
    assert_eq!(r.output.settlements.amount_str(), "-425.50");
    assert_eq!(r.output.difference.amount_str(), "0.00");
    assert_eq!(r.input.settlements.amount_str(), "-25.50");
}

#[test]
fn a_vat_payment_that_also_books_interest_is_a_settlement() {
    // The issue's reproduction: previous period's VAT, accrued interest and
    // interest expense paid in one transfer.
    let payment = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("2939", Side::Debit, "500.00", None),
            line("2740", Side::Debit, "2.00", None),
            line("8350", Side::Debit, "0.50", None),
            line("1910", Side::Credit, "502.50", None),
        ],
    );
    let r = reconcile(&[payment]);
    assert_eq!(r.status, VatReconciliationStatus::Reconciled);
    assert!(r.untagged_postings.is_empty());
    assert_eq!(r.output.settlements.amount_str(), "-500.00");
    assert_eq!(r.output.untagged.amount_str(), "0.00");
    assert_eq!(r.output.difference.amount_str(), "0.00");
}

#[test]
fn a_vat_payment_with_interest_and_a_penalty_is_a_settlement() {
    let payment = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("2939", Side::Debit, "380.00", None),
            line("8350", Side::Debit, "1.20", None),
            line("8380", Side::Debit, "50.00", None),
            line("1910", Side::Credit, "431.20", None),
        ],
    );
    let r = reconcile(&[payment]);
    assert_eq!(r.status, VatReconciliationStatus::Reconciled);
    assert!(r.untagged_postings.is_empty());
    assert_eq!(r.output.settlements.amount_str(), "-380.00");
}

#[test]
fn a_net_payment_clearing_input_vat_with_interest_flags_the_input_leg() {
    // An expense next to an input-VAT reduction may be a deduction correction, so
    // the 1763 leg needs an acknowledgement; the 2939 leg stays a settlement.
    let payment = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("2939", Side::Debit, "500.00", None),
            line("1763", Side::Credit, "120.00", None),
            line("8350", Side::Debit, "1.20", None),
            line("1910", Side::Credit, "381.20", None),
        ],
    );
    let r = reconcile(&[payment]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.input.untagged.amount_str(), "-120.00");
    assert_eq!(r.output.settlements.amount_str(), "-500.00");
}

#[test]
fn an_input_vat_deduction_correction_in_a_clearing_voucher_still_blocks() {
    let memo = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("2939", Side::Debit, "500.00", None),
            line("1763", Side::Credit, "120.00", None),
            line("2700", Side::Credit, "380.00", None),
            line("7660", Side::Debit, "10.00", None),
            line("1763", Side::Credit, "10.00", None),
        ],
    );
    let r = reconcile(&[memo]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.input.untagged.amount_str(), "-130.00");
}

#[test]
fn a_sale_netted_into_the_vat_payment_line_still_blocks() {
    // 500,00 paid and a 124,00 cash sale (25,20 VAT) written as one 2939 debit.
    let netted = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("2939", Side::Debit, "474.80", None),
            line("3000", Side::Credit, "98.80", None),
            line("1910", Side::Credit, "376.00", None),
        ],
    );
    let r = reconcile(&[netted]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.output.untagged.amount_str(), "-474.80");
}

#[test]
fn a_vat_refund_received_with_interest_income_is_a_settlement() {
    let refund = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("1910", Side::Debit, "301.50", None),
            line("1763", Side::Credit, "300.00", None),
            line("9160", Side::Credit, "1.50", None),
        ],
    );
    let r = reconcile(&[refund]);
    assert_eq!(r.status, VatReconciliationStatus::Reconciled);
    assert!(r.untagged_postings.is_empty());
    assert_eq!(r.input.settlements.amount_str(), "-300.00");
}

#[test]
fn an_untagged_sales_credit_note_is_left_out() {
    let credit_note = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("3000", Side::Debit, "100.00", None),
            line("2939", Side::Debit, "25.50", None),
            line("1700", Side::Credit, "125.50", None),
        ],
    );
    let r = reconcile(&[credit_note]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.output.untagged.amount_str(), "-25.50");
    assert!(r.settlement_postings.is_empty());
}

#[test]
fn an_untagged_credit_loss_vat_reduction_is_left_out() {
    // A credit loss books its net on an expense, like interest does; the net is
    // several times the VAT, so it is not taken for a settlement.
    let credit_loss = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("8450", Side::Debit, "100.00", None),
            line("2939", Side::Debit, "25.50", None),
            line("1700", Side::Credit, "125.50", None),
        ],
    );
    let r = reconcile(&[credit_loss]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.output.untagged.amount_str(), "-25.50");
}

#[test]
fn an_untagged_sale_booked_with_a_vat_payment_still_blocks() {
    // Income next to the payment's debit keeps it untagged too; the sale's
    // accrual-side leg is untagged in any case.
    let mixed = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("2939", Side::Debit, "500.00", None),
            line("1910", Side::Credit, "500.00", None),
            line("1700", Side::Debit, "124.00", None),
            line("3000", Side::Credit, "98.80", None),
            line("2939", Side::Credit, "25.20", None),
        ],
    );
    let r = reconcile(&[mixed]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.output.untagged.amount_str(), "-474.80");
    assert!(r.settlement_postings.is_empty());
}

#[test]
fn an_untagged_supplier_credit_note_booked_with_a_vat_payment_still_blocks() {
    // The amounts alone would pass (100,00 < 525,50); an expense credit is neither
    // interest nor a penalty, so both VAT reductions stay untagged.
    let mixed = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("2939", Side::Debit, "500.00", None),
            line("1910", Side::Credit, "500.00", None),
            line("2700", Side::Debit, "125.50", None),
            line("4000", Side::Credit, "100.00", None),
            line("1763", Side::Credit, "25.50", None),
        ],
    );
    let r = reconcile(&[mixed]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.input.untagged.amount_str(), "-25.50");
    assert!(r.settlement_postings.is_empty());
}

#[test]
fn an_untagged_customer_refund_booked_with_a_vat_payment_still_blocks() {
    let mixed = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("2939", Side::Debit, "500.00", None),
            line("3000", Side::Debit, "100.00", None),
            line("2939", Side::Debit, "25.50", None),
            line("1910", Side::Credit, "625.50", None),
        ],
    );
    let r = reconcile(&[mixed]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.output.untagged.amount_str(), "-525.50");
}

#[test]
fn an_untagged_purchase_return_reducing_input_vat_is_left_out() {
    // A reduction booked together with an expense is a credit note, not a
    // settlement: the return would otherwise keep the full deduction.
    let credit_note = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("2700", Side::Debit, "125.50", None),
            line("4000", Side::Credit, "100.00", None),
            line("1763", Side::Credit, "25.50", None),
        ],
    );
    let r = reconcile(&[credit_note]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.input.untagged.amount_str(), "-25.50");
}

#[test]
fn a_role_without_a_generated_direction_takes_it_from_the_account_kind() {
    let payment = voucher(
        1,
        "legacy-company@roles-v1",
        vec![
            line("2939", Side::Debit, "400.00", None),
            line("1910", Side::Credit, "400.00", None),
        ],
    );
    let r = reconcile(&[payment]);
    assert_eq!(r.status, VatReconciliationStatus::Reconciled);
    assert_eq!(
        r.settlement_postings[0].direction,
        Some(GeneratedVatRole::Output)
    );
}

#[test]
fn an_account_missing_from_the_chart_never_makes_a_settlement() {
    let unknown_counter = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("2939", Side::Debit, "400.00", None),
            line("9999", Side::Credit, "400.00", None),
        ],
    );
    let r = reconcile(&[unknown_counter]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
}

#[test]
fn vouchers_outside_the_period_are_ignored() {
    let draft = VoucherDraft::new(
        date("2026-04-02"),
        date("2026-04-02"),
        "april sale".into(),
        untagged_sale(1).postings().to_vec(),
        None,
    )
    .unwrap();
    let april = VoucherBody::from_draft(&draft, 1, "fi-general-2026@0".into()).unwrap();
    let r = reconcile(&[april]);
    assert_eq!(r.status, VatReconciliationStatus::Reconciled);
    assert!(r.accounts.is_empty());
}

#[test]
fn a_vat_coded_correction_in_the_same_period_reconciles_the_untagged_sale() {
    // The correction moves the base onto a VAT-coded line; the original 2939 leg
    // stays, and the return now carries the VAT within per-line rounding.
    let correction = voucher(
        2,
        "fi-general-2026@0",
        vec![
            line("3000", Side::Debit, "1976.10", None),
            line(
                "3000",
                Side::Credit,
                "1976.10",
                Some("sales-domestic-standard"),
            ),
        ],
    );
    let r = reconcile(&[untagged_sale(1), correction]);
    assert_eq!(r.status, VatReconciliationStatus::Reconciled);
    assert_eq!(r.untagged_vouchers(), vec![1]);
    assert_eq!(r.output.difference.amount_str(), "-0.01");
    assert_eq!(r.output.rounding_tolerance.amount_str(), "0.01");
}

#[test]
fn rounding_tolerance_does_not_hide_an_untagged_cent_beyond_it() {
    // One tagged line allows one cent; an untagged two-cent leg on top of the
    // tagged sale's own rounding is a real difference.
    let tagged = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("1700", Side::Debit, "2480.00", None),
            line(
                "3000",
                Side::Credit,
                "1976.10",
                Some("sales-domestic-standard"),
            ),
            line("2939", Side::Credit, "503.90", None),
        ],
    );
    let stray = voucher(
        2,
        "fi-general-2026@0",
        vec![
            line("1700", Side::Debit, "0.03", None),
            line("2939", Side::Credit, "0.03", None),
        ],
    );
    let r = reconcile(&[tagged, stray]);
    assert_eq!(r.output.difference.amount_str(), "0.02");
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
}

#[test]
fn a_partly_tagged_invoice_does_not_hide_the_uncoded_lines_vat() {
    let sale = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("1700", Side::Debit, "251.00", None),
            line(
                "3000",
                Side::Credit,
                "100.00",
                Some("sales-domestic-standard"),
            ),
            line("3000", Side::Credit, "100.00", None),
            line("2939", Side::Credit, "51.00", None),
        ],
    );
    let r = reconcile(&[sale]);
    assert_eq!(r.output.derived.amount_str(), "25.50");
    assert_eq!(r.output.difference.amount_str(), "25.50");
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.untagged_vouchers(), vec![1]);
}

#[test]
fn a_code_of_the_other_direction_does_not_shield_an_uncoded_sale() {
    // A payout netting an uncoded sale against a coded fee.
    let payout = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("1910", Side::Debit, "112.95", None),
            line(
                "4000",
                Side::Debit,
                "10.00",
                Some("purchase-domestic-standard"),
            ),
            line("1763", Side::Debit, "2.55", None),
            line("3000", Side::Credit, "100.00", None),
            line("2939", Side::Credit, "25.50", None),
        ],
    );
    let r = reconcile(&[payout]);
    assert_eq!(r.status, VatReconciliationStatus::Unreconciled);
    assert_eq!(r.untagged_postings.len(), 1);
    assert_eq!(r.untagged_postings[0].account, "2939");
    assert_eq!(r.input.difference.amount_str(), "0.00");
}

#[test]
fn coded_lines_without_vat_legs_stay_owned_by_their_codes() {
    let sale = voucher(
        1,
        "fi-general-2026@0",
        vec![
            line("1910", Side::Debit, "100.00", None),
            line(
                "3000",
                Side::Credit,
                "100.00",
                Some("sales-domestic-standard"),
            ),
        ],
    );
    let r = reconcile(&[sale]);
    assert_eq!(r.status, VatReconciliationStatus::Reconciled);
    assert!(r.untagged_postings.is_empty());
}
