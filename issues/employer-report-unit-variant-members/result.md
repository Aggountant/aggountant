# Result: employer-report-unit-variant-members

## Where the types are read

- **Command input: none.** `file-wage employer-report preview/submit` build the
  report from flags (`--month`, `--payments`, `--replace`,
  `--previous-pension-insurance`); no stdin or file input deserializes
  `EmployerReportContent` or `EmployerReportAction`. So there is no CLI
  `INVALID_INPUT` path to add or test.
- **Stored Run state** (`employer-report.json`, read by `read_employer_state`)
  and the **broker envelope body** (`EmployerReportRequest`), both written only
  through serde `Serialize`, which emits the tag alone for a unit variant. The
  variants have had no fields since they were introduced, so no stored record
  can carry members.

## What changed

`NoWagesPayable` and `New` now use `aggountant_model::de::deny_members`. A
member on them (say `"total"` on `no_wages_payable`, or `"ir_report_id"` on
`new`) used to be dropped. It also passed every check, because it does not
change the record that is sent. Now the Run is refused as `JOURNAL_CORRUPT`
in the state file, or `Malformed` in the envelope. The record sent to the
Incomes Register, the delivery id and the record hash are unchanged.

## Tests

- `payroll::tests::employer_report_unit_variants_refuse_members`: members are
  refused, the bare tag reads, and the serialized form is the tag alone.
- `a_member_added_to_a_stored_unit_variant_is_refused_not_dropped`
  (`crates/process/tests/file_employer_report.rs`): an accepted month's state
  with an added member reads as `JOURNAL_CORRUPT`. The original bytes read
  back as `Accepted`.
- Both fail without the fix. `scripts/ci-local.sh` passes.

Review and assessment: `review.md`, `assessment.md`.
