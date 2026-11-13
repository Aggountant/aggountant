---
created: 2026-11-10
updated: 2026-11-13
type: bug
status: open
priority: high
lane: close-year
lane_seq: 1
collision: [crates/process]
preparation: ready
---

# close-year leaves year-end adjustments out of the closing year's statements

## Problem

`close-year` seals the year's depreciation and accrual vouchers into the next
open period (posting date `<year+1>-01-01`), but never attributes them back to
the closing year. The statements it computes, approves and binds into
`SealYear` therefore leave out the year-end adjustments.

The ledger primitive exists: `Ledger::attribute_adjustment` and the
`year_vouchers` fold, which counts an attributed voucher in its effective year.
No caller exists in `crates/process/src/close_year.rs`.

## Reproduction (synthetic home)

1. One purchase voucher in 2026: 98.80 expense, 25.20 input VAT.
2. Seal all twelve months of 2026.
3. `close-year start` with one asset (2 000.00, machinery and equipment).
4. The Run computes 500.00 depreciation and seals it dated `2027-01-01`.
5. After all gates and `SealYear`, the bound statements show a result of
   -98.80: the depreciation is missing from 2026.

## Impact

Silently wrong statements for a sealed, irreversible fiscal year. Until this is
fixed, the user guide (`docs/user/`) tells users not to use `close-year` for a
real close.

## Expected

Every adjustment and correction voucher the Run seals is attributed to the
closing year before the statements are computed and bound.

## Acceptance criteria

- [ ] A test covers a close with depreciation and asserts the result includes it.
- [ ] Accrual and correction vouchers sealed by the Run are attributed the same way.
- [ ] The user guide's warning is removed in the same commit.
