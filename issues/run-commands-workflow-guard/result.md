# Result: run-commands-workflow-guard

Every command that continues a Run now first checks that the Run belongs to
the workflow the command implements. `check_version(&f, workflow)` in
`crates/process/src/lib.rs` refuses a Run of another workflow with
`INVALID_ANSWER` before anything is read from or written to the fiscal store or
the journal, and names the commands that continue it (`aggountant run`,
`aggountant capture-receipt`, `aggountant file-wage correction`, …). For a
workflow this binary does not know, the message says so and points to the
aggountant version that started the Run.

- `run answer` / `run resume`: a capture-receipt, hunt-receipts, run-payroll,
  close-year or unknown-workflow Run is refused. `run resume` still hands a
  file-vat Run to its own recovery first.
- The other workflows' answer, resume, retry and reconcile commands are guarded
  the same way (a close-period Run passed to `close-year resume` used to pass
  the version check too).
- `expected_version` no longer falls back to the close-period version; `run
  migrate` refuses a Run whose workflow has no version here instead of moving
  it to the close-period version.

No store migration. Sealed vouchers, journals and receipts are untouched; the
change only adds refusals.

## Tests

- `crates/process/tests/capture_receipt.rs`:
  `generic_run_answer_and_resume_refuse_a_capture_receipt_run`: a real
  capture-receipt Run in an open period; `run resume` and `run answer` are
  refused, the period stays Open with the same owner and transition head, and
  the journal is byte-for-byte the same.
- `crates/process/src/lib.rs` `workflow_guard_tests`: an unknown workflow
  (even claiming the close-period version) refused by resume, answer and
  migrate; `run migrate` refuses a file-wage Run; a close-period Run refused by
  all sixteen workflow-specific entry points.
- Disabling the workflow comparison fails all four tests.

## Overlap

Also covers `run-resume-refuses-other-workflows`: its scenario (a
capture-receipt Run in an open month moved to `closing` by `run resume`) is the
first test above. Its status was left for triage as briefed.

## Review

`/llm-review` (gpt-6-astra finder, no verifier needed) and assessment:
`review.md`, `assessment.md`. One coverage finding, fixed.
