---
created: 2026-11-21
updated: 2026-11-21
type: bug
status: open
priority: high
provenance: ai-review
source_ref: taskfleet:01m7yh5d4b1ky22txvkc6jakbk/review-finding:F1-domestic-input-eu-class
review_source: ai-review
originating_run: 01m7yh5d4b1ky22txvkc6jakbk
originating_run_kind: spinoff
review_target: filing-output-markers c90d0bf
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_severity: should-fix
review_confidence: medium-high
labels:
- ai-review-model:claude-fable-5-2
lane: vat-roles
lane_seq: 1
collision: [crates/domain/src/vat/expand.rs]
preparation: ready
---

# capture-receipt accepts domestic-input with an EU or zero-rated class and seals it at 0 % VAT

## Description

## Description

`capture-receipt record` accepts a `domestic-input` (or `reverse-charge`)
proposal with any class in the dated rate table, including classes whose rate
is 0 % and which belong to another treatment. Reproduced with a synthetic home
(`aggountant init`, Esimerkki Oy defaults):

```json
{"natural_key":"k1",
 "source":{"amount":{"amount":"10.00","currency":"EUR"},"document_date":"2026-07-15","description":"x"},
 "proposal":{"posting_date":"2026-07-16","net_account":"7690","contra_account":"1910",
   "vat":{"type":"domestic-input","rate_class":"eu-goods"}}}
```

`capture-receipt record --dry-run` returns `projection_valid: true`,
`ledger_preflight_valid: true`, `resolved_percent: "0.0"` and
`generated_vat_legs: []`; the voucher would seal as a domestic purchase with
no VAT. The same holds for `zero-rated` and `eu-services`.

## Why it matters

An EU goods or services purchase is reverse-charge: self-assessed output VAT
and the matching input VAT must reach the return. Sealed as `domestic-input`
with class `eu-goods`, the voucher carries no VAT legs, so the purchase is
silently absent from the reverse-charge boxes. VAT treatment is fixed at seal,
so the error is only corrected by a new voucher, and nothing flags it.

## Wanted

The Domain expansion (`crates/domain/src/vat/expand.rs`,
`vat_expand_with_role_table`) refuses a treatment/class pair that does not
belong together (at least `domestic-input` with `eu-goods`, `eu-services` or
`zero-rated`), with a specific code, rather than resolving it to 0 %. Decide
which classes each treatment admits from the rate table's meaning, not by
filtering in `process`.

Found while reviewing `filing-output-markers`, whose new
`CAPTURE_VAT_CLASS_UNKNOWN` message lists the known classes; that list now
says "known classes" and does not instruct the agent to pick from it.
