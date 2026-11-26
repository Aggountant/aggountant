---
created: 2026-11-24
updated: 2026-11-26
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:expense-claim-payment-matching
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
---

# Match expense claim payments in bank import and record other settlements

## Description

## Goal

When the bank statement shows the payment to the claimant, the import books
it as the settlement of the named claims, so the claim knows it is paid and
the Incomes Register payment date is a fact from the bank. Follows ADR-11
(`docs/site/adr-11-expense-claim.md`, section 6); this issue owns the
settlement details. aggountant pays nothing and keeps no payables ledger
(`crates/AGENTS.md`, "Vouchers, not ledgers of receivables"): the amount
outstanding is derived from links between vouchers.

## Scope

- `import camt` resolution kind "settles claims C1, C2": seals the payment
  voucher (claimant payable to bank) and writes the settlement link in the
  same batch transaction. The named claims' outstanding amounts (derived
  from links) must equal the payment; partial payment refuses for now.
- `claim settle` for a payment made otherwise (offset against a receivable
  from the claimant, an account whose statement is not imported): seals the
  settling voucher with evidence. For an offset the user states the payment
  date with a reason (ADR-11 open question 7).
- A claim payment never also matches as an ordinary bank entry
  (`BANK_ENTRY_POSSIBLY_BOOKED` rules unchanged).

## Acceptance criteria

- [ ] A synthetic statement paying two claims in one transfer settles both; a wrong sum refuses the batch.
- [ ] Re-importing an overlapping statement does not settle twice.
- [ ] Outstanding per claim is correct with a shared payable account.
- [ ] User guide (fi/en) bank-import page covers claim payments.

## Likely files

`crates/cli/src/bank_import.rs`, `crates/ledger/src/bank_event.rs`,
`crates/ledger/src/expense_claim.rs`, `crates/ledger/src/store/sqlite_fiscal.rs`,
`crates/cli/src/claim.rs`, `docs/user/{fi,en}/tasks/import-bank-statement.md`.

## Dependencies

Needs `expense-claim-approve-seal`.

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Proposed 2026-11-24, rewritten 2026-11-26 to the product owner's decisions); decision issue `expense-claim-model`. The ADR states the principle; the details above are this issue's own. Dependencies above are prose; the conductor sets `blocked_by` after triage.
