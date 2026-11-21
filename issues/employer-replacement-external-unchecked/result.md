# Result

A replacement of a month recorded with `complete-manual` (a report given
outside aggountant) is no longer built from the profile's pension insurances.

- The Incomes Register gives no way to fetch the previous version's content:
  transcripts 600/630 and record 312 carry only sums of employer's separate
  reports, and only the e-service "Submitted reports" view shows a report. So
  the program cannot learn it and does not guess it.
- `file-wage employer-report preview|submit --replace` for such a month is
  refused with `EMPLOYER_REPORT_PREVIOUS_UNKNOWN`, naming the safe routes: read
  the original's pension insurances in the e-service and state them with
  `--previous-pension-insurance PROVIDER_CODE:POLICY_NUMBER` (repeatable) or
  `--previous-no-pension-insurance`, or correct the report in the e-service.
- With the statement the usual rule applies: a EUR 0 (tax-free reimbursements
  only) replacement repeats the stated insurances, warning when they differ
  from the current settings; a "no wages payable" replacement, which states the
  profile's, is refused with `EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED` when the
  two differ (order-insensitive, both lists named). The preview warns that the
  rest of the original's content could not be compared.
- Stating the previous insurances where aggountant already knows them (it sent
  the report) or where nothing is replaced is refused with
  `EMPLOYER_REPORT_PREVIOUS_STATED_NEEDLESSLY`. Existing inputs and outputs are
  unchanged; the flags are optional and only accepted with `--replace`.
- The interim instruction in `skills/tulorekisteri/kiinnikurominen.md` is
  replaced by the program's check; the user guide (en/fi) and the error-code
  reference describe the new codes.

For the production case (an 81-month catch-up, one wrong "no wages payable"
month to be corrected to EUR 0): record the month's reference with
`complete-manual`, read the original's pension insurances in the e-service,
then preview with `--payments tax-free-reimbursements-only --replace` and the
stated insurances.

Tests: `crates/process/tests/file_employer_report.rs`
(`replacing_a_report_given_outside_aggountant_needs_its_stated_pension_insurances`,
`stated_pension_insurances_are_compared_without_regard_to_order`), the updated
history tests, and the CLI flow in
`crates/cli/tests/employer_report_history_cli.rs`. `scripts/ci-local.sh`
passes. Review and assessment: `review.md`, `assessment.md`.
