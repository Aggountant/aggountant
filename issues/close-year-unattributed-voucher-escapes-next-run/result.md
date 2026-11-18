# Result

`assert_no_unplanned_attributions` in `crates/process/src/close_year.rs` became
`assert_no_unplanned_close_year_vouchers`: besides a voucher attributed to the
closing year outside the Run's plan, it now refuses with `CLOSE_YEAR_BLOCKED`
while a sealed voucher whose natural key starts with `closeyear:<year>:` has no
attribution and is not in the plan, the state an abandoned Run leaves when it
stopped between sealing and attributing. The message lists the keys with their
voucher numbers and says the way out: start a run whose input includes the same
asset, accrual or correction with the same figures (its seal replays by natural
key and attributes the voucher), and if the entry was wrong, include it anyway
and reverse it with a correction in that run. It runs where the old check ran:
at start, in the `ApproveAdjustments` approval precondition, and last in the
statements binding that `run_seal_year` re-runs before `Ledger::seal_year`.

The ledger gets one read, `Ledger::unattributed_vouchers_with_key_prefix`.

Regression tests in `crates/process/tests/close_year.rs`, both failing on the
previous code:

- `a_run_without_a_voucher_an_abandoned_run_never_attributed_is_refused_at_start`
  (the issue's sequence; re-supplying kone-1 then completes the close with the
  depreciation in 2026 and nothing left in 2027);
- `statements_are_not_sealed_while_another_runs_voucher_is_unattributed`
  (the orphan appears after the statements are computed; the statements hash is
  unchanged, so only the new check stops the seal; the gate stays rejectable).

User guide fi + en (`tasks/year-end.md`): the paragraph on earlier unfinished
runs now covers an entry recorded in the new year but not yet assigned to the
closing year, the refusal to seal, and reversing a wrong entry by a correction.

Review and assessment: `review.md`, `assessment.md` beside this file. No
confirmed findings.

## Decisions for later review

- The guard lives in the process, not in `Ledger::seal_year`. A ledger check
  would hard-code the close-year key namespace in the ledger, and
  `run_seal_year`, the only caller, already runs the binding immediately
  before. Alternative: an additive check in `seal_year` taking the prefix as an
  argument; not needed while there is one caller.
- Every unattributed voucher with the prefix counts, regardless of the year it
  is posted in. The Run's own planned keys are excluded because
  `assert_run_vouchers_attributed` already refuses them until attributed and a
  resume or a repeated seal attributes them.
- A voucher sealed by hand (`voucher seal`) with a `closeyear:<year>:` key
  would block the close with no way out other than matching a run entry. Left
  as is: the namespace is the close-year run's own and the failure is loud.
- A run interrupted after approving `ApproveStatements` that then meets such a
  voucher has no gate left to reject (review F1). The same already holds for
  the other binding refusals on resume; the year stays open and the run resolves
  as superseded once another run seals the year. Not changed here.
