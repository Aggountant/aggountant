---
created: 2026-11-24
updated: 2026-11-24
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:expense-claim-legacy-migration
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
---

# Register file-based expense claims without resealing approved months

## Description

## Goal

A company that already keeps its expense claims as files (one file per line
and a monthly summary sealed as the payable voucher's attachment) moves to
the product's claims without resealing any approved month. Decided in
ADR-11 (`docs/site/adr-11-expense-claim.md`, section 12).

## Scope

- `claim register-legacy`: for an approved month, a link row of kind
  `legacy` (the claim id in the home's existing format, version 1, document
  hash = the voucher's existing `attachment_ref`, voucher number), the
  settlement link from the existing payment voucher, and the existing
  `file-wage` report found by its `source_reference`. No new document is
  made for a sealed month. `claim show` renders the sealed summary as it is.
- `claim import --legacy`: reads a month's line files into a draft and
  reports what is missing (free meals, start place, workplace entry) instead
  of inventing it. Documented to run on a throwaway copy of the home first.
- The guide says a summary edited after approval is not the voucher.

## Acceptance criteria

- [ ] Synthetic legacy home (Esimerkki Oy): two approved months register with correct status (paid, reported); the open month imports as a draft with open questions.
- [ ] `voucher verify` and `claim verify` pass after registration; no voucher is sealed by registration.
- [ ] Registering the same month twice is idempotent; a hash that does not match the voucher refuses.

## Likely files

`crates/process/src/expense_claim.rs`, `crates/ledger/src/expense_claim.rs`,
`crates/cli/src/claim.rs`, `docs/user/{fi,en}/`.

## Dependencies

Needs `expense-claim-approve-seal` (registration), `expense-claim-draft`
(import) and `expense-claim-payment-matching` (settlement link).

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Proposed 2026-11-24); decision issue `expense-claim-model`. Dependencies above are prose; the conductor sets `blocked_by` after triage.
