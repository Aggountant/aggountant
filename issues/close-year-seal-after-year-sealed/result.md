# Result

`assert_year_open` in `crates/process/src/close_year.rs` refuses with
`CLOSE_YEAR_BLOCKED` once the fiscal year is sealed, naming the sealing run and
telling the user to reject the open gate. It is called before anything is
journaled or sealed: on every `close-year answer … approve` (before the answer
is recorded, so the gate stays open and can be rejected), in
`seal_close_year_correction` before the effect starts, and first in
`seal_attributed` as the backstop for the resume path.

Regression tests (`crates/process/tests/close_year.rs`):
`approving_adjustments_after_another_run_sealed_the_year_books_nothing` and
`sealing_a_correction_after_another_run_sealed_the_year_books_nothing`. Both
failed on main (the approval returned `PERIOD_TRANSITION_CONFLICT` after sealing
a voucher into 2027).

## Decisions for later review

- The check covers every approval, not only `ApproveAdjustments`. Approving
  `ApproveStatements` after another run sealed the year used to record the
  answer and then fail in `SealYear`, leaving the Run with no open gate to
  reject; now the gate stays open. Alternative: limit to the gates that seal
  vouchers. A single rule ("no approval once the year is sealed") is simpler.
- A refused `seal-correction` leaves the correction effect `Planned` rather
  than recording it as rejected: nothing was attempted against the ledger.
