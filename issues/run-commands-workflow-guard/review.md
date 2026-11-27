# Review: run-commands-workflow-guard

- Skill: `/llm-review`, rigor routine (refusal paths in commands that drive
  fiscal and filing effects; no new side effect is added).
- Finder: `gpt-6-astra` (one call, read-only repository access, the diff, the
  issue and `crates/process/src/lib.rs` attached). Verifier: not called; the one
  candidate was confirmed locally and is a coverage gap, not a defect claim.
- Scope: the working-tree diff of this slice over `main` (`a642c1a`):
  `check_version(&f, workflow)` in `crates/process/src/lib.rs`, the one-line
  call-site changes in `capture_receipt.rs`, `close_year.rs`, `file_vat.rs`,
  `file_vat/recovery.rs`, `hunt_receipts.rs`, `run_payroll.rs`, the new tests.

## F1 (should-fix): workflow-specific guards lacked regression coverage

- Location: `crates/process/src/run_payroll.rs` (`resolve_errors`,
  `answer_run_payroll`, `handle_payment_failure`, `resume_run_payroll`) and the
  other newly guarded workflow-specific entry points; the first version of the
  reverse-direction test covered only `answer_close_year` / `resume_close_year`.
- Claim: deleting one of those `check_version` calls would leave every test
  green, because existing tests only pass each command Runs of its own
  workflow.
- Local evidence: confirmed by reading `crates/process/tests/run_payroll.rs`
  and the other workflow test files; no test passes a foreign Run to them.
- Impact: a later regression would let a wrong Run id reach that workflow's
  state reads and effects unnoticed.
- Fix: `a_close_period_run_is_refused_by_every_other_workflows_commands` in
  `crates/process/src/lib.rs` passes a close-period Run to all sixteen guarded
  entry points and asserts `INVALID_ANSWER`, the mismatch text and an untouched
  journal and period.

## Dropped candidates

None.

## Coverage

Read: the whole diff, `ProcessEngine::answer`/`resume`/`migrate`, every
`check_version` caller, the CLI dispatch for `run answer|resume|migrate` and
each workflow's resume command. Tested: the full `aggountant-process` suite and
`scripts/ci-local.sh`. Mutation checks: disabling the workflow comparison in
`check_version` fails all new tests. Outside scope: the Incomes Register
workflows (`file-wage`, corrections, employer reports), which already check
their own workflow and version.
