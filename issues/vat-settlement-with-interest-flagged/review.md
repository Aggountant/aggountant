# Review: vat-settlement-with-interest-flagged

Focused `/llm-review`, rigor routine, high-risk surface (VAT filing data
integrity: a misclassified settlement silently drops VAT from a return).

- Finder: `gpt-6-astra`, diff `f7ace2b..ef9ef1f` (first implementation).
- Verifier: `claude-fable-5-2`, diff `f7ace2b..f47177f`; asked to kill the finder's
  finding and one false-negative question on silent omissions.
- Model calls: 2. Scope: `crates/domain/src/vat/reconcile.rs`, its tests, both
  `result.md` files, the user guide paragraph.

## F1 (must-fix): a credit note sharing a voucher with a VAT payment became a settlement

- Location: `crates/domain/src/vat/reconcile.rs`, `settles_vat`.
- Claim: the voucher-wide test "income + expense < reduced VAT" let an untagged
  supplier credit note (2700 D 125,50 / 4000 C 100,00 / 1763 C 25,50) booked
  together with a 500,00 VAT payment classify both VAT legs as settlements, so
  `file-vat start` would file without the 25,50 reduction of deductible VAT.
- Evidence: traced locally; the old rule (no income/expense at all) blocked it.
- Fix: commit f47177f added a per-direction kind test; the verifier executed the
  voucher at f47177f and got `Unreconciled` (refuted against the fixed code).
- Assessment: CONFIRMED, likelihood OCCASIONAL (cash-basis bank-statement
  vouchers), outcome FIX, applied.

## F2 (must-fix): further silent omissions past the kind test

- Location: `settles_vat`, the per-direction tolerance.
- Claim (verifier, executed in a scratch copy): at f47177f, an input-VAT deduction
  correction in a clearing memo (7400 D 10,00 / 1763 C 10,00 beside 2939 D 500,00 /
  1763 C 120,00 / 2740 C 380,00) and a sale netted into the payment line
  (2939 D 474,80 / 3000 C 98,80 / 1910 C 376,00) reconciled silently.
- Fix: output-VAT reductions now tolerate only expense debits and input-VAT
  reductions only income credits. Tests
  `an_input_vat_deduction_correction_in_a_clearing_voucher_still_blocks` and
  `a_sale_netted_into_the_vat_payment_line_still_blocks`. Cost, pinned by
  `a_net_payment_clearing_input_vat_with_interest_flags_the_input_leg`: a net
  payment that clears 1763 together with interest flags its 1763 leg (acknowledge).
- Assessment: CONFIRMED, likelihood OCCASIONAL (period-end VAT memo with a
  deduction correction is ordinary practice), outcome FIX, applied.

## F3 (should-fix): residual gaps no account-kind rule can close

- Claim (verifier): an output-VAT reduction whose net is an expense debit (customer
  refund on an expense account, credit loss) or an input-VAT reduction whose net is
  an income credit (supplier refund on other income) still counts as a settlement
  when a larger settlement shares the voucher; and the older no-income/no-expense
  shapes (advance-payment refund, fixed-asset purchase return, private-use
  correction through equity) predate this change.
- Assessment: CONFIRMED, likelihood RARE for the new part (needs a combined voucher
  and a net on the non-canonical kind), architecture MODERATE (needs a bank /
  tax-account role or settlement marker across ledger, process and chart
  snapshots), outcome SPIN_OFF: filed unlaned as
  `vat-settlement-shape-hides-untagged-reductions`; both known-gaps lists updated.

## Dropped

None. The verifier's "not examined" note (manual VAT legs in a voucher with a VAT
expansion) is pre-existing and unchanged: non-generated legs in such a voucher are
counted against the voucher's full net, which keeps them untagged.

## Simplicity

The change reuses the chart account kinds the classifier already receives; process
and ledger call sites are untouched. One helper (`settles_vat`) replaces the former
`touches_result` flag, and the reduction-side test is stated once. A bank role and
a settlement marker were deliberately left out (F3).

## Coverage

Read: the diff, `reconcile.rs`, `tests_reconcile.rs`, both callers
(`process/src/vat_reconciliation.rs`, `ledger/src/lib.rs` validate warning).
Tested: `cargo test -p aggountant-domain reconcile` (27 tests) and
`scripts/ci-local.sh`.
