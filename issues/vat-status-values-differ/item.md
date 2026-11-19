---
created: 2026-11-19
updated: 2026-11-19
type: bug
status: fixed
priority: high
preparation: ready
lane: vat-filing
lane_seq: 1
collision: [crates/broker/src/vero_vat_api.rs, crates/process/src/file_vat/status.rs]
closed: 2026-11-19
closed_by: claude-opus-5-5
---

# file-vat status reports processed_values_differ without saying which values, and the comparison is structural

## Description

`file-vat status --period` answers `processed_values_differ` ("values differ
from the approved request. Compare it in OmaVero …") but does not say which
values differ. In production use this answer came back for two consecutive
monthly returns that were filed through aggountant, accepted with a receipt
and processed. Two independent periods both differing is more likely a
comparison defect than two wrong returns, but the output gives no way to tell.

The comparison (`GetFiledVatReturn::semantically_matches_attempt`,
`crates/broker/src/vero_vat_api.rs`) builds two JSON objects from four fields
(`VATOnDomesticSales[ByTaxRate]`, `VATOnPurchasesAndImports`, `DeductibleVAT`,
`SalesPurchasesImports`), normalises numbers and requires exact equality. Any
structural difference between the request and GetFiledVATReturn/v2 makes it
fail: for example extra zero-valued rate rows or keys in the authority's
answer, keys present with zero in one and omitted in the other, different key
names inside the rate array, array order, or strings vs numbers that
`normalize_numbers` does not touch (it only normalises JSON numbers, not
numeric strings).

Example (Esimerkki Oy, fictional): October return with domestic sales at 25.5 %
and one purchase deduction is filed and accepted; `status` reports
`processed_values_differ` although OmaVero shows the same amounts.

## Wanted

1. `status` (and `reconcile` where it uses the same comparison) lists the
   differing fields: for each, the approved value and the authority's value,
   as JSON (`differences: [{field, approved, authority}]`) and in the human
   line. Amounts only, no personal data.
2. Check the comparison against the GetFiledVATReturn/v2 schema and the
   FileVATReturn request schema (the Vero API documentation the broker already
   follows). Compare semantically: a missing field equals zero, zero-valued
   rate rows are ignored, rate rows match by rate regardless of order, numeric
   strings and numbers compare as decimals. A real difference in any amount
   still reports `processed_values_differ`.
3. Tests with recorded synthetic authority answers covering each structural
   variant above, plus one genuine amount difference.

## Resolution

### 2026-11-19T12:59:46Z · @claude-opus-5-5

Comparison is now by amount against the GetFiledVATReturn/v2 schema, with differences listed in status and reconcile; see result.md.
