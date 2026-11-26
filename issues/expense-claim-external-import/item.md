---
created: 2026-11-24
updated: 2026-11-26
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:expense-claim-external-import
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
---

# Import an expense claim from an outside travel-claim system with its lifecycle facts

## Description

## Goal

A claim prepared in an outside travel-claim system is brought in with that
system's lifecycle steps recorded as facts, and the approval made there can
be recorded as the approval here when the program's computation matches it.
Follows ADR-11 (`docs/site/adr-11-expense-claim.md`, section 8; product
owner 2026-11-24: lifecycle steps may come from elsewhere and are imported
as facts; 2026-11-26: an outside approval can be recorded as aggountant's
approval). This issue owns the import; the approval record and its match
check belong to `expense-claim-approve-seal`.

## Scope

- `claim import-external` creates a draft from a structured record (trips,
  lines, receipts) and stores the outside system's steps (submitted,
  reviewed, approved by whom and when) as `external_events` with the
  system's name and evidence hash.
- The record carries the content the outside system approved (per trip
  and line, the amounts it computed), so that approval can compare it with
  the program's own computation.
- The events appear in the preview and the sealed document but drive no
  state of their own. The claim is sealed either with a recorded outside
  approval, when the computation matches, or with an own approval; a
  difference is fixed at the source and imported again, or approved here as
  an own approval.
- A documented input shape; one synthetic example; no vendor-specific
  parser in this slice.

## Acceptance criteria

- [ ] A synthetic external record imports, shows its events, and seals with the outside approval recorded as the approval.
- [ ] A synthetic record whose approved per diem differs from the program's computation cannot be sealed as a recorded outside approval and names the difference; an own approval still seals it.
- [ ] Missing statutory fields in the record become open questions, never defaults.

## Likely files

`crates/process/src/expense_claim.rs`, `crates/cli/src/claim.rs`, `docs/user/{fi,en}/`.

## Dependencies

Needs `expense-claim-approve-seal`.

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Proposed 2026-11-24, rewritten 2026-11-26 to the product owner's decisions); decision issue `expense-claim-model`. The ADR states the principle; the details above are this issue's own. Dependencies above are prose; the conductor sets `blocked_by` after triage.
