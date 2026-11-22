# Review: employer-report-unit-variant-members

- Skill: `/llm-review`, rigor **routine**; the surface reads stored Run state,
  so a clean finder result still got one focused false-negative question.
- Scope: commit `d1a8cb0` against `6b1d925` (`crates/domain/src/payroll.rs`,
  `crates/process/tests/file_employer_report.rs`).
- Finder: `gpt-6-astra`, one call (diff plus `payroll.rs`,
  `file_employer_report.rs`, `tulorekisteri.rs`, `model/src/de.rs`).
  Verifier: `claude-fable-5-2`, one call, asked only whether any record a
  released version stored could carry a member on the two variants or be read
  through a deserializer that `deny_members` treats differently. Two model
  calls in total.
- Risk class: stored-data compatibility (an old record that stops reading
  would block a month's employer report as `JOURNAL_CORRUPT`).

## Confirmed findings

None.

## Evidence checked

- Both variants have been fieldless, with the same tag and renaming, since
  they were introduced (`4b35784`); `git log -S` shows no other shape.
- Two deserialization sites, both JSON: the Run state file
  (`file_employer_report.rs`, `read_employer_state`, written only by
  `write_state` from typed `RunState`) and the broker envelope body
  (`tulorekisteri.rs`, `EmployerReportRequest`, built with `to_value` in the
  same process; `Envelope` is not read back from disk). The outbox and journal
  store hashes, keys and receipt notes, not the report.
- No CLI command takes either type as input: the report is built from flags
  (`crates/cli/src/wage.rs`, `EmployerReportArgs::input`).
- `deserialize_with` affects deserialization only; the serialized form, the
  delivery id golden value and the record hash are unchanged (domain golden
  test and `payer_summary_schema` tests pass).
- Regression tests fail with the attributes removed and pass with them.

## Dropped candidates

0 candidates proposed; nothing dropped.

## Residual risk

No cross-version run (state written by v0.2.0, read by HEAD); the conclusion
rests on identical type definitions and the unchanged Serialize derive.
