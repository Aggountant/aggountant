---
created: 2026-11-11
updated: 2026-11-11
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
