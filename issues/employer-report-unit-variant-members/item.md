---
created: 2026-11-21
updated: 2026-11-22
type: bug
status: fixed
priority: normal
lane: unlaned
collision: [crates/domain/src/payroll.rs]
closed: 2026-11-22
closed_by: worker
commits:
- hash: d1a8cb0
  summary: 'payroll: employer report unit variants refuse members'
---

# Employer report unit variants drop members silently

## Description

Proposed by the worker of `receipt-vat-proposal-no-vat-unknown-members`.
`crates/domain/src/payroll.rs` has two internally tagged
`deny_unknown_fields` enums with unit variants,
`EmployerReportContent::NoWagesPayable` and `EmployerReportAction::New`.
Serde's container attribute does not reach a unit variant, so members on
them are dropped silently.

## Wanted

Find whether either type is read from command input (stdin or files) or
only built internally. Where it is input, refuse members with
`aggountant_model::de::deny_members` and INVALID_INPUT, with a regression
test. Where it is read from stored data, check stored records first, as in
the VAT expansion case.

## Decisions

### 2026-11-22T04:29:46Z · @conductor

Accepted by the product owner (2026-11-22).

## Resolution

### 2026-11-22T04:39:34Z · @worker

Neither type is command input; stored state and envelope are written only by Serialize, so deny_members was applied to both unit variants. A member now reads as JOURNAL_CORRUPT. See result.md.
