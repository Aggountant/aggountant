# Result

`seal_close_year_correction` in `crates/process/src/close_year.rs` refuses
with `GATE_NOT_OPEN` unless `ApproveAdjustments` is open and unanswered. A
correction that is already sealed still returns the Run view unchanged (the
idempotent repeat), because nothing is written in that case.

Regression test: `a_rejected_run_does_not_seal_its_correction` in
`crates/process/tests/close_year.rs`; it failed on main (the correction was
sealed and attributed to 2026).

## Decisions for later review

- The refusal is tied to the gate, not to the Run status. That covers a Run
  rejected at `ApproveAdjustments` and a Run whose `ApproveAdjustments` was
  approved, where every correction is already sealed by the existing
  precondition, so nothing legitimate is lost. Alternative: check
  `RunStatus::Rejected`; it would let a correction through on an approved Run
  whose state somehow gained an unsealed correction.
