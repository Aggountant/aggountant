---
created: 2026-11-13
updated: 2026-11-13
type: bug
status: in-progress
priority: high
provenance: other
provenance_detail: conductor audit of a real company's filed periods with the new reconciliation
source_ref: conductor:2026-11-13/vat-settlement-with-interest
lane: vat-calculation
lane_seq: 0
collision: [crates/domain/src/vat/code.rs, crates/domain/src/vat/guards.rs, crates/domain/src/vat/reconcile.rs]
preparation: ready
---

# A VAT payment that also books interest is taken for untagged output VAT

## Description

## Problem

The VAT reconciliation added by `vat-untagged-posting-omitted` recognises a
VAT settlement (payment of a previous period's VAT) by voucher shape: a
reduction-side untagged posting on a VAT account in a voucher with no income
or expense account. A VAT payment that also books late-payment interest has an
expense line, so it is not recognised as a settlement. Its debit on the
output-VAT account is classified as untagged output VAT, and `file-vat preview`
reports `VAT_ACCOUNTS_UNRECONCILED` (blocking) for a period whose return is
correct. `file-vat start` would refuse until the user acknowledges the voucher.

## Reproduction (synthetic, Esimerkki Oy)

One voucher in a sealed month: 2939 debit 500,00 € (previous period's VAT),
2740 debit 2,00 € (accrued interest), 8350 debit 0,50 € (interest expense),
1910 credit 502,50 €. `file-vat preview` for the month: output side
`untagged` -500,00, `difference` -500,00, status `unreconciled`, the voucher
named in a blocking attention item. Without the two interest lines the same
payment is a settlement and the month reconciles.

## Expected

A debit (reduction side) on the output-VAT account, or a credit on the
input-VAT account, paired with a bank or tax-account leg is a settlement even
when the same voucher books interest or penalties on expense/accrual accounts.
The fix keeps the rule that VAT treatment is never inferred from an account
number; it only refines which untagged reduction postings count as
settlements. The known-gaps list in the issue's `result.md` is updated.

## Acceptance criteria

- [ ] The reproduction reconciles; a test covers a payment with interest and penalty lines.
- [ ] An untagged sale (accrual side) in a voucher with income lines is still blocking.
