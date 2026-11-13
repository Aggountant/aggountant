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
lane_seq: 5
collision: [crates/process/src/file_vat.rs, crates/broker/src/vero_vat_api.rs, crates/process/src/file_vat/status.rs]
---

# A FileVATReturn 2xx without a readable receipt is recorded as a rejection

## What goes wrong

When aggountant files a VAT return and the Tax Administration answers with a
success status, but the answer cannot be read as a receipt, aggountant records
the filing as **rejected by the authority**. In fact the return may well have
been accepted. The operator sees a rejection, the period stays unfiled in the
books, and nothing tells them to find out what actually happened. The product
rule for an uncertain filing is the opposite: record it as `OUTCOME_UNKNOWN` and
resolve it by asking the authority (`file-vat reconcile`, `file-vat status`),
never by sending it again.

It does not take a broken answer to trigger this. `FileVatReceipt`
(`crates/broker/src/vero_vat_api.rs`) is `deny_unknown_fields`. If the Tax
Administration adds one field to its receipt response, every successful filing
from then on is recorded as a rejection.

## Where in the code

- Both FileVATReturn adapters in `vero_vat_api.rs`, the live
  `VeroFileVatReturnAdapter` and the `ScriptedVeroFileVatReturnAdapter` that
  tests drive, handle the `200` arm the same way. A body that does not
  deserialize as `FileVatReceipt`, or that has an empty `UniqueIdentifier` or
  `AcceptedTimestamp`, becomes `AdapterError::Malformed`. Any other 2xx falls
  to the `other =>` arm as `AdapterError::Rejected { code: "HTTP_2xx" }`.
- `Broker::submit` (`crates/broker/src/broker.rs`) turns `Malformed` into
  `AttemptOutcome::Failed { code: "MALFORMED" }` and
  `RemoteOutcome::Rejected`. `run_submit_filing`
  (`crates/process/src/file_vat.rs`) then journals `EffectRejected` with
  `MALFORMED: …` or `HTTP_2xx: …`, and fiscal `vat_filed` stays false.
- Fix this in the adapters, not in the broker's `Malformed` arm. The adapters
  also return `Malformed` before anything is sent (for example a missing
  `request_bytes`), and there "failed" is the truth.
- `period_has_unresolved_unknown_filing` (`file_vat.rs`) only recognises
  `OUTCOME_UNKNOWN`, so `file-vat complete-manual` is not held back after a
  receiptless 200 the way it is after a timeout.
- The read side already knows better. `file-vat status`
  (`crates/process/src/file_vat/status.rs`, `outcome_may_be_accepted`) reports
  `MALFORMED` and `HTTP_2…` filing rejections as `outcome_unknown` and points to
  `file-vat reconcile`, which accepts any parked filing effect. Runs journalled
  before this fix keep those codes, so that read-side rule stays.

## The fix

A 2xx answer from FileVATReturn is never a rejection. If it carries both receipt
fields, the filing was delivered. That includes an answer with extra fields,
so `FileVatReceipt` should stop denying unknown fields; `encode()` still
stores only the two fields we use. Anything else in the 2xx range (fields
missing or empty, an unparseable body, a non-200 2xx) is
`AdapterError::AmbiguousDelivery`, which the broker records as `Unknown` and the
process journals as `OUTCOME_UNKNOWN`. Make the live and scripted adapters agree.

The `complete-manual` guard should treat an older journalled `MALFORMED` or
`HTTP_2…` filing rejection as unresolved too. One predicate shared with
`status.rs` is the simplest way to keep the two from drifting apart.

## Acceptance

- A 200 with both receipt fields plus an extra field is delivered: the period
  becomes `vat_filed` with that receipt.
- A 200 with a missing or empty receipt field, a 200 with a non-JSON body, and a
  2xx other than 200 each leave the filing effect parked as `OUTCOME_UNKNOWN`,
  with `vat_filed` false, in both adapters.
- After such an answer, `file-vat complete-manual` for the period is refused
  until remote-status evidence is given. The same holds for a Run journalled
  with `MALFORMED` before this change.
- 400, 401/403 and 5xx keep their current meaning.
- Tests cover this at the broker level. `vero-vat-api-mock`'s
  `FileVatReturnFixture::Receipt` with an empty `unique_identifier` gives the
  missing-field case. The scripted adapter takes arbitrary JSON, so it gives the
  extra-field and non-200 cases.

## What this fix does not cover

The original finding also said a new `file-vat` Run would send the return
again. That is true, but it is not specific to this case. Re-checked on
2026-11-13: a new Run for the same period also re-sends after a genuine
`OUTCOME_UNKNOWN`, because the broker deliberately does not short-circuit an
`Unknown` attempt and no Run checks another Run's unresolved filing. That needs
a decision on when re-filing becomes allowed, so it is tracked separately in
`vat-filing-resent-by-new-run-after-unknown`. This issue makes the receiptless
2xx land in the `OUTCOME_UNKNOWN` state; that issue makes the state hold across
Runs.

Found by the verifier in the `/llm-review` of `vat-filed-status` (finding F3 in
`issues/vat-filed-status/assessment.md`).
