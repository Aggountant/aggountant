# Result: user guide for non-calendar fiscal years

The guide changes listed in `issues/non-calendar-fiscal-year/result.md`
("User guide changes to apply") are applied to `docs/user/en` and
`docs/user/fi`: periods, status, glossary, is-it-for-you, set-up-company,
commands, error codes and year-end. The guide no longer says the fiscal year
must be the calendar year; it documents `fiscal-year define`, `list` and
`show`, and the error codes `FISCAL_YEAR_CONFLICT`, `FISCAL_YEAR_UNDEFINED`
and `INVALID_FISCAL_YEAR`, plus the extended `PERIOD_TRANSITION_CONFLICT`
row.

Commands and messages were checked against a build of this tree:
`fiscal-year --help`, `define`, `show` (prints the dates and the length in
months), `list`, and the `INVALID_FISCAL_YEAR` and `FISCAL_YEAR_UNDEFINED`
refusals.

## Decisions for later review

- **Year-end step 2.** The heading and the explanatory sentence now speak of
  the fiscal year's last month; the example prompt ("Make a monthly report for
  December") is kept, because the guide's Esimerkki Oy has a calendar year.
- **Set-up-company additions placed inline.** The step 3 sentence is appended
  to the step's paragraph; the step 4 addition is a continuation paragraph of
  the voucher-date bullet rather than a new bullet, since it qualifies that
  rule.
- **Heading changes kept in step** (year-end step 2 in both languages); the
  language-pair check passes.
