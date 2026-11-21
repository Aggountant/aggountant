# Result: filing-output-markers

Three facts the skills had to carry now appear in the output itself.

## What changed

- **Sensitive `record_hash`.** `file-wage preview` and `file-wage start
  --dry-run` carry `sensitive.record_hash`, a sentence saying the hash is
  derived from the employee's personal identity code and is used only to
  approve the report, never stored, logged or shared. Human output adds the same
  line. `--help` says so too. (`crates/process/src/file_wage.rs`,
  `crates/cli/src/main.rs`, `crates/cli/src/wage.rs`)
- **Test seam visible.** Any command that built its broker from
  `AGGOUNTANT_BROKER_SCRIPT` (file-vat, run-payroll, capture/hunt runs) adds
  `test_seam: {active, variable, message}` to its JSON output and to its JSON
  error line; without `--json` a `warning: AGGOUNTANT_BROKER_SCRIPT is set…`
  line goes to stderr. Commands that build no broker are unmarked. The marker
  is set in `build_scripted_broker_path`, the single place a scripted broker is
  made.
- **Unknown VAT class.** `capture-receipt record` refuses a `domestic-input` or
  `reverse-charge` class not in the dated rate table with
  `CAPTURE_VAT_CLASS_UNKNOWN` (user error, exit 1) and lists the known classes.
  Other Domain refusals on the capture paths no longer repeat their code
  (`INVALID_ANSWER: VAT_CODE: …` instead of `… VAT_CODE: VAT_CODE: …`).
- Guide (fi, en): error-code row and `test_seam` note, troubleshooting and
  status pages, travel-allowance task. `skills/` untouched as briefed.

All additions are new fields or a new code on a path that already failed, so
existing inputs and outputs keep working.

## Tests

- `crates/process/tests/file_wage.rs`, `crates/cli/tests/file_wage_cli.rs`:
  the sensitivity note in preview and start dry run.
- `crates/cli/tests/file_vat_cli.rs`: `test_seam` on start and approve-filing,
  absent on `period show` and without a script; on an error line; the stderr
  warning in human mode.
- `crates/cli/tests/capture_structured_cli.rs`: the new code and clean message;
  an undated known class names `VAT_CODE` once.

`scripts/ci-local.sh` green before and after the review fixes.

## Review

`review.md` (`/llm-review`, routine, `claude-fable-5-2`) and `assessment.md`:
two confirmed should-fix findings, both fixed. The underlying acceptance of
`domestic-input` with an EU or zero-rated class (pre-existing, Domain) is filed
unlaned as `capture-domestic-input-eu-class`.
