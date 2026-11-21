---
created: 2026-11-21
updated: 2026-11-21
type: bug
status: untriaged
priority: high
provenance: ai-review
source_ref: taskfleet:01m7z96fkrs15d0vqv8af8waxf/review-finding:receipt-vat-proposal-no-vat-members
review_source: ai-review
originating_run: 01m7z96fkrs15d0vqv8af8waxf
originating_run_kind: spinoff
labels:
- ai-review-model:claude-fable-5-2
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_confidence: high
review_severity: should-fix
review_target: HEAD~1..HEAD
---

# ReceiptVatProposal no-vat silently drops rate_class/scope members on capture-receipt stdin

## Description

## Problem

`ReceiptVatProposal` (`crates/process/src/capture_receipt.rs`, the
`#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]` enum
read from `capture-receipt` stdin) has a unit variant `NoVat`. Serde's
container-level `deny_unknown_fields` does not reach a unit variant of an
internally tagged enum, so members on `no-vat` are silently dropped.

Synthetic reproduction (verified against the real type during the review of
`vat-treatment-deny-unknown-fields`):

```json
{"type":"no-vat","rate_class":"domestic-standard","scope":"eu-goods"}
```

deserializes as `ReceiptVatProposal::NoVat` and maps to `VatTreatment::NoVat`
(`capture_receipt.rs`, the proposal-to-treatment mapping), so the receipt is
sealed with no VAT leg and no error.

## Impact

An agent that writes `no-vat` while giving a rate class or scope signals a
contradiction. It should be refused, not sealed as no VAT. The input VAT
deduction is then silently lost from the VAT return. `VatTreatment` itself was
closed against the same gap (`vat-treatment-deny-unknown-fields`), so the two
stdin paths now behave differently.

## Expected

Refuse members on `ReceiptVatProposal::NoVat` with INVALID_INPUT, the same way
`VatTreatment::NoVat` does through its variant-level
`#[serde(deserialize_with = "deny_members")]` in
`crates/domain/src/vat/expand.rs`. Give the helper one owner (make it public
in `domain` or move it to `model`) rather than copying it. Add a regression
test. `VatExpansionTreatment::NoVat` (`crates/model/src/vat_expansion.rs`) has
the same shape, but it is read from stored vouchers, so tightening it needs a
check that no stored record carries surplus members. Leave it out unless that
check is done.
