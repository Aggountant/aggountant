# Assessment of the guide-new-year-reports-wait-seal review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | `--help` overstated the opening-voucher exemption | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX — applied |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: confirmed against `boundary_cutover`. A migrating company that closed
  its cutover month would meet it. Fixed in the help text; the guide pages
  link to the setup page, which already states the conditions in full, so
  they were left at one sentence.

No follow-up issue was filed.

## Simplicity

- Reuses the existing `PERIOD_NOT_SEALED` code and the ledger's own reason
  string verbatim; the CLI adds context in front rather than a new error code
  or a ledger change, because the ledger message is also embedded in the
  month-close content note, where "make the report again" would be wrong.
- The guide keeps one home for the fact (`tasks/monthly-report.md`, "If
  something goes wrong"); `year-end.md`, the error-code row and the skill give
  it one sentence with a link.
- Left out: a fallback report that omits opening balances. The issue asked
  for an explanation, not a new report mode.
