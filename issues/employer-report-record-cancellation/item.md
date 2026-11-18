---
created: 2026-11-18
updated: 2026-11-18
type: bug
status: open
priority: normal
provenance: other
provenance_detail: found by taskfleet worker during employer-separate-report review
source_ref: taskfleet:01m7q4nb0tg24x4ny8ffkhv28m/observed:employer-report-cancellation-unrecordable
originating_run: 01m7q4nb0tg24x4ny8ffkhv28m
originating_run_kind: spinoff
lane: wage-reporting
lane_seq: 1
collision: [crates/broker/src/tulorekisteri.rs, crates/cli/src/wage.rs]
preparation: ready
---

# employer-report: an e-service cancellation cannot be recorded; the month keeps the cancelled report as current

## Description

`file-wage employer-report` has no way to record that an accepted employer's
separate report was cancelled (mitätöity) in the Incomes Register e-service.

The register's guidance (Reporting data to the Incomes Register: employer's
separate report, VH/6456/05.00.00/2025, §5) requires cancel-and-new for some
corrections: the reporting period, the payer's identifiers, the pension
provider code or policy number, and the accident insurance details.
aggountant refuses those as replacements
(`EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED`) and sends the user to the
e-service, which is right. But once the user has cancelled the month there and
given a new report, aggountant still holds the cancelled report as the month's
accepted one:

- `submit` for the month returns `EMPLOYER_REPORT_ALREADY_FILED`;
- `complete-manual` with the new report's reference also returns
  `EMPLOYER_REPORT_ALREADY_FILED`;
- `status` keeps showing the cancelled report's reference as `current`.

Example (Esimerkki Oy, fictional): October 2026 is filed through aggountant as
"No wages payable" with pension policy 12-3456789; the policy number turns out
to be wrong; the user cancels it in the e-service and gives a new report with
the right policy. aggountant's record of October now names a report that no
longer exists in the register, and a later replacement would target it.

Fix sketch: a `complete-manual --supersedes` (or a separate
`record-cancellation`) that records e-service evidence of the cancellation and,
optionally, the new report's reference, so `current` moves to the report the
register actually holds. Found while gating employer-separate-report.
