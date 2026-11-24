---
created: 2026-11-24
updated: 2026-11-24
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:travel-allowance-rates-and-calculator
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
---

# Compute Finnish travel allowances from a trip (rate table and calculator preview)

## Description

## Goal

The program computes Finnish scheduled travel allowances for a trip, so the
user (and the product owner trying it) can give a synthetic trip as JSON and
see the travel days, per diem, mileage, meal and night allowances, the
Incomes Register lines and the proposed postings, with the rate-table edition
used. Nothing is written. Decided in ADR-11 (`docs/site/adr-11-expense-claim.md`,
sections 1 and 4).

## Scope

- `crates/domain/data/fi-travel-allowances.json`: one edition per year (2026
  from decision 970/2025, 2027 from VH/5639/00.01.00/2026), each with
  statute and diary number, issue date, URL, domestic amounts, mileage and
  surcharges, and the foreign per diem keyed by ISO 3166 code with sub-keys
  for the cities and regions the decision lists, plus the "not separately
  listed" row.
- A pure `domain::travel::fi` module: mileage (R1 surcharges, passengers,
  car-benefit fuel), travel days from elapsed time, the 15 km / 5 km test,
  halving by free meals, foreign per diem by travel-day end country with the
  return-day and under-10-hour rules, meal allowance (needs the
  usual-eating-place and lunch-benefit answers, decision 14 §), night
  allowance (4 h between 21 and 07, no lodging provided), company-paid meal
  deduction (ohje example 60), rounding once per claim half away from zero.
- Refusals with machine-readable codes for the unsettled cases listed in
  ADR-11 section 4 (free and company-paid meal on one day, company-paid meal
  above half, foreign trip through Finland, holiday-home start, leisure,
  border-crossing allowance, posting, work abroad), `TRAVEL_RATES_MISSING`,
  `ALLOWANCE_ABOVE_MAXIMUM`.
- Defaults shown when the result depends on them: lower amount for a travel
  day across the year boundary; elapsed time across a daylight-saving change.
- A read-only CLI preview, for example `claim compute --json < trip.json`.

## Acceptance criteria

- [ ] Golden vectors for synthetic trips covering: a partial and a full
      domestic day, exactly 6 h (no per diem), a two-day trip with a tied
      2 h last day, halving by free meals, a foreign trip with return-day
      rule, an under-10-hour foreign trip, a year-boundary trip, a
      company-paid meal deduction, mileage rounding (125.6 km reported as 126).
- [ ] The Tax Administration guidance examples 56, 57, 59, 61 and 63 reproduce.
- [ ] The foreign table is checked against the decision (row count and spot checks); an invalid country code refuses instead of falling to "not listed".
- [ ] A missing year edition refuses; nothing is extrapolated.
- [ ] Output stamps the table hash and decision identifiers.
- [ ] User guide (fi/en) describes the preview.

## Likely files

`crates/domain/src/travel/` (new), `crates/domain/data/fi-travel-allowances.json`,
`crates/domain/src/lib.rs`, a new `crates/cli/src/claim.rs`, `crates/cli/src/main.rs`,
`docs/user/{fi,en}/`.

## Dependencies

None. The regular-workplace refusal joins when the register slice
(`claimant-workplace-register`) lands; until then the preview takes the
trip's workplace classification as input and says so.

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Proposed 2026-11-24); decision issue `expense-claim-model`. Dependencies above are prose; the conductor sets `blocked_by` after triage.
