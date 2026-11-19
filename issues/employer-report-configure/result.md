# Result: employer-report-configure

```
aggountant file-wage employer-report configure \
  --registered-from YYYY-MM --contact-name NAME --contact-phone PHONE \
  (--pension-insurance CODE:POLICY ... | --no-pension-insurance) \
  [--accept-register-mismatch] [--offline] [--dry-run]
aggountant file-wage employer-report configure [--offline] < block.json
```

## What was built

- `crates/cli/src/wage.rs`: the `configure` subcommand. All four fields as
  flags, or none and the `employer_report` block as stdin JSON; a partial flag
  set is refused (`INVALID_INPUT`, naming what is missing). Neither pension
  flag is refused, so an empty list is never a default; both conflict.
  `CODE:POLICY` is parsed and validated at the flag. `--help` carries three
  examples.
- `crates/process/src/file_employer_report.rs`: `configure_employer_report`
  validates the settings, splices the block, runs the YTJ comparison for the
  new `registered_from` (unless `RegistrationCheck::Offline`), refuses any
  result other than `match` with `EMPLOYER_REGISTER_MISMATCH` (the message
  carries the comparison and the register's month) unless the mismatch is
  accepted, and writes only when the bytes change and it is not a dry run.
  A YTJ failure is `YTJ_UNAVAILABLE` and names `--offline`. Output:
  `before` (as written), `after`, `changed`, `written`, `dry_run`,
  `registration_verdict` (`match`, `mismatch_accepted`, `not_checked`) and the
  full `registration` comparison.
- `crates/broker/src/tulorekisteri.rs`: `with_employer_report` replaces or
  appends the block keeping every other byte (located with `RawValue`
  offsets, the result re-parsed and compared), repairing an invalid existing
  block but refusing an invalid rest; `write_wage_profile` writes atomically;
  `EmployerReportProfile::validate`.
- `crates/domain/src/payroll.rs`: `PensionInsurance::validate`, the one home
  of the schema's provider code and policy-number rule.
- `crates/process/src/employer_report_history.rs`: the YTJ error mapping
  factored into `register_entries`, shared with `registration`.
- `crates/cli/src/main.rs`: a human line showing before/after and whether
  anything was written.
- Skill `vero-fi` and the guide (fi, en, error codes) use the command instead
  of a hand-edited profile.

## Tests

Broker unit tests (splice add/replace/compact/repair/duplicate, validation),
process tests with recorded YTJ answers (mismatch refused and accepted, match,
idempotent rerun, dry run, YTJ down, not in register, invalid pension,
offline), CLI tests (offline flags and stdin, partial flags, conflicts,
non-JSON output, help). `scripts/ci-local.sh` passes.

Review and assessment: `review.md`, `assessment.md` beside this file.
