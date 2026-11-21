# Review: virtually-tidy-owl

- Skill: `/llm-review`, rigor **routine**. The change does not move money or
  send anything, but it decides what the user sees before the most frequent
  permanent act (closing a month) and adds a new refusal to that path, so
  availability of the close and fidelity of the shown figures were checked.
- Scope: the branch against `3fe66a0` (`crates/process/src/close_period.rs`
  new, `crates/process/src/lib.rs`, `error.rs`, `crates/cli/src/main.rs`,
  tests, user guide fi+en, `skills/capture-receipt/SKILL.md`).
- Finder: `gpt-6-astra`, one call (diff plus `close_period.rs`, `lib.rs`,
  `ledger/src/reports.rs`, `obligation.rs`). Verifier: not called; the two
  surviving candidates were confirmed by reading the code and by tests written
  for them, and the third was dropped on local evidence. One model call.
- Risk class: availability of the month close; figures shown at an approval.

## F1 (must-fix, found while checking finder candidate 3): the close required the previous fiscal year to be sealed

- Location: `crates/process/src/close_period.rs`, `close_period_content`
  (`ledger.fiscal_year_data(...)`), reached from `start_close_period_seam`
  before `begin_close`.
- Claim: `Ledger::fiscal_year_data` refuses with `PERIOD_NOT_SEALED` when
  the previous fiscal year has activity and is not sealed
  (`crates/ledger/src/lib.rs`, opening-source resolution). Building the
  content before every close therefore refused to close January–(year-end
  close) of every company whose previous year is kept in aggountant, which is
  the normal state for months: January's VAT is due in February, the year-end
  close months later.
- Evidence: the finder's candidate 3 cited this refusal for a contrived resume
  window; reading the ledger showed it fires on every start. Test
  `january_closes_while_the_previous_fiscal_year_is_still_open`.
- Fix: on `PERIOD_NOT_SEALED` the content is built from the year's own
  vouchers without the opening position, and `balance_sheet.opening_note`
  says the balances are this fiscal year's movements only and why. The period
  income and year-to-date result do not depend on the opening position.

## F2 (should-fix): a resumed start skipped the pre-commit content check

- Location: `crates/process/src/lib.rs`, `resume`, the
  crash-before-fiscal-commit retry of `begin_close`.
- Claim: a fresh start builds the content before `begin_close`, so a failure
  leaves the period Open; the resume retry checked only obligations, so the
  same failure would leave the period Closing with no gate to reject.
- Evidence: local read. Fix: the retry calls `close_period_content` with the
  obligation check before `begin_close`.

## F3 (should-fix): `ready: true` in the preview did not cover the ledger's coverage rules

- Location: `crates/process/src/close_period.rs`, `preview_close_period`.
- Claim: the ledger also refuses a close that mixes months and quarters in one
  year, a quarter in a non-calendar fiscal year, or a period of a sealed year
  (`crates/ledger/src/store/sqlite_fiscal.rs`, `transition_period`); the
  preview checks only the period's own state and obligations.
- Evidence: local read; the refusal at start is clean
  (`PERIOD_TRANSITION_CONFLICT`, nothing changes). Exposing those checks
  read-only would need a ledger refactor outside this slice.
- Fix: the field's documentation says what `ready` covers and that the
  coverage rules are checked at start.

## Dropped (1)

- Finder candidate 1, "resume seals without rechecking the approved content"
  (`lib.rs`, approve-then-crash recovery). The approval was checked against
  the content when it was given; what the seal locks is the period's own
  vouchers, which the Closing state keeps unchanged. The window needs a crash
  between `GateAnswered` and `SealPeriod` and then a write to an earlier
  open month before `resume`, and refusing there would strand a Run whose
  gate is already answered. Dropped as a blue-moon window whose end result
  (July sealed with exactly the vouchers approved) is correct.

## Coverage

Read: the whole diff, the ledger's `fiscal_year_data`, `year_vouchers`,
`monthly_statements_report`, `transition_period`, the obligation store.
Tested: process tests for preview/gate equality, changed content refusal,
blockers, legacy gate, cross-year quarter, unsealed previous year; CLI test
for preview, start, show (JSON and human); the month result, year-to-date
result and assets total are compared with `monthly_statements_report`.
