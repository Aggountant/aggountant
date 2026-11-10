---
created: 2026-11-10
updated: 2026-11-10
type: improvement
status: open
priority: normal
lane: year-end
lane_seq: 30
collision: [crates/ledger]
---

# Year and period reads decode the whole ledger

## Problem

`Ledger::year_vouchers` and `period_vouchers` take a verification snapshot and
strictly decode every voucher in the ledger to filter by date.
`compute_year_statements` runs several times per close, and `year_vouchers(Y)`
also folds every earlier year to build the opening balances. The cost of a
close is therefore proportional to the whole history and is paid again on every
preview and retry. Fine at test scale; slow for a ledger with many years.

## Expected

- A `posting_date` predicate on stored vouchers (populated at seal and
  cross-checked by `verify`), so a period or year read decodes only its own
  vouchers.
- A hash-bound closing-balance snapshot per sealed year, written at
  `SealYear`, so year Y starts from year Y-1's snapshot plus its own movements.

## Acceptance criteria

- [ ] A benchmark or test with a synthetic multi-year ledger shows a year close decoding only that year's vouchers plus the previous snapshot.
- [ ] `verify` detects a snapshot that disagrees with the vouchers.
