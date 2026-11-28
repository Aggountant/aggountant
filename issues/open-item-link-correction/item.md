---
created: 2026-11-28
updated: 2026-11-28
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-12
source_ref: taskfleet:01m8gcdm7et7akjt1egj1dcmdm/slice:open-item-link-correction
originating_run: 01m8gcdm7et7akjt1egj1dcmdm
originating_run_kind: technical-decision
---

# Correct a wrong settlement link or open-item identifier without touching sealed vouchers

## Description

## Goal

A settlement link or an item identifier recorded wrong can be corrected
without touching a sealed voucher, and the correction keeps its history.
Follows ADR-12 (`docs/site/adr-12-open-items.md`, sections 2 and 5).

## Scope

- Supersede or retract a settlement row (a payment linked to the wrong
  claim) and an origin or identifier row (a wrong reference or due date), each
  with a reason. Nothing is updated or deleted.
- Checks before writing: no posting ends up over-allocated, no item ends up
  settled twice, and the replacement names an existing posting on the kind's
  role account.
- If a superseded link already fed an Incomes Register payment date, the
  claim status shows the conflict and points to `expense-claim-correction`;
  nothing is resent by itself.
- A posting on the wrong account is still corrected by a correction voucher;
  the command says so when asked to move a link across accounts.
- For claims the command is a claim command (name indicative:
  `claim relink`); a generic form waits for the first non-claim kind.

## Acceptance criteria

- [ ] A synthetic payment linked to claim A is moved to claim B: A shows
      unpaid, B paid, the books are unchanged, and both rows remain readable.
- [ ] A correction that would over-allocate or settle twice refuses with a
      code naming the conflicting rows.
- [ ] `claim verify` passes after corrections and detects a hand-inserted
      over-allocation.
- [ ] Companies without the feature see no questions, output changes or
      slowdown: on a synthetic home without items, existing command outputs
      are unchanged (regression test).
- [ ] User guide (fi/en) section on correcting a claim's payment link.

## Likely files

`crates/ledger/src/open_item.rs`, `crates/cli/src/claim.rs`,
`docs/user/{fi,en}/`.

## Dependencies

Needs `expense-claim-payment-matching` (settlement rows exist).

## Decision

ADR-12, `docs/site/adr-12-open-items.md` (Proposed); decision issue
`open-items-model`.
