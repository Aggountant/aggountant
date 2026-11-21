# Review: close-year-explicit-answer

- Skill: `/llm-review`, rigor **critical**: the change guards the approval that
  seals a fiscal year irreversibly.
- Scope: `crates/cli/src/main.rs` (`CloseYearCmd` flags and help) and the new
  `crates/cli/tests/close_year_cli.rs`, checked against
  `crates/process/src/close_year.rs`.
- Finder: `gpt-6-astra`, one call (`--task review`, diff plus the engine and the
  issue), asked in particular to check every factual claim in the new help text
  against the engine. Verifier: none; one model call in total.
- Risk class: irreversible operation (fiscal-year seal).

## Findings

No findings. The finder verified the gate order and both permanent sealing
steps (`answer_close_year`, `run_seal_adjustments`, `run_seal_year`), that a
rejection only records the answer and leaves the run `Rejected`, the `cy-` run
id prefix and the `correction_id` field.

## False-negative check (local)

Because the surface is high-risk, the one boundary that a clean finder could
miss was checked locally: whether a close-year gate can still be approved by
omission through another command. The generic `run answer` already declares
`--answer` without a default, and no other command calls
`answer_close_year`. No verifier call was needed for that.

## Dropped candidates

None.

## Coverage

Read: the diff, `answer_close_year` and its seal steps, `RunCmd::Answer` and
`ProcessEngine::answer`. Tested: `scripts/ci-local.sh` (fmt, clippy, deny,
stable and MSRV tests) passed. Outside scope: the same `default_value =
"approve"` on `capture-receipt`, `hunt-receipts` and `run-payroll` answer flags;
those gates do not seal a fiscal year and the issue covers only `close-year`.
