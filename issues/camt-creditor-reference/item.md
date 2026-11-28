---
created: 2026-11-28
updated: 2026-11-28
type: improvement
status: open
priority: normal
lane: bank-import
lane_seq: 20
collision: [crates/import]
---

# camt import: read the structured creditor reference (viitenumero)

## Problem

`import camt` reads the free-text remittance (`RmtInf/Ustrd`) but not the
structured creditor reference (`RmtInf/Strd/CdtrRefInf/Ref`, Finnish
viitenumero or RF reference). Finnish banks carry it on many incoming and
outgoing payments, and every Finnish bookkeeping program uses it as the first
key for matching a payment to an invoice, a tax payment or an expense claim.
The product already has a validated reference type (`PaymentReference`,
`crates/model/src/payment_reference.rs`) for tax payments, but a statement
cannot tell the agent which reference a payment carried.

## Expected

- `import camt parse` (and the preview) show a typed `creditor_reference`
  per transaction when the statement carries one: the raw value, its kind
  (domestic, RF, other) and whether the check digit is valid. An invalid or
  unknown reference is shown, never refused or dropped.
- Passthrough rule, as in `camt-rich-fields`: the reference does not enter
  the transaction key, the source fingerprint, the default description or the
  posting projection. Statements imported by an earlier build re-import as
  duplicate observations, never `SOURCE_FACTS_CHANGED` or
  `POSTING_PROJECTION_CHANGED`.
- When an outgoing payment's reference equals the company's recorded
  self-assessed tax reference (`tax-payments show`), the import output marks
  it as a recognised tax payment (for example
  `tax_reference_match: self_assessed`), so the skill proposes the tax-account
  posting instead of guessing. The reference does not identify the period;
  the output says so.

## Acceptance criteria

- [ ] Synthetic fixture with a domestic reference, an RF reference, a
      zero-padded reference and one with a wrong check digit; golden output.
- [ ] Identity test: the same fixture with and without `Strd` gives the same
      key, fingerprint and description; re-import over a home sealed by the
      previous build is a duplicate observation.
- [ ] A file without the element parses exactly as before (existing test).
- [ ] Tax-reference recognition tested with a home that has and has not
      recorded tax payment details; homes without them see no change.
- [ ] Guide (fi/en) bank-statement page and the capture-receipt skill mention
      the field.
- [ ] No store migration.

## Source

Follow-up research on open items between period closes (private research,
2026-11-28): the reader skips the structured reference although real
statements carry it. Prerequisite for reference-based matching of open items
later.

## Decisions

### 2026-11-28T06:09:41Z · @conductor

Product owner 2026-11-28 asked to develop the open-items report; reading the structured reference is the research's independent first step. Accepted for execution.
