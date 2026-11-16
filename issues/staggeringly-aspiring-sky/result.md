# Result: staggeringly-aspiring-sky

A FileVATReturn answer in the 2xx range is never a rejection any more.

- `crates/broker/src/vero_vat_api.rs`: both the live and the scripted adapter
  send every 2xx through `file_vat_return_success`. A 200 whose body is a JSON
  object with non-blank `UniqueIdentifier` and `AcceptedTimestamp` is
  delivered; anything else is `AmbiguousDelivery` with cause
  `HTTP_<status>_WITHOUT_RECEIPT`, which the broker records as `Unknown` and the
  Run journals as `OUTCOME_UNKNOWN`. `FileVatReceipt` accepts unknown fields;
  `encode()` still stores only the two receipt fields.
- `crates/process/src/file_vat.rs`: `filing_outcome_may_be_accepted` is the one
  rule for "this filing may have been accepted", used by `file-vat status` and
  the `complete-manual` guard. It keeps the older `MALFORMED`/`HTTP_2` codes.

## Acceptance

- 200 with both fields plus an extra field is delivered and the period becomes
  `vat_filed` with that receipt: `live_receipt_with_an_extra_field_is_delivered`,
  `scripted_adapter_classifies_success_statuses_like_the_live_one`
  (`crates/broker/tests/vero_vat_api.rs`), `a_receipt_with_an_extra_field_files_the_period`
  (`crates/process/tests/file_vat.rs`).
- Missing or empty field, non-JSON or non-object body, and a non-200 2xx park as
  `Unknown` with one attempt, in both adapters:
  `live_success_status_without_a_receipt_is_unknown_not_rejected` (mock
  `Receipt` with an empty `unique_identifier` / `accepted_timestamp`, and
  `ServerError` fixtures with 200/202 bodies) and the scripted test (also 204).
  At Run level the effect is `OUTCOME_UNKNOWN` with `vat_filed` false:
  `a_success_status_without_a_receipt_parks_unknown_and_blocks_manual_completion`.
- `complete-manual` is refused until remote-status evidence is given, after such
  an answer (same test) and for a Run journalled with `MALFORMED` or `HTTP_204`
  before this change (`crates/process/src/file_vat/tests.rs`, in-crate because
  the journal is crate-private). A stated rejection still does not block it.
- 400, 401/403 and 5xx unchanged: the existing
  `structured_400_is_rejected_while_timeout_and_5xx_are_unknown` and
  `scripted_adapter_keeps_the_meaning_of_error_statuses`.

Gate: `scripts/ci-local.sh` green; `scripts/check-user-guide.sh` green. Review:
`review.md`, `assessment.md` beside this file.

User guide: `concepts/filings.md` and `troubleshooting.md` (fi + en) name a
success answer without a receipt as an uncertain outcome. The `vero-fi` skill
says the same and tells the agent that older Runs show it as `MALFORMED`/`HTTP_2xx`.

## Decisions for later review

- **A current `MALFORMED` also holds back `complete-manual`.** A 400 with an
  unstructured body and a request that could not be built still journal
  `MALFORMED: …`, which the shared prefix rule cannot tell from an older
  receiptless 200. Chose to treat all of them as possibly accepted (the operator
  then supplies `--remote-status-evidence-ref`), as `file-vat status` already
  did. Alternative: match the older 200 message text exactly; rejected because
  journals from before the public history may carry other texts, and a wrong
  match would let a manual filing through after a delivered one. Revisit when
  the next issue reuses the predicate to refuse a new Run: there a
  pre-send `MALFORMED` blocking re-filing would be a real cost, and the adapter
  could journal those under a distinct code.
- **A 2xx other than 200 is unknown even with a full receipt.** The issue says
  only a 200 proves delivery; the broker's AGENTS rule agrees. Kept.
- **The `vero-fi` skill was updated** although it is outside the brief's file
  list, because it told the agent that only a timeout or 5xx leaves
  `OUTCOME_UNKNOWN`. One paragraph; no wage text touched.
