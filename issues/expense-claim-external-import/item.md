---
created: 2026-11-24
updated: 2026-11-24
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
system's lifecycle steps recorded as facts, so the approval there shows as
evidence for the bookkeeping approval here. Decided in ADR-11
(`docs/site/adr-11-expense-claim.md`, section 7; product owner 2026-11-24:
lifecycle steps may come from elsewhere and are imported as facts).

## Scope

- `claim import-external` creates a draft from a structured record (trips,
  lines, receipts) and stores the outside system's steps (submitted,
  reviewed, approved by whom and when) as `external_events` with the
  system's name and evidence hash.
- The events appear in the preview and the sealed document but drive no
  state of their own; aggountant's approve-and-seal remains the bookkeeping
  step and lists the outside approval as evidence.
- A documented input shape; one synthetic example; no vendor-specific
  parser in this slice.

## Acceptance criteria

- [ ] A synthetic external record imports, shows its events, and seals with the outside approval as evidence.
- [ ] Missing statutory fields in the record become open questions, never defaults.

## Likely files

`crates/process/src/expense_claim.rs`, `crates/cli/src/claim.rs`, `docs/user/{fi,en}/`.

## Dependencies

Needs `expense-claim-approve-seal`.

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Proposed 2026-11-24); decision issue `expense-claim-model`. Dependencies above are prose; the conductor sets `blocked_by` after triage.
