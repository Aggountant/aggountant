---
created: 2026-11-24
updated: 2026-11-24
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:three-year-site-clock
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
---

# Track the three-year rule for customer sites and refuse expired tax-free allowances

## Description

## Goal

The program tracks the three-year rule for customer sites (TVL 72 a §) and
refuses tax-free allowances when it has run out, so a long customer
engagement cannot silently turn into pay or a hidden dividend. Decided in
ADR-11 (`docs/site/adr-11-expense-claim.md`, sections 3 and 4; product owner
2026-11-24: the rule is tracked from the first version).

## Scope

- Register additions: principal-site assessment (yes/no/unknown from a
  date), earlier exposure the program cannot see, agreed assignment end
  (date/open-ended/unknown, and whether an extension was unforeseen), gap
  confirmations (worked elsewhere during a gap).
- Site clock: visit days derived from drafts and sealed claims plus earlier
  exposure; six-month gaps by the guidance's date rule (ohje 5.3.4, month
  end clamping); clock start at the principal period; last tax-free day.
- Refusals: principal site past its last day, principal site open-ended or
  known from the start to exceed three years (ohje 5.3.1–5.3.3); open
  questions: principal status needed (share of weekdays over 90 days or age),
  gap confirmation needed; warnings: approaching the limit, assessment
  contradicted by the trip pattern, a place recorded as occasional visited
  weekly (KHO 2017:178).
- `claim verify` lists sealed claims whose workplace snapshot differs from
  what the register now says; it changes nothing.

## Acceptance criteria

- [ ] Synthetic cases: weekly visits for five years with principal "no" (no limit); open-ended principal site (refused from the start); gap ending on 28.2 vs 1.3 after a last visit on 31.8; a single meeting breaking a gap; a backward correction listing affected sealed claims.
- [ ] Every refusal and question has a code and an explanation naming the guidance section.

## Likely files

`crates/domain/src/travel/`, `crates/ledger/src/store/sqlite_pii.rs`,
`crates/ledger/src/employee.rs` or `claimant.rs`, `crates/process/src/expense_claim.rs`,
`crates/cli/src/claim.rs`, `docs/user/{fi,en}/`.

## Dependencies

Needs `claimant-workplace-register` and `expense-claim-draft`.

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Proposed 2026-11-24); decision issue `expense-claim-model`. Dependencies above are prose; the conductor sets `blocked_by` after triage.
