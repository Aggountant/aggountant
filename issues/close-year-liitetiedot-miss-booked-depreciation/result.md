# Result

`assert_no_unplanned_attributions` in `crates/process/src/close_year.rs`
refuses with `CLOSE_YEAR_BLOCKED`, listing the natural keys, while a voucher
attributed to the closing year is not in the Run's plan (depreciation lines,
accruals, corrections). It is the converse of `assert_run_vouchers_attributed`;
both now share `planned_natural_keys` and `attributed_natural_keys`, which
replace the inline construction the old check had. It runs:

- in `start_close_year`, before the Run directory is created (the issue's
  scenario: an abandoned Run already booked depreciation);
- in the `ApproveAdjustments` approval precondition, before the answer is
  recorded, so a Run started before another Run booked an entry keeps its gate
  open and can be rejected;
- last in `assert_statements_binding`, as the backstop before `SealYear`.

It deliberately does not run in `run_compute_statements`: that step follows
an answered `ApproveAdjustments`, so a resumed Run refused there would have no
open gate left to reject (found by the review; test
`a_resumed_run_with_an_entry_outside_its_plan_can_still_be_rejected`).

The user guide (fi + en, `tasks/year-end.md`) says that a new run must include
what an earlier unfinished run already recorded, and that other open runs can
only be rejected once one run has sealed the year (the latter is the behaviour
of `close-year-seal-after-year-sealed`, documented here so the paragraph reads
as one).

Regression tests: `a_run_without_depreciation_another_run_booked_is_refused_at_start`
and `approving_adjustments_is_refused_while_another_runs_entry_is_outside_the_plan`
in `crates/process/tests/close_year.rs`; both failed on main.

## Decisions for later review

- Every attribution to the year counts, not only `closeyear:<year>:` keys. Only
  close-year Runs call `Ledger::attribute_adjustment`, so the two are the same
  set today, and a prefix filter would be a rule nobody needs.
- In `assert_statements_binding` the check runs after the hash checks, so an
  existing refusal (`JOURNAL_CORRUPT` for statements or breakdowns made stale
  by another Run) keeps its code and message. Alternative: check first and
  report every such case as an unplanned entry.
- The refusal does not try to recover the missing input from the ledger (for
  example re-deriving an asset from the depreciation voucher): the agent
  re-supplies the asset, accrual or correction, and its seal replays by
  natural key; a re-supplied entry with different figures is refused by the
  ledger's natural-key conflict check.
