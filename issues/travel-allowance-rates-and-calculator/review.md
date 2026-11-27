# Review: travel-allowance-rates-and-calculator

- Skill: `/llm-review`, rigor **routine**, with the surface treated as
  high-risk: the amounts become sealed claims and Incomes Register reports,
  and an amount above the maximum is taxable pay reported as tax-free.
- Scope: the new `crates/domain/src/travel/fi/` (`mod.rs`, `trip.rs`,
  `mileage.rs`, `rates.rs`), `crates/cli/src/claim.rs` and the golden tests
  `crates/domain/tests/travel_fi.rs`, read against the issue, the text of
  decision 970/2025 and guidance VH/630/00.01.00/2026 chapters 8.3–8.6.
- Finder: `gpt-6-astra`, one call. Verifier: `claude-fable-5-2`, one call,
  asked to refute the seven candidates against the same code and sources; it
  ran synthetic trips through the debug binary. Two model calls in total.
- Risk class: tax-free amounts and authority report contents.

All seven candidates survived local checking and the verifier (A narrowed).
All are fixed in this slice.

## A (should-fix): two meal allowances accepted on a short trip

- Location: `trip.rs` `validate` and `meal_allowance`.
- Claim: guidance 8.6 allows two meal allowances only when the working day
  with breaks is over 11 hours; `meals: 2` was accepted on any trip.
- Evidence: local read; verifier reproduced 27.00 on a 3-hour trip. The
  working time itself is the caller's answer (the input has no working time),
  so only the necessary condition can be checked: the trip's elapsed time is
  an upper bound on the working day.
- Fix: `meals: 2` on a trip of 11 hours or less is `TRAVEL_INPUT_INVALID`.
  Test `two_meal_allowances_need_a_trip_over_eleven_hours`.

## B (must-fix): meal allowance despite an employer-arranged meal

- Location: `trip.rs` `meal_allowance`, `MealAllowanceInput`.
- Claim: guidance 8.6 excludes the meal allowance when the employer arranged
  the meal (for example in a course package); the input could not say so.
- Evidence: local read; verifier confirmed no other route carries the fact
  (a trip without a per diem has no day answers).
- Fix: a third required answer `employer_arranged_meal`; `null` refuses with
  `TRAVEL_MEAL_ALLOWANCE_UNANSWERED`, `true` gives zero with the reason. Test
  `meal_allowance_is_zero_when_the_employer_arranged_a_meal`.

## C (must-fix): special-industry daily journeys earned a per diem

- Location: `trip.rs` (`compute`, the plan), `Situation`.
- Claim: decision 3 §, last paragraph: on an industry where the special place
  of work changes often, a daily journey from home with no regular workplace
  earns travel costs and a meal allowance only.
- Evidence: local read of the decision; verifier reproduced 54.00 approvable.
- Fix: situation `special_industry_daily_journey`, refused with
  `TRAVEL_SPECIAL_INDUSTRY` (computing it needs a product decision on its
  meal-allowance variant). Covered in
  `refusal_situations_the_sources_do_not_settle`.

## D (must-fix): an omitted regular-workplace distance passed the 5 km test

- Location: `mod.rs` `Distances::from_regular_workplace`.
- Claim: `#[serde(default)]` turned a missing answer into "no regular
  workplace", so the 5 km condition of decision 11 § passed silently.
- Fix: the field is required; explicit `null` still means "none". Test
  `an_omitted_regular_workplace_distance_is_an_error`.

## E (should-fix): per-trip `paid` checks let the claim exceed its maximum

- Location: `trip.rs` `payable`, `mod.rs` `compute_claim`.
- Claim: each trip's paid amount was compared with its own rounded maximum, so
  two trips at 5.51 against an exact 5.5055 each totalled 11.02 against a
  once-rounded claim maximum of 11.01.
- Fix: a claim-level check per category (refusal with `trip: null`). Test
  `paid_amounts_are_also_checked_against_the_claim_maximum`.

## F (should-fix): monthly vouchers did not add up to the claim

- Location: `mod.rs` `month_postings`.
- Claim: each month's voucher was rounded on its own, so a claim over two
  months could credit 2740 a cent more than the claim and its 311 line.
- Fix: each month books the rounded running total less what earlier months
  booked. Test `monthly_vouchers_add_up_to_the_claim`.

## G (should-fix): kilometres and per diem types reported for unpaid allowances

- Location: `trip.rs` `compute` (codes and km), `mod.rs`
  `incomes_register_lines`.
- Claim: with `paid` 0 on one trip of a multi-trip claim, its kilometres and
  per diem types still went into the 311 and 331 lines.
- Fix: a trip contributes kilometres only when its mileage is paid and types
  only for paid categories. Test
  `unpaid_allowances_leave_no_kilometres_or_types_in_the_report`.

## Dropped

None. The verifier's caveat on G (the Incomes Register rule is not in the
attached sources) does not change the fix: the report should not state
kilometres for money not paid either way.

## Coverage and residual risk

Read and tested: every refusal path, the travel-day, return-day, halving,
deduction, year-boundary, daylight-saving and rounding rules against
guidance examples 56–59, 61 and 63. Not reviewed by a model: the rate table's
figures (checked instead by a script that parsed both primary renderings of
each decision, Finlex PDF and vero.fi page, and compared them row by row,
plus spot-check tests), the user guide pages, and the CLI wiring beyond
`claim.rs`.
