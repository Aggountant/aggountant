---
created: 2026-11-28
updated: 2026-11-28
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-12
source_ref: taskfleet:01m8gcdm7et7akjt1egj1dcmdm/slice:open-item-reference-matching
originating_run: 01m8gcdm7et7akjt1egj1dcmdm
originating_run_kind: technical-decision
---

# Propose open-item settlements in bank import by reference, then amount and name

## Description

## Goal

When a statement is imported, the preview proposes which open items a
transaction settles: first by the structured payment reference, then by
amount, date and counterparty name. A human approves every proposal; nothing
settles by itself. Follows ADR-12 (`docs/site/adr-12-open-items.md`,
sections 5 and 7).

## Scope

- `import camt` preview: settlement candidates per transaction in the same
  shape as the `BANK_ENTRY_POSSIBLY_BOOKED` candidates, using the reference
  read by `camt-creditor-reference`, then amount within the outstanding
  amount, `DATE_TOLERANCE_DAYS` and the counterparty name.
- A claim with a live payment schedule is ranked first for a matching
  outgoing payment; a counterparty name that does not match the claimant is
  a warning.
- The candidate search runs only when the company has open items, and reads
  only the item indexes and the named items' postings; no second pass over
  vouchers.

## Acceptance criteria

- [ ] A synthetic statement with a reference-matching payment proposes the
      right item first; a payment with a wrong or missing reference falls back
      to amount and name; no proposal is ever applied without the resolution
      naming it.
- [ ] Two equal transfers to two claimants on one day produce a name warning
      when crossed.
- [ ] Companies without the feature see no questions, output changes or
      slowdown: a home without items gets a byte-identical import preview and
      no measurable extra time (regression test).
- [ ] User guide (fi/en) bank-import page describes the proposals.

## Likely files

`crates/cli/src/bank_import.rs`, `crates/ledger/src/bank_event.rs`,
`crates/ledger/src/open_item.rs`, `docs/user/{fi,en}/tasks/import-bank-statement.md`.

## Dependencies

Needs `camt-creditor-reference` and `expense-claim-payment-matching`;
reference matching of invoices needs `invoice-open-items`.

## Decision

ADR-12, `docs/site/adr-12-open-items.md` (Proposed); decision issue
`open-items-model`.
