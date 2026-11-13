# Result: VAT payments with interest are settlements

## What changed

`classify_vat_account_postings` (`crates/domain/src/vat/reconcile.rs`) no longer
requires a settlement voucher to be free of income and expense postings. A
reduction-side untagged VAT posting is a settlement when the voucher's income and
expense postings sum to less than its untagged reduction-side VAT postings and are
all of the kind a settlement of that direction books: expense debits (interest,
penalties) beside an output-VAT reduction, income credits (refund interest) beside
an input-VAT reduction. Every account must be known to the chart and every amount
in one currency. The same classifier drives `voucher validate`'s
`VAT_POSTING_UNTAGGED` warning, so the draft-time warning follows.

Tests in `tests_reconcile.rs`: settlements for the issue's reproduction (payment
with accrued and expensed interest), a payment with interest and a late-filing
penalty, and a refund received with interest income. Blocking for an untagged sales
credit note, a credit-loss VAT reduction, an input-VAT deduction correction in a
clearing voucher, a sale netted into the payment line, and an untagged sale,
supplier credit note or customer refund booked beside a VAT payment. A net payment
that also clears 1763 with interest flags its 1763 leg (pinned). The existing
blocking tests are unchanged and pass.

User guide (fi + en, `tasks/file-vat-omavero.md`): payments, refunds and transfers
stay out of the comparison also when the voucher books interest or a late fee. The
known-gaps list in `issues/vat-untagged-posting-omitted/result.md` is updated.

## Simplicity

- No new data or signature: the rule uses the chart account kinds the classifier
  already receives. Process and ledger call sites are untouched.
- One helper (`settles_vat`) replaces the former `touches_result` flag; the
  reduction-side test is stated once and used for both the sum and the class.
- Deliberately left out: a bank / tax-account role in the chart, and a settlement
  marker on the voucher.

## Decisions for later review

1. **Amount and kind tests instead of "paired with a bank or tax-account leg".**
   The issue's expected rule names a bank or tax-account leg, but the chart role
   table has no such role and the account kind cannot tell a bank account from a
   receivable. Allowing any expense line next to a reduction would have turned an
   untagged credit-loss VAT reduction (expense debit, output VAT debit, receivable
   credit) into a silent settlement, the same shape as interest. A credit note's or
   credit loss's net is at least 1 / 25,5 % ≈ 3,9 times its VAT, while interest and
   penalties are small beside the VAT paid, so "result lines < reduced VAT"
   separates them on their own. The amount test is voucher-wide, so a credit note,
   deduction correction or netted sale sharing a voucher with a larger payment would
   pass it; the kind test closes most of that: output-VAT reductions tolerate only
   expense debits and input-VAT reductions only income credits. Both were found by
   the review (`review.md`). Alternatives: a bank role in the chart (masterdata
   change in every snapshot), or a voucher-kind marker (model change); filed as
   `vat-settlement-shape-hides-untagged-reductions`.
   Known gaps, when a larger VAT settlement shares the voucher: an untagged
   output-VAT reduction whose net is an expense debit (credit loss, customer refund
   booked on an expense) or an input-VAT reduction whose net is an income credit
   (supplier refund booked on other income) counts as a settlement. Flagged and
   acknowledged instead: a payment whose interest and penalties reach the VAT it
   pays, and a net payment clearing 1763 together with interest expense.
2. **Only untagged, non-generated reduction legs count toward the VAT side** of the
   comparison, so a voucher with a VAT expansion cannot let its own net hide behind
   generated legs.
