---
created: 2026-11-13
updated: 2026-11-13
type: bug
status: untriaged
priority: normal
provenance: ai-review
source_ref: taskfleet:01m7a1vmb1vv3rhg8nx3dx98sq/review-finding:verifier-unlisted-settlement-gaps
review_source: ai-review
originating_run: 01m7a1vmb1vv3rhg8nx3dx98sq
originating_run_kind: spinoff
labels:
- ai-review-model:claude-fable-5-2
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_confidence: high
review_severity: should-fix
review_target: f7ace2b..HEAD (vat-settlement-with-interest-flagged)
---

# VAT settlement detection by voucher shape hides some untagged VAT reductions

## Description

## Problem

The period VAT reconciliation (`classify_vat_account_postings` and `settles_vat` in
`crates/domain/src/vat/reconcile.rs`) recognises a VAT settlement (payment, refund or
clearing, excluded from the comparison) by voucher shape alone. The chart has no role
that marks a bank or tax-account leg, so some untagged VAT reductions that belong in
the return are classified `settlement` and reconcile silently. `file-vat start` then
files a return that leaves them out, with no attention item.

Untagged reduction-side postings counted as settlements today:

1. A voucher with no income or expense posting: an advance-payment refund
   (2620 D 100,00 / 2939 D 25,50 / 1910 C 125,50), a fixed-asset purchase return
   (2700 D 1 255,00 / 1150 C 1 000,00 / 1763 C 255,00), a private-use input-VAT
   correction through equity (2000 D 25,50 / 1763 C 25,50), an asset-sale credit note.
   This shape predates `vat-settlement-with-interest-flagged`.
2. Since `vat-settlement-with-interest-flagged`, when a larger VAT settlement shares
   the voucher (for example a cash-basis bank-statement voucher): an output-VAT
   reduction whose net is an expense debit (a customer refund or credit loss booked on
   an expense account), and an input-VAT reduction whose net is an income credit (a
   supplier refund booked on other income). Example, Esimerkki Oy: 2939 D 500,00 /
   7780 (expense) D 100,00 / 2939 D 25,50 / 1910 C 625,50 reconciles; the return
   keeps 25,50 € of output VAT that the refund reduced.

The known-gaps lists in `issues/vat-untagged-posting-omitted/result.md` (decision 1)
and `issues/vat-settlement-with-interest-flagged/result.md` record these.

## Possible fix

Mark settlement evidence explicitly instead of inferring it from shape: a chart role
for bank and tax-account (verotili) accounts, so that a settlement needs its
reduction paired with such a leg, or a settlement marker on the voucher. Either is a
masterdata or model change across ledger, process and every chart snapshot.

Found by the focused review of `vat-settlement-with-interest-flagged` (verifier
question on unlisted silent omissions).
