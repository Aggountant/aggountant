---
created: 2026-11-11
updated: 2026-11-28
type: feature
reporter: jari
status: open
priority: normal
preparation: ready
lane: tax-account
lane_seq: 1
collision: [crates/ledger, crates/process]
---

# Track the tax account: liabilities, payments and interest

## Description

The Tax Administration's tax account holds VAT liabilities and refunds,
payments, late-payment interest and possibly a payment plan. The general
ledger balance alone does not say what is on the tax account, and reconciling
the two is manual today.

Record tax-account events (liability, payment, interest) with source and
as-of identity, linked to ledger vouchers and artifacts, and show the
difference between the derived balance and the authority's balance. A full
payment-plan model and live synchronisation are left out.

## Acceptance Criteria

- [ ] Tax-account events can be recorded and listed with their source.
- [ ] A report shows derived vs authority balance and the difference.

## Comments

### 2026-11-28T06:23:57Z · @worker

ADR-12 (`docs/site/adr-12-open-items.md`, Proposed 2026-11-28) reserves the open-item kind `tax` in the generic tables. When this issue models tax-account liabilities and payments, coordinate so a declared tax is an open item settled through the same settlement link, rather than a second list of what is owed. `camt-creditor-reference` already recognises an outgoing payment with the company's self-assessed tax reference.
