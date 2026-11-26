---
created: 2026-11-24
updated: 2026-11-26
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
that closes it. Follows ADR-11 (`docs/site/adr-11-expense-claim.md`,
section 6); this issue owns the states and their closing commands.

## Scope

- `claim status <id>` and `claim list --open [--json]`: state derived every
  time from facts (draft, open questions, version link, settlement link,
  `file-wage` run state), never stored, with `next_command` and the
  statutory Incomes Register deadline from the payment month; a pointer to
  the employer's separate report (`file-wage employer-report`) when a
  month's only payments are tax-free reimbursements.
- States and their closing commands (command names indicative):

  | Derived state | From | Closing commands |
  |---|---|---|
  | Draft with open questions | draft exists, questions remain | answer (draft write, register entry) or `claim discard --reason` |
  | Ready for approval | draft, no open questions | `claim submit` (in a one-person company acknowledged at once as self-approval), or `claim discard --reason` |
  | Awaiting approval by *X* | submitted, the company's recorded approver has not acted | `claim approve` by the approver, `claim reject` (back to the draft) by the approver, `claim withdraw` by the claimant |
  | Approved, unpaid | version link, no settlement | bank import settles it; `claim settle` with evidence (offset or another account); `claim cancel` |
  | Paid, report due *(deadline)* | settlement, reportable lines, no accepted report | `file-wage from-claim` then the `file-wage` Run; `file-wage complete-manual` for a report given in the e-service |
  | Report outcome unknown | the `file-wage` Run is `OutcomeUnknown` | `file-wage reconcile` (never a resend) |
  | Register correction outside aggountant | a correction reduced an amount already paid and reported | `file-wage correction complete-manual` with the e-service evidence |
  | Complete | settled, and reported or nothing to report | none |
  | Superseded / cancelled | a later version or a cancellation exists | none |

- "Awaiting approval by X" is in the first version in every company
  (product owner 2026-11-26, ADR-11 section 5); in a one-person company the
  step exists but is never seen waiting. The approver comes from the
  company's approver record (`expense-claim-approve-seal`).
- A refusal leaves no claim state to close: a preview that refuses records
  nothing, and a refused approval leaves the draft as it was, with the
  ledger's rejection record as for any refused seal.
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

ADR-11, `docs/site/adr-11-expense-claim.md` (Accepted 2026-11-26); decision issue `expense-claim-model`. The ADR states the principle; the details above are this issue's own. Dependencies above are prose; the conductor sets `blocked_by` after triage.
