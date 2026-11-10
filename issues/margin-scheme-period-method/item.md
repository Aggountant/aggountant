---
created: 2026-11-10
updated: 2026-11-10
type: feature
status: open
priority: normal
lane: vat-calculation
lane_seq: 40
collision: [crates/domain/src/vat/code.rs, crates/domain/src/vat/derive.rs]
related: ['@margin-scheme-model']
---

# VAT margin scheme: per-period method with carry-forward

## Problem

The margin scheme is supported only with the per-item method. The VAT Act
(arvonlisäverolaki 79 j §) also allows a simplified per-period method: the
reseller nets the period's margin-scheme purchases against its margin-scheme
sales, computes VAT on the net period margin, and carries a negative period
margin forward to the next period (a negative per-item margin is not creditable;
a negative period margin is).

This does not fit the per-voucher, stateless `derive_vat_figures` path, because
it needs aggregation of margin-scheme purchases across vouchers (purchases
carry no deductible VAT and are untagged today) and persisted carry-forward
state between periods.

## Expected

A per-period method the company elects explicitly, with margin-scheme purchases
tagged, the period margin computed at close, and a negative period margin
carried into the next period's computation. Builds on `margin-scheme-model`.

## Acceptance criteria

- [ ] Primary-source citation for the method and its carry-forward rule in the plan.
- [ ] A synthetic two-month case: month 1 negative period margin, month 2 positive; month 2's VAT reflects the carry-forward.
