---
created: 2026-11-21
updated: 2026-11-21
type: feature
status: open
priority: normal
lane: unlaned
collision: [crates/domain/src/vat/expand.rs, docs/user]
---

# Purchases exempt from Finnish VAT from another EU country: guide and hint

## Description

A purchase from another EU member state whose supply is exempt from VAT
(for example insurance, financial or education services) carries no
Finnish VAT and is not reported in the VAT return's EU purchase boxes:
those boxes cover purchases where the buyer owes Finnish VAT under the
reverse charge. Since `capture-domestic-input-eu-class`, a
`reverse-charge` treatment with class `zero-rated` is refused
(`VAT_TREATMENT_CLASS_MISMATCH`) and the hint points at `no-vat`, which is
the correct booking. What is missing is that the documentation says so.

## Wanted

- The VAT treatment concept page (fi/en) names the case: an exempt EU
  purchase is booked with `{"type":"no-vat"}`, it does not appear in the
  return, and why (with a reference to Verohallinto's guidance on
  services trade, checked from the primary source at implementation time).
- The `VAT_TREATMENT_CLASS_MISMATCH` entry in the error-code guide (fi/en)
  and, if it fits, the error message or hint name exempt EU purchases as a
  `no-vat` case alongside the reverse-charge case.
- If the primary source turns out to require reporting exempt EU purchases
  somewhere in the return, stop and escalate instead of documenting `no-vat`.

## Acceptance Criteria

- [ ] Concept page and error-code guide updated in both languages.
- [ ] Any message change covered by the existing tests.
