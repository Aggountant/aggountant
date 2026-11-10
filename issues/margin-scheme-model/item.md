---
created: 2026-11-10
updated: 2026-11-10
type: feature
status: open
priority: normal
lane: vat-calculation
lane_seq: 30
collision: [crates/domain/src/vat/code.rs, crates/domain/src/vat/derive.rs]
related: ['@margin-scheme-period-method']
---

# VAT margin scheme: structured on-ledger model (prices, item identity, eligibility)

## Problem

The VAT margin scheme for second-hand goods is supported per item:
`sales-margin-<category>` codes, VAT-inclusive extraction in
`derive_vat_figures`, and `compute_margin_vat`, which applies the rule that a
negative item margin carries no VAT. The model behind it overloads
`Posting.amount`: for a normal code the tagged amount is the VAT-exclusive base,
for a margin code it is the VAT-inclusive profit margin. The meaning is selected
by a prefix in an untyped `vat_code`. Consequences:

1. **Silent over-declaration.** If the full selling price is tagged under a
   margin code, VAT is extracted from the whole price (5 000.00 × 25.5/125.5 ≈
   1 015.94 instead of 255.00 on a 1 255.00 margin) with nothing detecting it.
2. **`compute_margin_vat` is not linked to sealing.** Nothing forces a voucher to
   use its output; any code plus amount is accepted by `build_draft`.
3. **No item identity.** A correction that drives an item's margin below zero
   should cap the VAT reversal at the item's original VAT, which needs a stable
   item identifier linking purchase, resale and adjustment.
4. **Eligibility is asserted, not established.** A category string does not
   record why the item qualifies for the scheme (acquisition source, whether
   input VAT was deducted).

## Proposed direction

A typed margin field on the posting (or a structured margin-trade input) that
keeps both prices on the ledger, so the margin is computed rather than
asserted: for example `VatKind::Margin { category, item_id, selling_price,
purchase_price, eligibility_ref }`. The voucher then records the real selling
price, `derive_vat_figures` sums structured VAT lines instead of reinterpreting
an amount by string prefix, and the margin base can reach the return's
numbered fields. The per-period method (`margin-scheme-period-method`) becomes
an explicit election.

Not in scope: rejecting `sales-margin-*` in the meantime (the per-item path is
correct for positive margins and credit notes), and clamping derived amounts to
non-negative per posting (that would break margin credit notes).

## Acceptance criteria

- [ ] Margin vouchers carry selling and purchase price; the margin and its VAT are computed at seal time.
- [ ] Tagging a full selling price as a margin is impossible or detected.
- [ ] Corrections are capped per item.
