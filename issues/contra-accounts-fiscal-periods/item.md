---
created: 2026-11-10
updated: 2026-11-12
type: improvement
status: open
priority: normal
lane: year-end
lane_seq: 20
collision: [crates/ledger, crates/process/src/close_year.rs]
---

# Contra-account kind and non-calendar fiscal years

## Problem

Two limits of the year-end model:

1. **Contra accounts.** Accumulated-depreciation accounts work today through a
   signed balance on `AccountKind::Asset`. An explicit contra kind
   (`ContraAsset`, `ContraLiability`) or a chart-declared normal sign would be
   less fragile than relying on the sign of the balance.
2. **Non-calendar fiscal years.** A fiscal year is a calendar year made of
   twelve months (or four quarters in older homes). Finnish companies may have
   a fiscal year that does not follow the calendar year, and a first or last
   fiscal year of 6 to 18 months.

## Expected

A fiscal-year definition (start, end, constituent periods) stored in master
data and used by `close-year`, and an explicit contra-account kind or sign in
the chart.

## Acceptance criteria

- [ ] A synthetic company with a fiscal year 1.7.–30.6. can close its year.
- [ ] An 18-month first fiscal year is accepted; a longer one is rejected with a clear error.
- [ ] Accumulated depreciation is presented correctly without relying on a negative asset balance.
- [ ] Primary-source citation (kirjanpitolaki) for the allowed fiscal-year lengths.

## Comments

### 2026-11-12T09:31:13Z · @conductor

Non-calendar fiscal years split out to @non-calendar-fiscal-year (high priority, 12.11.2026); this issue keeps the contra-account kind.
