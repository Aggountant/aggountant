---
created: 2026-11-21
updated: 2026-11-21
type: feature
status: untriaged
priority: normal
---

# Exempt intra-Community goods acquisitions: report the value with zero tax

## Description

Found while implementing `eu-exempt-purchase-guide`. Verohallinto's VAT
return instructions say that an intra-Community acquisition of goods whose
supply is exempt in Finland is still reported under "Tavaraostot muista
EU-maista" (with no tax). No aggountant treatment models that: `no-vat`
leaves the purchase out of the return, and `reverse-charge` with class
`zero-rated` is refused (`VAT_TREATMENT_CLASS_MISMATCH`). The user guide now
says the case is unsupported and points to the bookkeeper.

## Wanted

A way to book an exempt intra-Community goods acquisition so that its value
reaches the EU goods purchase box with zero tax, and the guide and error
message updated to match. Confirm the reporting rule from the primary
source at implementation time, and decide whether this is a new treatment,
a scope on an existing one, or a class, from what the return derivation
needs.

## Acceptance Criteria

- [ ] Such a purchase can be recorded through capture-receipt and the bank import.
- [ ] It appears in the EU goods purchase box with zero tax in `file-vat preview`.
- [ ] Guide (fi/en) and error message describe it.
