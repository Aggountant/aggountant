# Review of staggeringly-aspiring-sky

Focused `/llm-review` of the implementation diff (base `53a90f1`).

- Finder: `gpt-6-astra` (`--task review`, diff attached, issue, `broker.rs` and
  `crates/broker/AGENTS.md` attached). One call.
- Verifier: `claude-fable-5-2`, one call. The finder returned no findings, and
  the surface is high-risk (classification of an authority filing's outcome),
  so the verifier got one focused false-negative question about that boundary
  rather than a list to refute.
- Rigor: routine. Risk class: authority filing / data integrity.

## Finder

"No findings." It read the adapter, broker, journal, manual-completion, status
and recovery paths.

## Verifier

Asked whether any 100–599 status or body (a) records Delivered without both
non-blank receipt fields from a 200, (b) records a definitive rejection for a
2xx, (c) makes the broker resend within one submit, and whether anything
downstream behaves worse for `OUTCOME_UNKNOWN` than for the old codes.

- (b), (c), downstream: no findings, with evidence. Only `RetryableTransport`
  and `RateLimited` loop in `Broker::submit`; neither is produced by an answered
  filing. The fold ignores the error code, `reconcile_file_vat` needs only a
  `Rejected` effect, `filing_resume_safe` never re-drives a `Rejected` filing,
  and `retry_file_vat_submit` refuses the filing unconditionally.

### V1 (should-fix): a 200 whose body is a two-element JSON array is Delivered

`crates/broker/src/vero_vat_api.rs`, `file_vat_return_success`. Serde's derived
struct `Deserialize` also accepts a positional sequence, so
`["id","timestamp"]` became a receipt although neither named field was present.
Pre-existing (the old code parsed the same way); no documented Vero answer has
this shape. Confirmed locally by adding the case to
`receiptless_success_answers()`.

### V2 (should-fix): predicate comment says `MALFORMED` is legacy-only

`crates/process/src/file_vat.rs`, `filing_outcome_may_be_accepted`. `MALFORMED`
is still journalled for a 400 with an unstructured body and for a request that
cannot be built, so since this change `complete-manual` asks for remote-status
evidence after those too. The direction is safe (it over-blocks); the comment
was inaccurate. Confirmed by reading the 400 arm and `request_bytes`.

## Dropped

None.

## Coverage

Read: the full diff, `Broker::submit`, both FileVATReturn adapters,
`run_submit_filing`, `reconcile_file_vat`, `resume_file_vat`,
`retry_file_vat_submit`, `complete_manual_vat`, `status.rs`. Tested:
`scripts/ci-local.sh` green before and after the fixes. Outside scope: a new
Run re-sending after an unknown outcome (`vat-filing-resent-by-new-run-after-unknown`).
