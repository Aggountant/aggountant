# Result: close-period-answer-placeholder-pipe

For an open `ApproveClose` gate, `run start` / `run show` now return

    next_command: aggountant run answer --run-id cp-… --gate ApproveClose --answer '<approve|reject>'

A shell passes the quoted placeholder as one argument, which `run answer`
refuses with `INVALID_ANSWER` without touching the period; the message says to
put approve or reject in its place. The stale-content branch still returns
`--answer reject`.

The same sweep reworded the `--help` texts that printed a command-shaped
`a|b` choice: `close-year answer`, `capture-receipt answer`,
`hunt-receipts answer` (found by the review), `run-payroll approve-run` and
`file-vat approve-filing`. The close-month guide (en, fi) says what the
placeholder means.

Tests: `close_period_content_is_shown_and_bound_by_start` asserts the exact
command and that the placeholder is refused with the period still closing;
the CLI test and the unit test assert the exact string. Review and
assessment: `review.md`, `assessment.md`.
