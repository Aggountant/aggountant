---
created: 2026-11-24
updated: 2026-11-26
type: feature
status: open
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:expense-claim-payment-matching
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
lane: expense-claim
lane_seq: 40
collision: [crates/import]
blocked_by: ['@expense-claim-approve-seal']
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
  Partial settlement (offset plus bank payment of one claim) comes later
  with the travel-advance slice after the first version (ADR-11 section 11),
  so the settlement link should not assume one settling voucher per claim.
- `claim settle` for a payment made otherwise (offset against a receivable
  from the claimant, an account whose statement is not imported): seals the
  settling voucher with evidence. The offset date is the Incomes Register
  payment date (product owner 2026-11-26, ADR-11 section 6; Tax
  Administration VH/5655/05.00.00/2026, ch. 2.1). When the offset and the
  bank payment of the difference would fall in different months the source
  does not settle the date; the guidance is to date the offset on the day
  the difference is paid.
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

ADR-11, `docs/site/adr-11-expense-claim.md` (Accepted 2026-11-26); decision issue `expense-claim-model`. The ADR states the principle; the details above are this issue's own. Dependencies above are prose; the conductor sets `blocked_by` after triage.
