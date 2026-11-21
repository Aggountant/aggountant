# Result: the month close shows what the user approves

## What changed

- New read command `aggountant run preview close-period --period YYYY-MM`
  (also `YYYY-Qn`): returns the close content, its `content_hash`, the
  period's state, `ready` and `blockers` (period not Open, open obligations),
  a `message` and a `next_command`. It changes nothing.
- The content (`ClosePeriodContent`, `crates/process/src/close_period.rs`):
  the period's dates; voucher count and first/last number; the period's
  income statement (non-zero rows), its result and the year-to-date result;
  the balance sheet at the period's last day (non-zero rows, per-account
  closing balances, totals, `balanced`); open obligations for the period;
  `locks` (what approving seals); for a month, the `report statements`
  command with the same figures. When the previous fiscal year is not sealed,
  the balances are this year's movements and `balance_sheet.opening_note`
  says so; the close does not wait for the year-end close.
- `ApproveClose`'s `snapshot_hash` now covers the content hash besides the
  period lock. `run start`, `run show`, `run answer` and `run resume` of a
  close-period Run waiting at the gate add `open_gate` (`content`,
  `content_hash`, `bound`), `message` and `next_command`. Existing fields are
  unchanged.
- Approving recomputes the hash; if the books changed after the gate opened
  (an earlier open month's entry, an obligation, the chart), it refuses with
  the new code `CLOSE_CONTENT_CHANGED`; the close is rejected and started
  again. A gate opened by an older binary is still approved under its old
  hash, and its content is shown with `bound: false`.
- The content is built before `begin_close` on start and on the resumed
  retry, so a failure leaves the period Open.
- Skill `capture-receipt`: the passage saying the gate shows only a hash and
  the agent must assemble the proposal is replaced; the agent now shows the
  gate content and adds what the program does not know (statement match,
  payables explanation, what stays open). User guide fi+en: close-month,
  practice month, commands, error codes.

## Review

`review.md` and `assessment.md` beside this issue. Fixed here: the content
first required the previous fiscal year to be sealed, which would have blocked
closing a new year's months (found via the finder's resume candidate); the
resumed start's missing content check; the meaning of `ready`.

## Decisions for later review

- **Rebuild, not persist, the content.** Closing freezes the period's
  vouchers; anything else that changes the content should refuse the approval,
  so rebuilding and comparing hashes is enough and adds no state file.
  Alternative: store the content in the Run like close-year.
- **No workflow version bump.** In-flight Runs keep closing under their
  period-lock hash instead of needing `run migrate`.
- **`next_command` with `approve|reject`.** The answer is the user's choice;
  the command is a template, not a default. When the content changed, it is
  the literal reject command.
- **Content failure blocks the close.** Apart from the unsealed previous year
  (handled), the content fails only when statements cannot be computed at all,
  which also breaks the monthly report and the year-end close.
