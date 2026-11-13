# Result: VAT payments with interest are settlements

## What changed

`classify_vat_account_postings` (`crates/domain/src/vat/reconcile.rs`) no longer
requires a settlement voucher to be free of income and expense postings. A
reduction-side untagged VAT posting (output VAT debited, input VAT credited) is a
settlement when the voucher's income and expense postings sum to less than its
untagged reduction-side VAT postings, every account is known to the chart and every
amount is in one currency. The same classifier drives `voucher validate`'s
`VAT_POSTING_UNTAGGED` warning, so the draft-time warning follows.

Tests in `tests_reconcile.rs`: the issue's reproduction (payment with accrued and
expensed interest), a net payment clearing 2939 and 1763 with interest and a
late-filing penalty, a refund received with interest income, and blocking cases for
an untagged sales credit note, an untagged credit-loss VAT reduction, and an untagged
sale booked in the same voucher as a VAT payment. The existing blocking tests are
unchanged and pass.

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

1. **Amount comparison instead of "paired with a bank or tax-account leg".** The
   issue's expected rule names a bank or tax-account leg, but the chart role table
   has no such role and the account kind cannot tell a bank account from a
   receivable. Allowing any expense line next to a reduction would have turned an
   untagged credit-loss VAT reduction (expense debit, output VAT debit, receivable
   credit) into a silent settlement, the same shape as interest. A credit note's or
   credit loss's net is at least 1 / 25,5 % ≈ 3,9 times its VAT, while interest and
   penalties are small beside the VAT paid, so "result lines < reduced VAT" separates
   them without reading account numbers. Alternatives: a bank role in the chart
   (masterdata change in every snapshot), or a voucher-kind marker (model change).
   Known gaps: a payment whose interest and penalties reach the VAT it pays is
   flagged (acknowledge it); a small untagged credit note booked in the same voucher
   as a larger VAT payment counts as a settlement.
2. **Only untagged, non-generated reduction legs count toward the VAT side** of the
   comparison, so a voucher with a VAT expansion cannot let its own net hide behind
   generated legs.
