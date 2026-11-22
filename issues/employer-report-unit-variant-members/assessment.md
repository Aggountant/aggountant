# Assessment of the employer-report-unit-variant-members review

Assessed against `d1a8cb0`.

The review confirmed no findings, so there is nothing to fix, spin off or
discuss.

FIX: 0   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

## Simplicity

- Reuses the existing `aggountant_model::de::deny_members` helper, as the
  receipt VAT proposal does; no new abstraction.
- No repetition added: one attribute per unit variant, with the reason stated
  once on `EmployerReportContent`.
- Deliberately left out: a CLI-level test, because neither type is command
  input; the observable path is a stored Run state, tested in
  `crates/process/tests/file_employer_report.rs`.
