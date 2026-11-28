---
created: 2026-11-28
updated: 2026-11-28
type: task
status: open
priority: high
lane: open-items
lane_seq: 10
---

# Decide the open-item model (expense claims first, then sales and purchase invoices)

## Goal

Decide how aggountant knows, between period closes, what is still open: an
approved expense claim not yet paid (or scheduled for payment), and later
issued sales invoices and purchase invoices not yet settled. Record it as an
ADR and split the implementation into slices.

## Why now

A production deployment reported that sales invoices and expense-claim
payments are created in the bank's own invoicing tool, so the books learn
about them only at period close from the statement. An approved claim whose
payment was scheduled mid-month looked unpaid at month close and was nearly
paid twice. Bank statements carry only booked transactions, so a scheduled
payment never appears there.

`expense-claim-approve-seal` makes the expense claim's only allowed fiscal
store migration. If its settlement link is claim-specific, generalising it
later needs a second, data-copying migration over production homes. The
decision must therefore land before that slice starts.

## Constraints

- `crates/AGENTS.md`, "Vouchers, not ledgers of receivables": no invoicing,
  receivables ledger, collection or payment batches. The ADR decides whether a
  derived open-item view fits that rule and proposes its wording.
- Companies that do not use the feature see no questions, changes or slowdown.
- aggountant never initiates payments.

## Done

ADR accepted by the product owner; slices filed; `expense-claim-approve-seal`
updated to create the generic tables if the ADR so decides.
