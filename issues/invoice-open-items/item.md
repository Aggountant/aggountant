---
created: 2026-11-28
updated: 2026-11-28
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-12
source_ref: taskfleet:01m8gcdm7et7akjt1egj1dcmdm/slice:invoice-open-items
originating_run: 01m8gcdm7et7akjt1egj1dcmdm
originating_run_kind: technical-decision
---

# Open-item identity for sales and purchase invoices and an open-item list

## Description

## Goal

A company that wants to see its open sales and purchase invoices in
aggountant can give a sealed invoice voucher an open-item identity (reference,
invoice number, due date, counterparty, source system) and list what is open
and overdue, derived from the books. Follows ADR-12
(`docs/site/adr-12-open-items.md`, sections 1–3). Waits for the product
owner's answers to ADR-12 open questions 1 (scope) and 2 (where open sales
invoices are watched weekly).

## Scope

- Identity at seal (optional fields on `voucher draft/seal` input) and
  attached later to an already sealed voucher, writing open-item and origin
  rows of kind `sales_invoice` or `purchase_invoice`; the payment reference
  in its own indexed column.
- Settlement of invoice items by bank import resolution and by attaching an
  existing payment voucher, through the same link and in-transaction refusal
  as claims.
- The first generic command group (name indicative: `open-item list --open
  [--overdue] [--kind] [--as-of] --json`, `open-item show`), with state, due
  date, days overdue, source and next command. Claims appear as their kind.
- Nothing is asked of a sales voucher without an identity: the fields are
  optional and absent by default.

## Acceptance criteria

- [ ] A synthetic sales invoice sealed with an identity is open, becomes
      overdue after its due date, and is settled by a bank import that names
      it; a second settlement refuses naming the first.
- [ ] An invoice booked earlier without identity can be attached later, and
      a payment already booked can be linked.
- [ ] Companies without the feature see no questions, output changes or
      slowdown: `voucher seal` without identity fields, `import camt`, and
      the close previews are byte-identical on a synthetic home without items
      (regression test).
- [ ] User guide (fi/en): recording an invoice's identity and the open list.

## Likely files

`crates/ledger/src/open_item.rs`, `crates/cli/src/` (voucher input, a new
open-item command module), `crates/cli/src/bank_import.rs`,
`docs/user/{fi,en}/`.

## Dependencies

Needs `expense-claim-payment-matching` (the generic settlement path) and the
product owner's answers to ADR-12 questions 1 and 2.

## Decision

ADR-12, `docs/site/adr-12-open-items.md` (Proposed); decision issue
`open-items-model`.
