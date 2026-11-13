# Review: file-vat status (vat-filed-status)

- Skill: `/llm-review`, routine rigor. The surface is authority filing, so it
  was treated as high-risk and a verifier was called even for fixed candidates.
- Finder: `gpt-6-astra` (one call). Verifier: `claude-fable-5-2` (one call).
- Scope: diff from `66c232f` to the implementation commit, plus the fixes made
  after the finder. That covers `crates/process/src/file_vat/status.rs`, the
  helper extraction in `crates/process/src/file_vat.rs`,
  `crates/broker/src/vero_vat_api.rs`, the CLI wiring, the tests,
  `docs/user`, and `skills/vero-fi`.

## Confirmed findings

### F1 (must-fix): a no-activity return matched any processed remote return

- Location: `crates/broker/src/vero_vat_api.rs`,
  `FiledVatReturn::semantically_matches_attempt`.
- Claim: when the expected request had `NoActivity=true`, the function returned
  `true` without looking at the remote amounts. `status` would then say
  "processed with the approved values" even though the authority showed
  non-zero figures. `reconcile` could also adopt such a return as ours.
- Evidence: the code had an early `return expected_details.is_none()`. The
  verifier confirmed the fix against the working tree.
- Fix: a no-activity request now matches only when the remote amounts are
  absent or zero. This holds for both numeric and string amounts. The change
  only makes the check stricter. Unit test:
  `a_no_activity_return_matches_only_a_remote_return_without_amounts`.

### F2 (should-fix): several Runs whose filing effect folds to Succeeded made status fail

- Location: `crates/process/src/file_vat/status.rs`, `filing_run_with_receipt`.
- Claim: `resume_file_vat` can adopt a filing made by another Run through
  `EffectReconciled`, which also folds to Succeeded. Two such Runs then made
  status fail with `JOURNAL_CORRUPT`.
- Fix: Runs that bound identical request bytes now count as one. A Run with
  different bytes still fails closed, and the error says the receipt proves the
  filing. The verifier traced the remaining ambiguous case as reachable only
  through a profile switch, an `IdempotencyMismatch` and a library-only resume.
  It never yields `not_filed`.

### F3 (must-fix, found by the verifier): a 200 without a readable receipt was reported as `not_filed`

- Location: `crates/process/src/file_vat/status.rs`, the rejected-run branch.
  The root cause is in the broker: an HTTP 200 whose body does not parse as a
  receipt is recorded as `Failed { MALFORMED }` / `Rejected`.
- Claim: status answered `not_filed … nothing was accepted` after an HTTP 200
  without a readable receipt, although Vero may have accepted the return.
- Evidence: the `200 =>` arm of `VeroFileVatReturnAdapter::call` and the
  `Malformed` arm of `Broker::submit`, confirmed locally.
- Fix (status side): a filing rejection coded `OUTCOME_UNKNOWN`, `MALFORMED…`
  or `HTTP_2…` now answers `outcome_unknown` and points to
  `file-vat reconcile`, which accepts any rejected filing effect. Test:
  `a_success_status_without_a_readable_receipt_is_not_reported_as_not_filed`.
- The broker misclassification also allows a second filing. That belongs to
  the filing path, not to this read-only command, so it went to intake as
  `staggeringly-aspiring-sky`.

## Dropped

- Candidate: "`Ledger::open` migrates the fiscal schema, so status is not
  read-only". Every command opens the ledger the same way, including
  `file-vat preview`, which the guide calls read-only. A schema upgrade changes
  no receipt, filing state or Run state. Dropped as existing convention.
- Verifier suggestion: expose the fiscal filing `sent_hash` as a Ledger read to
  select the Run exactly. `crates/ledger` is outside this task's boundary, and
  the current selector fails closed. Not done; recorded in `result.md`.

## Coverage and residual risk

I read every changed file, `resume_file_vat`, `reconcile_file_vat`,
`run_submit_filing`, the broker submit and outbox classification, and the mock.
The tests run against the mTLS mock server (11 process tests) and the scripted
CLI seam (5 CLI tests). They assert that no FileVATReturn request is made and
that the home's files are byte-identical before and after `status`. Residual
risk: the real GetFiledVATReturn status vocabulary beyond `Missing`,
`Being Processed` and `Processed` is unknown. Any other value answers
`unrecognized_remote_status`, never processed or not filed.
