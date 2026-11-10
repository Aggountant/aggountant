---
created: 2026-11-10
updated: 2026-11-10
type: improvement
status: open
priority: normal
lane: vat-calculation
lane_seq: 20
collision: [crates/import, crates/domain/src/vat/code.rs, crates/domain/src/vat/derive.rs, crates/domain/src/vat/expand.rs, crates/domain/src/vat/guards.rs]
---

# Import path: explicit output-VAT expansion for taxable sales

## Problem

The bank-import path (`crates/import`) expands a purchase into explicit legs
through `VatTreatment`: net amount, deductible input VAT and, for reverse
charge, the output VAT. Sales have no such expansion. A domestic sale still goes
through the legacy `vat_code` path as a two-leg posting without an explicit
output-VAT leg, so the import path cannot record a taxable sale in the same
explicit form as a purchase.

A guard already rejects a taxable legacy `vat_code` import that would seal the
gross amount as the VAT base, and `Assignment.document_date` already decides the
VAT rate date. What remains is the sales side and the fate of the legacy path.

## Expected

`VatTreatment` (or its successor) models taxable sales: net revenue, output VAT
to the output-VAT account, and the numbered return field the sale feeds. Once
sales are covered, the legacy `vat_code` import path for taxable entries is
removed rather than kept beside the new one.

## Acceptance criteria

- [ ] A synthetic domestic sale of 125.50 gross imports as 100.00 revenue and 25.50 output VAT and reaches field 301.
- [ ] Reduced-rate sales reach fields 302 and 303.
- [ ] No taxable import goes through the legacy `vat_code` path.
