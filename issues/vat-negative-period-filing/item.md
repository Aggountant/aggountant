---
created: 2026-11-10
updated: 2026-11-10
type: improvement
status: open
priority: normal
lane: vat-filing
lane_seq: 20
collision: [crates/domain/src/vat/filing.rs, crates/domain/src/vat/derive.rs]
---

# Correction-dominated period: negative derived VAT is rejected by the filing

## Problem

`DerivedVatFigures` documents that a correction-dominated period (for example a
month whose only sales entry is a credit note) may derive a negative
`sales_vat` or `purchase_vat`. `compute_vat_filing`
(`crates/domain/src/vat/filing.rs`) rejects any negative input, and reports the
sign error under the unrelated `VAT_CURRENCY` code. The type contract and the
filing API disagree, so such a period cannot be filed and the error points the
user at currency.

## Expected

A decision on how a correction-dominated period is represented in the return,
implemented in `compute_vat_filing`, and a sign-specific error code for any
input that remains invalid.

## Acceptance criteria

- [ ] A period whose derived sales or purchase VAT is negative either files correctly or fails with a sign-specific error code.
- [ ] `VAT_CURRENCY` is used only for currency problems.
- [ ] Tests cover a credit-note-only period.
