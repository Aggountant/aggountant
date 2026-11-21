---
created: 2026-11-21
updated: 2026-11-21
type: bug
status: open
priority: high
provenance: ai-review
source_ref: taskfleet:01m7yedg2ttbhyra7g89krq6t7/review-finding:vat-treatment-unknown-fields
review_source: ai-review
originating_run: 01m7yedg2ttbhyra7g89krq6t7
originating_run_kind: spinoff
labels:
- ai-review-model:gpt-6-astra
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_confidence: high
review_severity: should-fix
review_target: HEAD~1..HEAD
lane: vat-roles
lane_seq: 0
collision: [crates/domain/src/vat/expand.rs]
preparation: ready
---

# VatTreatment accepts unknown fields: a misplaced deductibility on bank import is silently ignored

## Description

## Problem

`aggountant_domain::VatTreatment` (`crates/domain/src/vat/expand.rs`, the
`#[serde(tag = "type", rename_all = "kebab-case")]` enum) does not deny unknown
fields. The bank import reads it as `Assignment.vat_treatment`
(`crates/import/src/lib.rs`) on `import camt053 seal` stdin, so a member that
belongs to another variant is silently dropped instead of refused.

Reproduced with a synthetic object: deserializing

```json
{"type":"domestic-input","rate_class":"domestic-standard",
 "deductibility":{"numerator":1,"denominator":2,
  "provenance":{"source":"documented-use-allocation","reference":"x"}}}
```

as `VatTreatment` gives `Ok(DomesticInput { rate_class: "domestic-standard" })`.

## Impact

An agent that means a 50 % deduction but writes `domestic-input` instead of
`domestic-input-partial` gets full input-VAT deduction sealed, with no error.
That is silently wrong bookkeeping that reaches the VAT return. `Assignment`
itself and every other stdin input deny unknown fields, so the agent has no
reason to expect this.

## Expected

Add `deny_unknown_fields` to `VatTreatment` so the object above is refused
with INVALID_INPUT, plus a regression test. `VatTreatment` is deserialized only
from the import assignment (sealed vouchers keep their expansion in another
type), so no stored data is affected; confirm that before the change. Then
drop the sentence "on the full types a deductibility member is ignored, not
refused" from the `import camt053 seal` INPUT help
(`crates/cli/src/input_help.rs`).

Found by the review of `cli-stdin-input-help`.
