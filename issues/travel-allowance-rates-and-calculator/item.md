---
created: 2026-11-24
updated: 2026-11-27
type: feature
status: done
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:travel-allowance-rates-and-calculator
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
lane: travel-calc
lane_seq: 10
closed: 2026-11-27
closed_by: spinoff-01m8ehz12nsg309tcjd5yjhp71
---

# Compute Finnish travel allowances from a trip (rate table and calculator preview)

## Description

## Goal

The program computes Finnish scheduled travel allowances for a trip, so the
user (and the product owner trying it) can give a synthetic trip as JSON and
see the travel days, per diem, mileage, meal and night allowances, the
Incomes Register lines and the proposed postings, with the rate-table edition
used. Nothing is written. Follows ADR-11 (`docs/site/adr-11-expense-claim.md`,
sections 2 and 5); this issue owns the computation, the refusals and the
defaults below.

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
- Meals (product owner 2026-11-26, ADR-11 section 2): a free meal is one
  meal whatever its price, and the per diem is halved by the number of free
  meals. A company-paid own meal (bought by the traveller, paid by the
  company, for example with the company's card) is not a free meal: the
  tax-free per diem is the per diem minus the meal's cost, but never less
  than half the per diem (VH/630/00.01.00/2026, 8.3.3 and example 60). An
  expensive meal therefore ends at half the per diem and is not refused.
- Leisure combined with a trip (product owner 2026-11-26, ADR-11 section 2;
  guidance 8.3.2): computed from the work time and the travel times the user
  states. The number of per diems comes from that duration; the free meals
  and, abroad, the country are answered for each computed per diem day, not
  by the clock (the draft asks them per computed day). Not refused in the
  first version.
- The edition is chosen by the date of the part of the trip (decision
  19 §), never by payment or approval date; the table hash and decision
  identifiers are stamped into the output. Every new edition ships test
  vectors, including a trip across the year boundary.
- Refusals, each with a machine-readable code and an explanation; a refusal
  blocks approval, never the draft write:
  - an amount above the scheduled maximum (`ALLOWANCE_ABOVE_MAXIMUM`), with
    the explanation that the excess is pay and needs withholding, which the
    product cannot do yet (product owner 2026-11-24);
  - a missing rate-table edition (`TRAVEL_RATES_MISSING`); an invalid country
    code (never falls to the "not separately listed" row);
  - free meals not answered on a trip that earns a per diem (an empty answer
    is not "no free meals"); for a meal allowance, the usual-eating-place and
    lunch-benefit answers not given;
  - a meal allowance with a per diem; a night travel allowance with lodging
    paid for that night;
  - the cases the sources do not settle: a free meal and a company-paid meal
    on the same travel day, a foreign trip that passes through Finland and
    continues abroad, a trip that starts from a holiday home, a workplace
    change inside a trip (split the trip), border-crossing allowance, posting
    (komennus), and work abroad. A company-paid meal above half the per diem
    and leisure combined with a trip were on this list until the product
    owner decided them on 2026-11-26 (above).
- Rules where the sources are silent, shown in the preview when the
  result depends on them: a travel day across the year boundary gets the
  lower of the two years' amounts (product owner 2026-11-26, formerly ADR-11
  open question 1); a travel day is 24 hours of elapsed time, so a
  daylight-saving change counts as it happened; rounding once per claim,
  half away from zero (product owner 2026-11-26, formerly open question 3).
- `crates/AGENTS.md` gains the product rule ADR-11 adopted (product owner
  2026-11-26): a Run exists only for the program's own irreversible outside
  act (a filing to an authority, a payment). This is the first claim slice
  in the ADR's order; if another claim slice lands first, that slice adds
  the rule instead.
- A read-only CLI preview, for example `claim compute --json < trip.json`.

## Acceptance criteria

- [x] Golden vectors for synthetic trips covering: a partial and a full
      domestic day, exactly 6 h (no per diem), a two-day trip with a tied
      2 h last day, halving by free meals, a foreign trip with return-day
      rule, an under-10-hour foreign trip, a year-boundary trip, a
      company-paid meal deduction, a company-paid meal costing more than
      half the per diem (ends at half, no refusal), two free meals of
      different prices (halving by count), a trip combined with leisure
      (per diems from the stated work and travel times), mileage rounding
      (125.6 km reported as 126).
- [x] The Tax Administration guidance examples 56, 57, 59, 61 and 63 reproduce.
- [x] The foreign table is checked against the decision (row count and spot checks); an invalid country code refuses instead of falling to "not listed".
- [x] A missing year edition refuses; nothing is extrapolated.
- [x] Output stamps the table hash and decision identifiers.
- [x] Every refusal above has a code and a test with a synthetic trip.
- [x] `crates/AGENTS.md` carries the Run rule (unless an earlier claim slice already added it).
- [x] User guide (fi/en) describes the preview.
- [x] The rate table is loaded only when a claim is computed; commands unrelated to claims do not read it (test).

## Likely files

`crates/domain/src/travel/` (new), `crates/domain/data/fi-travel-allowances.json`,
`crates/domain/src/lib.rs`, a new `crates/cli/src/claim.rs`, `crates/cli/src/main.rs`,
`docs/user/{fi,en}/`.

## Dependencies

None. The regular-workplace refusal joins when the register slice
(`claimant-workplace-register`) lands; until then the preview takes the
trip's workplace classification as input and says so.

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Accepted 2026-11-26); decision issue `expense-claim-model`. The ADR states the principle; the details above are this issue's own. Dependencies above are prose; the conductor sets `blocked_by` after triage.

## Decisions

### 2026-11-26T06:00:21Z · @spinoff-01m8b6z13skk6403cxj7xcj1nh

Product owner 2026-11-26: companies that do not use expense claims must see no questions, changes or slowness from them (ADR-11 Consequences, 'Companies that do not use claims'). The acceptance criteria added today make this slice's share of that property testable.

### 2026-11-27T17:52:58Z · @conductor

Product owner 2026-11-27: keep the refusals the calculator chose where no source settles the case (night travel allowance abroad, company-paid meal on the half per diem of the return day, foreign days ending in Finland, Macao) until a real case needs a computation.

## Resolution

### 2026-11-27T13:52:11Z · @spinoff-01m8ehz12nsg309tcjd5yjhp71

Landed: claim compute preview, 2026 and 2027 rate editions from decisions 970/2025 and 929/2026, refusals with codes, Run rule in crates/AGENTS.md, guide pages. Review and assessment in this directory; see result.md.
