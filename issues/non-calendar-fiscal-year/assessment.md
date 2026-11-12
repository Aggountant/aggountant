# Assessment of the final-diff review

Source: `review.md` beside this file (2026-11-12, finder `gpt-6-astra`,
verifier `claude-fable-5-1`).

| Finding | Classification | Likelihood on real data | Outcome | Status |
| --- | --- | --- | --- | --- |
| F1 definition removes an attributed year | CONFIRMED (failing test) | Low: needs an attribution and a later change of fiscal year, but the result is silently wrong statements | FIX | fixed, test green |
| F2 attribution to a nonexistent year | CONFIRMED (failing test) | Medium: an agent may pass a July company's starting calendar year as the label | FIX | fixed, test green |
| F3 definition strands active quarters | CONFIRMED (failing test) | Medium for quarterly-VAT companies changing their fiscal year | FIX | fixed, test green |
| F4 quarter closed inside a non-calendar year | CONFIRMED (failing test) | Medium: a quarterly-VAT July company would try exactly this | FIX | fixed, test green |

F1 and F2 are data-integrity findings and blocked the merge until fixed; F3 and
F4 would have left a year permanently unsealable once the quarter's VAT return
was filed. All four are fixed in this slice, so nothing is filed as a new issue.

## Simplicity

- **Reused:** the `i32` fiscal-year identity everywhere it is stored (year
  rows, transition log, attributions, opening sources, statements, archive
  paths, close-year natural keys). A calendar year's label is its own number,
  so no data migrates and no wire format changes.
- **Repetition removed:** three ledger copies of "posting year =
  `PeriodId::containing(date).year()`", two domain filters on the calendar year,
  the `year-12-31` re-date and the close-year `year + 1`/`12-31` arithmetic now
  all go through one `FiscalCalendar`. The quarter rule ("a quarter only in a
  calendar year that is wholly one calendar fiscal year") is one helper that the
  define path and the period-transition path both call.
- **Remaining repetition, justified:** the quarter-overlap check in
  `year_close_periods` duplicates the two guards defensively, for fiscal stores
  written in some way the guards did not see. It refuses and never guesses.
- **Left out on purpose:** a fiscal-year id type through storage and wire;
  changing or deleting definitions; two fiscal years ending in the same
  calendar year; quarterly closing of non-calendar years; prorated depreciation;
  a separate opening-balance command; `init` flags (setup collects the fiscal
  year after `init`).
