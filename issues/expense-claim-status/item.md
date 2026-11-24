---
created: 2026-11-24
updated: 2026-11-24
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:expense-claim-status
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
---

# Derived expense claim status, open-claim list and month-close checks

## Description

## Goal

"What is unfinished with expense claims, by when, and what next?" is one
command, and the month close shows it. Every unfinished state has a command
that closes it. Decided in ADR-11 (`docs/site/adr-11-expense-claim.md`,
section 7).

## Scope

- `claim status <id>` and `claim list --open [--json]`: state derived every
  time from facts (draft, open questions, version link, settlement link,
  `file-wage` run state), never stored. States and closing commands as in
  ADR-11 section 7's table, with `next_command` and the statutory Incomes
  Register deadline from the payment month; a pointer to the employer's
  separate report when a month's only payments are tax-free reimbursements.
- `close-period`: a draft with trips or lines in the period is unresolved
  intake (ADR-5 must-seal), dispositioned by approval, discard or reasoned
  deferral; approved but unpaid or unreported claims are warnings.

## Acceptance criteria

- [ ] Each derived state has a test reaching it and a test leaving it through its closing command.
- [ ] No state is persisted; deleting nothing but changing facts changes the status.
- [ ] `close-period` refuses with a stable blocker id for an open draft and only warns for post-approval states.
- [ ] User guide (fi/en) month-close page lists the claim checks.

## Likely files

`crates/process/src/expense_claim.rs`, `crates/process/src/close_period.rs`,
`crates/process/src/obligation.rs`, `crates/cli/src/claim.rs`, `docs/user/{fi,en}/tasks/close-month.md`.

## Dependencies

Needs `expense-claim-approve-seal`. The paid and reported states become
reachable with `expense-claim-payment-matching` and `wage-report-from-claim`;
the slice can land before them with those states tested through fixtures.

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Proposed 2026-11-24); decision issue `expense-claim-model`. Dependencies above are prose; the conductor sets `blocked_by` after triage.
