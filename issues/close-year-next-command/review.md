# Review: close-year next step, plain help, archive write error

- Skill: `/llm-review`, rigor **routine**, with attention on the irreversible
  boundary: the change tells the agent which command comes next at gates whose
  approval seals vouchers or the fiscal year.
- Scope: the slice commit (`crates/process/src/close_year.rs`,
  `crates/process/src/lib.rs`, `crates/cli/src/main.rs`,
  `crates/cli/src/input_help.rs`, tests, guide, skill), covering
  `close-year-next-command`, `close-year-plain-help` and
  `close-year-archive-write-error`.
- Finder: `gpt-6-astra`, one call (diff plus `close_year.rs` and `lib.rs`).
  Verifier: not called. Both candidates were confirmed locally by
  reproduction (a shell parse and a failing assertion), so a refutation call
  could not change the outcome. One model call in total.
- Risk class: guidance toward irreversible bookkeeping actions.

## F1 (must-fix): `--answer approve|reject` in `next_command` runs as a pipe

- Location: `crates/process/src/close_year.rs`, `close_year_next`, the
  `CloseYearNextFor::Show` branch (`answer_command(…, "approve|reject")`).
- Claim: SOUL tells the agent to run `next_command`. In a shell
  `--answer approve|reject` is `… --answer approve` piped to `reject`; the
  approval runs (at `ApproveStatements` it seals the fiscal year) and the shell
  reports `reject: command not found`.
- Evidence: `bash -c 'f(){ echo "argv: $*"; }; f --answer approve|reject'`
  runs `f` with `approve` and fails on `reject`.
- Fix: the placeholder is quoted, `--answer '<approve|reject>'`; run as it is
  the shell passes one argument that `answer` refuses with `INVALID_ANSWER`.
  The message says to put approve or reject in its place. Test
  `the_answer_placeholder_in_next_command_is_not_a_valid_answer`.
- The same string exists in `close-period`'s `next_command`
  (`close_period.rs`), outside this slice's files: filed as
  `close-period-answer-placeholder-pipe`.

## F2 (should-fix): an obstruction at the archive path read as a written archive

- Location: `close_year_next` (Completed branch) and `resume_close_year`
  (archive rewrite branch), both testing `archive_path(..).exists()`.
- Claim: when the write failed because something other than a file occupies
  `archive/<year>/tilinpaatos.json` (the obstruction the test itself used),
  guidance said the archive was written and resume did nothing.
- Evidence: the test removed the obstruction before checking guidance;
  asserting before removal failed.
- Fix: both checks use `is_file()`; the test now asserts the missing-archive
  message and a resume error while the obstruction stays, then the rewrite
  after it is removed.

## Dropped

None.

## Coverage

Read: the whole diff, `close_year.rs`, the fold in `lib.rs`, the
`close_period` guidance it mirrors. Tested: process and CLI close-year suites,
`scripts/ci-local.sh`. Outside scope: the close-period guidance (filed above).
