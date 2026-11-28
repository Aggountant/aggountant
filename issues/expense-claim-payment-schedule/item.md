---
created: 2026-11-28
updated: 2026-11-28
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-12
source_ref: taskfleet:01m8gcdm7et7akjt1egj1dcmdm/slice:expense-claim-payment-schedule
originating_run: 01m8gcdm7et7akjt1egj1dcmdm
originating_run_kind: technical-decision
---

# Record a scheduled claim payment and derive the scheduled and not-seen states

## Description

## Goal

When the user schedules a claim's payment in the bank, aggountant knows it:
the claim leaves the pay-now list, the month close says "payment scheduled
for D" instead of "unpaid", and a schedule that the imported statements show
did not happen asks for a decision instead of returning the claim to the
list by itself. Follows ADR-12 (`docs/site/adr-12-open-items.md`, section 4);
this issue owns the schedule fact's commands and the derived refinements.

## Scope

- Commands (names indicative): `claim schedule-payment <claim> --date D
  --source user|system:<name> [--reference R] [--evidence <hash>]`,
  moving a schedule (a new fact superseding the old), cancelling it at the
  bank (superseding fact marked cancelled), and retracting one recorded on
  the wrong claim (retraction with a reason). Each writes one row to the
  open-item fact table (kind `payment_scheduled`) created by
  `expense-claim-approve-seal`; recording needs no open period.
- Derived refinements of "Approved, unpaid" in `claim status` and
  `claim list --open`:
  - *Approved, payment scheduled for D*: a live schedule, and the company's
    imported statements do not yet cover D plus `DATE_TOLERANCE_DAYS`;
  - *Scheduled payment not seen in the bank*: the statements cover D plus the
    tolerance and no settlement exists. Closing commands: import the missing
    statement, cancel (back to unpaid), reschedule, `claim settle` with
    evidence.
- Coverage of imported statements is derived from what the store already
  holds (the retained statements or the latest booked bank event); this slice
  decides which and documents it.
- The pay-now list is "Approved, unpaid" with no live schedule. A stale
  schedule never returns a claim to it.
- `close-period` warning text follows the refinement. Whether claim warnings
  are part of the close's approved content is ADR-12 open question 4; build
  what the product owner answers (recommendation: included, serialised only
  when present).
- Whether evidence is mandatory is open question 3 (recommendation: source
  always, evidence optional).

## Acceptance criteria

- [ ] A synthetic claim scheduled for D: the close before D warns "scheduled"
      and the pay-now list is empty; importing a statement covering D plus the
      tolerance without the payment gives "not seen"; cancelling returns it to
      the pay-now list.
- [ ] A schedule recorded after its month is closed is accepted and changes
      the state; "what was known at the close" is answerable from the
      recording times.
- [ ] Moving, cancelling and retracting each leave the earlier rows intact.
- [ ] Companies without the feature see no questions, output changes or
      slowdown: on a synthetic home without claims, `close-period`,
      `close-year` and `import camt` previews are byte-identical to the
      previous build (regression test).
- [ ] User guide (fi/en): recording a scheduled payment, and the claim states.

## Likely files

`crates/ledger/src/open_item.rs` (the module `expense-claim-approve-seal`
creates), `crates/process/src/expense_claim.rs`,
`crates/process/src/close_period.rs`, `crates/cli/src/claim.rs`,
`docs/user/{fi,en}/`.

## Dependencies

Needs `expense-claim-approve-seal` (the fact table) and
`expense-claim-status` (the states it refines).

## Decision

ADR-12, `docs/site/adr-12-open-items.md` (Proposed); decision issue
`open-items-model`.
