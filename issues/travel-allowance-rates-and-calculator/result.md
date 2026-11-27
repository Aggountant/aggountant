# Result: travel-allowance-rates-and-calculator

`aggountant claim compute` reads the trips of one claim as JSON on stdin and
prints the computed scheduled tax-free travel allowances. It writes nothing
and needs no company home.

```
aggountant --json claim compute < trip.json
```

## What was built

- `crates/domain/data/fi-travel-allowances.json`: editions 2026 (decision
  970/2025, VH/6575/00.01.00/2025) and 2027 (decision 929/2026,
  VH/5639/00.01.00/2026), each with statute, diary number, issue and
  publication dates, Finlex and vero.fi URLs, mileage rates and surcharges,
  domestic amounts and the foreign per diem table keyed by ISO 3166 code,
  with the decision's cities and regions as sub-keys and the "not separately
  listed" row. The territory definitions the decisions attach (Kanaalisaaret
  and Man to Britain, Vatican to Italy, Hong Kong under China; Macao
  unsettled; FI and AX domestic) are in the same file. The foreign tables were
  extracted from the Finlex PDFs and compared row by row with the vero.fi
  rendering of each decision; 211 rows each.
- `crates/domain/src/travel/fi/`: the pure computation.
  `compute_claim(&ClaimInput) -> Result<ClaimComputation, DomainError>`.
  Travel days from elapsed time, the 15 km / 5 km test, domestic per diems
  (over 6 h partial, over 10 h full; after full days at least 2 h partial and
  over 6 h full), foreign per diems by the country where each full day ends
  with the return rules (over 10 h full, over 2 h half, under 10 h total
  domestic, 10–24 h the destination), halving by free meals (count, not
  price), the company-paid meal deduction down to half, meal allowance (three
  answers), night travel allowance (4 h between 21 and 07, no provided
  lodging), mileage with surcharges, passengers, forest road and car benefit,
  leisure from the stated work and travel time, the year-boundary lower
  amount, and rounding once per claim half away from zero. The output stamps
  the table hash and the decisions used, and lists the Incomes Register lines
  (311 with kilometres, 331 with per diem types) and the proposed postings
  per month (7300, 7350 against 2740).
- Refusals, each with its own code and a test: `ALLOWANCE_ABOVE_MAXIMUM`,
  `TRAVEL_RATES_MISSING`, `TRAVEL_COUNTRY_INVALID`,
  `TRAVEL_COUNTRY_UNANSWERED`, `TRAVEL_TERRITORY_UNSETTLED`,
  `TRAVEL_FREE_MEALS_UNANSWERED`, `TRAVEL_MEAL_ALLOWANCE_UNANSWERED`,
  `TRAVEL_MEAL_ALLOWANCE_WITH_PER_DIEM`, `TRAVEL_NIGHT_LODGING_UNANSWERED`,
  `TRAVEL_NIGHT_ALLOWANCE_WITH_LODGING`, `TRAVEL_NIGHT_ALLOWANCE_WITH_LEISURE`,
  `TRAVEL_NIGHT_ALLOWANCE_ABROAD`, `TRAVEL_FREE_AND_PAID_MEAL_SAME_DAY`,
  `TRAVEL_PAID_MEAL_ON_RETURN_HALF`, `TRAVEL_FOREIGN_DAYS_IN_FINLAND`,
  `TRAVEL_FROM_HOLIDAY_HOME`, `TRAVEL_WORKPLACE_CHANGE`,
  `TRAVEL_BORDER_CROSSING_ALLOWANCE`, `TRAVEL_POSTING`, `TRAVEL_WORK_ABROAD`,
  `TRAVEL_SPECIAL_INDUSTRY`, `TRAVEL_TO_REGULAR_WORKPLACE`. Malformed input is
  `TRAVEL_INPUT_INVALID`.
- `crates/cli/src/claim.rs`: the `claim compute` command, its INPUT help and
  the test that unrelated commands never read the rate table (a thread-local
  read count) and that the preview leaves a synthetic home's command output and
  files identical.
- `crates/AGENTS.md` carries the Run rule; `crates/domain/AGENTS.md` the rules
  for keeping the travel table (primary source, a new edition every autumn
  with year-boundary vectors, unsettled cases refused).
- User guide: "Laske matkan verovapaat korvaukset" / "Compute the tax-free
  allowances of a trip", status, commands and error-code entries; CHANGELOG.

## Decisions taken here

- Times are RFC 3339 with an explicit offset; Finnish local time is printed
  back. Night hours (21–07) are counted in Finnish time, and a night travel
  allowance on a foreign trip is refused because the decision does not say
  which time zone counts abroad.
- A company-paid meal on the half foreign per diem of the return is refused:
  free meals do not halve that half, so the 8.3.3 floor has no settled base.
- A foreign trip with no full day ending abroad, or with a day ending in
  Finland before a later one ends abroad, is refused
  (`TRAVEL_FOREIGN_DAYS_IN_FINLAND`).
- An employer-arranged meal with two meal allowances gives zero meal
  allowance, not one.
- `paid` is per trip; the claim is also checked against its once-rounded
  maximum. Monthly vouchers book the rounded running total, so they add up to
  the claim.
- The Incomes Register 331 line carries one amount with every per diem type
  it holds (`allowance_codes`), as the register's guidance describes; today's
  `file-wage` takes one code per line until `wage-report-multiple-allowance-codes`
  lands.

## For the register slice

`claimant-workplace-register` supplies the trip's classification as
`TripInput.workplace` (`aggountant_domain::travel::fi::Workplace`:
`SpecialWorkPlace`, `SecondaryWorkplace`, `RegularWorkplace`) from its
snapshot for the trip's start date, then calls
`aggountant_domain::travel::fi::compute_claim`. The note
`workplace_from_trip_input` is added in `trip::compute`; remove it when the
classification comes from the register.

## Review

`review.md` and `assessment.md`: seven findings, all confirmed and fixed.
