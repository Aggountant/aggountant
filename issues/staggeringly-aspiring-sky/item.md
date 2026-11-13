---
created: 2026-11-13
updated: 2026-11-13
type: bug
status: open
priority: high
provenance: ai-review
source_ref: taskfleet:01m79z4hcfcrhvd8c8jtkffejr/review-finding:receiptless-200-definitive-rejection
review_source: ai-review
originating_run: 01m79z4hcfcrhvd8c8jtkffejr
originating_run_kind: spinoff
review_target: 66c232f..HEAD (vat-filed-status)
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_severity: must-fix
review_confidence: high
labels:
- ai-review-model:claude-fable-5-2
lane: vat-filing
lane_seq: 6
collision: [crates/process/src/file_vat.rs, crates/broker/src/vero_vat_api.rs]
---

# A FileVATReturn 200 without a readable receipt is recorded as a definitive rejection

## Description

## Problem

`VeroFileVatReturnAdapter::call` (`crates/broker/src/vero_vat_api.rs`, the
`200 =>` arm) maps an HTTP 200 whose body does not deserialize as a complete
`FileVatReceipt` to `AdapterError::Malformed`. `Broker::submit`
(`crates/broker/src/broker.rs`, the `Err(AdapterError::Malformed(..))` arm)
records that as `AttemptOutcome::Failed { code: "MALFORMED" }` and returns
`RemoteOutcome::Rejected`, i.e. a definitive negative. Any other 2xx falls to
`other => Rejected { code: "HTTP_2xx" }`. The scripted adapter behaves the same.

The authority answered with its success status, so the return may have been
accepted. `FileVatReceipt` is `deny_unknown_fields`, so a single extra field
added to the receipt response by the Tax Administration is enough to trigger
this.

## Effect

- `run_submit_filing` journals `EffectRejected` with `MALFORMED: ...`; fiscal
  `vat_filed` stays false.
- The outbox holds only a `Failed` attempt, so a new `file-vat` Run with the
  same bytes dispatches FileVATReturn again: a possible duplicate filing.
- `period_has_unresolved_unknown_filing` (`crates/process/src/file_vat.rs`)
  recognises only `OUTCOME_UNKNOWN`, so `file-vat complete-manual` is not held
  back either.

`file-vat status` (issue vat-filed-status) already treats `MALFORMED` and
`HTTP_2xx` filing rejections as `outcome_unknown` and points to
`file-vat reconcile`, which accepts any rejected filing effect. The filing path
itself still treats them as definitive.

## Suggested fix

Return `AdapterError::AmbiguousDelivery` (journalled as `OUTCOME_UNKNOWN`) for
a 2xx without a valid receipt in both the live and scripted FileVATReturn
adapters, and consider relaxing `deny_unknown_fields` on the receipt so an
added field does not turn an accepted filing into an uncertain one. A broker
test with the mock (`FileVatReturnFixture::Receipt` with an empty
`unique_identifier`, or an extra field) pins it.

Found by the verifier in the /llm-review of vat-filed-status (taskfleet run
01m79z4hcfcrhvd8c8jtkffejr, 2026-11-13), confirmed against the code.
