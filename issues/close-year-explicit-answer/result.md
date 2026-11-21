# Result: close-year-explicit-answer

`close-year answer` no longer has a default answer. Without `--answer` it is a
usage error (exit 1) before any run is read, so the approval of
`ApproveStatements`, which seals the fiscal year, can no longer be given by
omission. Existing calls that pass `--answer` behave exactly as before.

The help now:

- names the four gates in the order they open;
- says that approving `ApproveAdjustments` seals the depreciation and accrual
  vouchers permanently, and approving `ApproveStatements` seals the fiscal year
  and cannot be undone;
- says that `reject` ends the run;
- documents every flag of `answer`, `seal-correction`, `resume` and `show`.

Tests: `crates/cli/tests/close_year_cli.rs` (missing `--answer` is refused by
the parser; help names the gates, states the permanence, and every flag has
help text). The user guide already passed `--answer` explicitly; no change.
