---
created: 2026-11-12
updated: 2026-11-12
type: feature
reporter: jari
status: open
priority: high
related: ['@contra-accounts-fiscal-periods']
preparation: ready
lane: year-end
lane_seq: 0
collision: [crates/ledger, crates/domain/src/statements.rs]
---

# Support fiscal years that do not follow the calendar year

## Description

aggountant treats every fiscal year as a calendar year: the year is an integer
throughout the ledger (year state, adjustment attribution, opening balances,
statements). Many Finnish companies have a fiscal year that does not follow
the calendar year, and a company's first or last fiscal year, or the year in
which it changes its fiscal year, can be 6 to 18 months (kirjanpitolaki 1:4).
Such a company cannot use aggountant today, so this is a gate for early users,
split out from `contra-accounts-fiscal-periods`.

Store a fiscal-year definition (start, end, constituent months) in master
data, set at `init` and for opening balances, and use it wherever the code
now derives the year from a date: year state, attribution of year-end
adjustments, opening balances, reports and year-to-date figures. Monthly
periods and VAT periods are unchanged.

## Acceptance Criteria

- [ ] A synthetic company with a fiscal year 1.7.–30.6. can book, close months, see year-to-date figures for its own year and record opening balances.
- [ ] A first fiscal year of up to 18 months is accepted; a longer one is rejected with a clear error citing the rule.
- [ ] Existing calendar-year homes keep working without migration steps by the user.
- [ ] The user guide (fi and en) drops the calendar-year limitation.
