# Review: guide-new-year-reports-wait-seal

- Skill: `/llm-review`, rigor **routine**: the change rewords a refusal
  message and guide text; it changes no bookkeeping state, error code, error
  kind or exit code.
- Scope: the uncommitted slice on top of `b043891`: `crates/cli/src/reports.rs`
  (refusal message, `--help`), `crates/cli/tests/monthly_reports.rs`, guide
  pages `tasks/monthly-report.md`, `tasks/year-end.md`,
  `reference/error-codes.md` (fi, en), `skills/capture-receipt/SKILL.md`.
- Finder: `gpt-6-astra`, one call with the diff, pointed at `fiscal_year_data`,
  `boundary_cutover`, `monthly_statements_report`, `close_period.rs` and the
  CLI error mapping. Verifier: none (one candidate, confirmed locally). One
  model call in total.
- Risk class: user-facing text about the year-end seal (irreversible step).

## F1 (should-fix): `--help` overstated the opening-voucher exemption

- Location: `crates/cli/src/reports.rs`, `STATEMENTS_HELP`, section
  NEW FISCAL YEAR.
- Claim: the draft said a year "opened with opening-balance vouchers dated the
  previous year's last day does not wait". `boundary_cutover`
  (`crates/ledger/src/lib.rs`) also requires every prior voucher to be
  balance-sheet-only and no prior period to have been operated in aggountant
  (any transition, even a reopened close); `opening_balances.rs` tests the
  refusal.
- Evidence: local read of `boundary_cutover` confirms both extra conditions.
- Impact: an agent reading `--help` could expect a January report after
  closing a cutover December, and misread the refusal.
- Fix: the help now names all three conditions.

## Checked and holding

- The only `PeriodNotSealed` reachable from `monthly_statements_report` is the
  opening-balance gate in `fiscal_year_data`, so the rewording is specific.
- `close-period` tolerates the gate (`close_period.rs`, opening note), and VAT
  filing checks only the period's own seal; the "months can still be closed
  and VAT-filed" claim holds.
- Code `PERIOD_NOT_SEALED`, kind `user`, exit 1 unchanged (new CLI test).
- fi/en additions agree; the error-code table keeps the format `explain.rs`
  tests (ci-local green).

Dropped candidates: none.
