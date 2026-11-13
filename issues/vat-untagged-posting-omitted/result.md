# Result: period VAT reconciliation

## What changed

- `domain::reconcile_vat_accounts` compares a period's movement on every VAT-role
  account with `DerivedVatFigures`, per direction (output / input). Each posting on a
  VAT-role account is classified by `classify_vat_account_postings`:
  - `tagged`: the voucher carries a `vat_code` whose own `base × rate` explains its
    VAT legs within a cent per coded line (legs it does not explain are `untagged`);
  - `generated`: a hash-bound generated leg of a sealed expansion;
  - `settlement`: untagged, on the side that reduces the VAT balance, in a voucher whose
    income and expense postings sum to less than its reduction-side VAT and are only
    interest or penalties: expense debits beside output VAT, income credits beside
    input VAT (VAT payment, refund, clearing between VAT accounts; refined by
    `vat-settlement-with-interest-flagged`);
  - `untagged`: everything else (the return leaves it out).
  The period is `unreconciled` when a direction has untagged postings and its
  `difference = ledger_movement − settlements − derived` exceeds the rounding
  tolerance (one cent per tagged posting of that direction), or when an untagged
  posting has no known direction or is not in EUR.
- `file-vat preview` returns `vat_reconciliation` and `attention` items
  (`VAT_ACCOUNTS_UNRECONCILED` blocking, `VAT_UNTAGGED_POSTINGS_COVERED` and
  `VAT_ACCOUNTS_DIFFER_ON_TAGGED_VOUCHERS` info). It stays read-only and works on
  filed periods (test `preview_audits_a_period_whose_return_is_already_filed`).
- `file-vat start` refuses an unreconciled period with `VAT_ACCOUNTS_UNRECONCILED`
  before the GetVATPeriods preflight, unless stdin carries
  `vat_reconciliation_acknowledgement {reason, vouchers}` naming exactly the
  untagged vouchers. The acknowledgement is stored in the Run state, in the
  reconcile effect's journal note and echoed in the start output. A wrong, blank or
  unneeded acknowledgement refuses with `VAT_RECONCILIATION_ACK_MISMATCH`.
- `voucher validate` returns `warnings` with `VAT_POSTING_UNTAGGED` for a draft
  posting that would be classified `untagged`. Sealing is not refused.
- User guide fi+en (VAT-return tasks, record-other-entries, vat-treatment, error
  codes, status) and the `capture-receipt`, `vero-fi` and `aggountant-init` skills.

`file_vat.rs` changes are limited to wiring: the snapshot carries the
reconciliation, the preview adds two fields, start calls `require_reconciled` and
records the acknowledgement. The logic lives in `process/src/vat_reconciliation.rs`
and `domain/src/vat/reconcile.rs`.

## Simplicity

- Reuses `DerivedVatFigures`, the chart role table (`generated_role_of`, `role_of`)
  and the ledger's chart list; no new masterdata, no model change.
- One classifier serves both the period check and the validate warning, so the
  draft-time and filing-time rules cannot drift.
- The per-voucher comparison of coded vouchers reuses `derive_vat_figures` on the
  single voucher instead of restating the `base × rate` rule.
- Deliberately left out: a voucher-kind marker for settlements and any automatic
  correction.

## Decisions for later review

1. **Settlements are excluded by side and voucher shape, not by account number.**
   A plain movement comparison would flag every month that contains a VAT payment
   (the default chart has only 2939 and 1763, so payments hit 2939). Reduction-side
   untagged postings are treated as settlements when the voucher's income and expense
   postings sum to less than that VAT and are only expense debits (beside output VAT)
   or income credits (beside input VAT), so a payment that also books interest or a
   penalty is one. Alternatives: an explicit voucher marker (model change), or flag
   everything and rely on acknowledgements (noise every month). Known gaps, all
   needing a bank or tax-account role or a settlement marker to close: an untagged
   reduction in a voucher with no income or expense (asset-sale credit note,
   advance-payment refund, fixed-asset purchase return, private-use correction
   through equity) counts as a settlement; so does an untagged output-VAT reduction
   whose net is an expense debit (credit loss, or a customer refund booked on an
   expense) or an input-VAT reduction whose net is an income credit (a supplier
   refund booked on other income), when a larger VAT settlement shares the voucher.
   Flagged and needing an acknowledgement: an untagged refund credited to 2939, a
   payment whose interest and penalties reach the VAT it pays, and an input-VAT
   clearing booked together with interest expense.
2. **Direction for a role without `generated_role` comes from the account kind**
   (liability → output, asset → input). Older and imported snapshots mark 2939 only
   `tagged-only`. This is not a treatment decision; it only says which side accrues.
   Without a known direction, every untagged posting blocks.
3. **Blocking needs untagged postings and a difference beyond rounding.** Tagged
   vouchers routinely differ by a cent from their VAT leg (VAT is computed per line
   from the base: the issue's 1 976,10 € base gives 503,91 € against a 503,90 € leg),
   and existing books and test fixtures carry tags with no VAT leg at all. A
   difference with no untagged postings is an info item, not a refusal. A coded
   voucher whose VAT legs its own codes do not explain (a line whose code was
   forgotten, a code of the other direction next to an uncoded leg) has those legs
   classified untagged; codes with no VAT leg at all stay owned by the codes. With
   untagged postings, a VAT-coded correction in the same period that brings the
   direction back within tolerance reconciles it. The tolerance sums over the
   period's coded lines, so a period with many coded lines tolerates as many cents
   of untagged VAT.
4. **Acknowledgement instead of refusal without override.** Legitimate untagged
   accrual-side postings exist (opening-balance VAT in the first month, a refund
   credited to 2939). The acknowledgement names the exact vouchers and a reason and
   is recorded in the Run, so it cannot carry over silently.
5. **Remedy in the guide differs from the brief's "tag via a correcting entry".**
   The user surface cannot reopen a closed month, and the check runs after the close.
   A VAT-coded correction in the next month would put March's VAT into April's return
   (or report it twice if March is then filed correctly by hand). The guide therefore
   says: the books already hold the VAT; file the month in OmaVero with corrected
   figures (replacement return if already filed), and do not re-record the VAT with a
   code later. The skill tells the agent to tag the base line at draft time, and
   `voucher validate` warns before sealing, which is where a correction is still free.
6. **The VAT code goes on the base line, not the VAT leg.** The tagged path computes
   `base × rate` from the tagged posting; a code on the 2939 leg would compute VAT on
   VAT. The brief's "put a `vat_code` on VAT legs" is implemented as this rule.
7. **Validate warns, seal does not.** `voucher seal` output was left unchanged: the
   agent runs `validate` before asking for approval, and the period check catches the
   rest. Adding the warning to the sealed-voucher record would widen the ledger
   surface for no extra protection.
