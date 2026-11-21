# Result: guide-new-year-reports-wait-seal

`report statements` for a month of a new fiscal year still refuses with
`PERIOD_NOT_SEALED` (kind `user`, exit 1) while the previous year is not
sealed, but the message now says why and what lifts it:

> report statements for 2027-01 needs its fiscal year's opening balances, and
> they exist only once the previous fiscal year is sealed by its year-end
> close (…ledger reason…). Way out: finish the previous year's `close-year`
> run (its last approval, ApproveStatements, seals the year and cannot be
> undone), then make the report again. Nothing is wrong with the books
> meanwhile: the new year's months can still be recorded, closed with
> `close-period` and their VAT filed; only this report waits.

- `crates/cli/src/reports.rs`: the message, and a NEW FISCAL YEAR section in
  `report statements --help` with the exact exemption conditions.
- `crates/cli/tests/monthly_reports.rs`: the refusal's code, kind, exit and
  wording; the previous year's month still reports.
- Guide (fi, en): `tasks/monthly-report.md` (prerequisite and an "If
  something goes wrong" entry), `tasks/year-end.md` (one paragraph in the
  close-year step), the `PERIOD_NOT_SEALED` row in `reference/error-codes.md`
  (so `explain` and the error `hint` carry it too).
- `skills/capture-receipt/SKILL.md`: the reconciliation section tells the
  agent the refusal is not a bookkeeping error.

Review and assessment: `review.md`, `assessment.md` (one finding, fixed).
