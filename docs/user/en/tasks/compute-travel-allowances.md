# Compute the tax-free allowances of a trip

After a business trip, aggountant computes the maximum tax-free allowances for
it: mileage allowance (kilometrikorvaus), per diems (päiväraha), meal
allowance (ateriakorvaus) and night travel allowance (yömatkaraha). The
computation is a preview. It records nothing, stores no trip and reports
nothing to Tulorekisteri.

## Before you start

- You know when the trip started and ended, where it went, and how far the
  place of work was from home and from the regular workplace.
- For each travel day you know how many free meals you had and whether the
  company paid for one of your meals.
- If you drove your own car, you know the dates and kilometres of the drives.
- The traveller's regular workplace and relationship to the company are
  recorded: [Record your regular workplace](record-regular-workplace.md). The
  computation classifies the destination from the register, not from what the
  agent says.

## How to do it

1. **Tell the agent about the trip.**

   > Compute the allowances for Maija Meikäläinen's trip to Tampere: left home
   > on 10 March at 7:30, back at 19:15, drove her own car 344.6 km to the
   > customer in Tampere, 172 km from home. The customer offered lunch.

2. **The agent** gathers the trip's details and computes them. The
   destination is classified from the register for the trip's start date: a
   regular workplace earns no tax-free allowance, a secondary workplace travel
   costs only, and other places are special work sites. When the
   computation is missing an answer, the agent asks for it: the first run
   shows the travel days, and every per diem day needs the number of free
   meals. An empty answer does not mean there were none, so the agent asks
   even when the answer is zero. For a trip abroad the agent also asks in
   which country each travel day ended.

3. **Check the result.** Look at the travel days and their times, the basis
   of each per diem (full or partial domestic, foreign) and any halving for
   free meals, the kilometres and the amounts. The example trip earns one
   full per diem of €54.00 (one free meal does not halve a full per diem) and
   a mileage allowance of €189.53, reported to Tulorekisteri as 345 km. The
   result also names the year of the Tax Administration decision the amounts
   come from.

The result also shows the Tulorekisteri lines (mileage allowance 311 and per
diem 331) and proposed postings (7300 Matkakulut, 7350 Päivärahat, against
2740 Muut lyhytaikaiset velat) for the month in which the trip ended. They are
proposals. For now, record the allowances as a manual entry, as in
[settling a travel advance](record-travel-advance.md), and report them
[to Tulorekisteri](report-travel-allowances.md).

### When the computation refuses

aggountant does not guess. When the trip is missing a fact, or the situation
is one that the Tax Administration's decision and guidance do not settle, the
result holds a refusal and its reason. A refused computation cannot be
approved, but the other amounts are still shown. The most common:

| Code | What it means | What you do |
|---|---|---|
| `TRAVEL_FREE_MEALS_UNANSWERED` | a day has no number of free meals | give the number, zero included |
| `ALLOWANCE_ABOVE_MAXIMUM` | the allowance to be paid is above the tax-free maximum | pay at most the maximum; the excess would be pay, which aggountant does not handle yet |
| `TRAVEL_RATES_MISSING` | this version has no Tax Administration decision for the trip's year | update aggountant |
| `TRAVEL_COUNTRY_INVALID` | the country code is not an ISO 3166 code, or the city is not in the decision | correct the code; an unknown code never gets the "other countries" per diem |
| `TRAVEL_MEAL_ALLOWANCE_WITH_PER_DIEM` | a meal allowance was asked for a trip that earns a per diem | choose one; a meal allowance belongs only to trips without a per diem |
| `TRAVEL_NIGHT_ALLOWANCE_WITH_LODGING` | a night travel allowance was asked for a night whose lodging the company paid | leave the night travel allowance out for that night |

The other refusals concern situations the sources do not settle: a day with
both a free meal and a company-paid meal, a foreign trip that passes through
Finland, a night travel allowance on a foreign trip, a trip from a holiday home, a workplace that changes during the
trip, the border-crossing allowance, a posting (komennus), working abroad and
daily journeys to a changing site on a special industry (erityisala).
Ask your bookkeeper about them.

### Rules the sources do not give

In these situations aggountant follows the product owner's decision, and the
result names the rule when an amount depends on it:

- A travel day across the year boundary gets the lower of the two years'
  amounts.
- A travel day is 24 hours of elapsed time, so the start or end of summer
  time counts as it happened.
- Amounts are rounded to cents once for the whole computation, halves away
  from zero.
- A free meal is one meal whatever it cost. A meal of your own that the
  company paid is deducted from the per diem, but the per diem never falls
  below half.
- When leisure is combined with a trip, the per diems are computed from the
  working time and travel time you state.

<details>
<summary>The command the agent runs</summary>

```sh
aggountant --json claim compute < trip.json
```

`claim compute` reads the traveller and the trips as JSON, classifies the
trips from the company home's register and prints the computation. It writes
nothing. The input fields are shown by
`aggountant claim compute --help`.
</details>

## Done when

- The computation has no refusals (`approvable` is `true`).
- You know the trip's per diems, mileage allowance and the kilometres to
  report to Tulorekisteri.

## Limitations

- The computation does not store the trip. Trips are stored in an expense
  claim draft ([Collect an expense claim](collect-expense-claim.md)), but a
  claim cannot be approved in aggountant yet: the allowances are recorded and
  reported by hand.
- If no relationship to the company or regular workplace is recorded for the
  traveller on the trip date, the computation refuses
  (`CLAIMANT_RELATIONSHIP_UNKNOWN`, `REGULAR_WORKPLACE_UNKNOWN`) and computes
  nothing for the trip.
- [What is ready and what is not](../reference/status.md#receipts-and-entries)
  describes the situation.

## See also

- [Report travel allowances to Tulorekisteri](report-travel-allowances.md)
- [Record a travel advance and its settlement](record-travel-advance.md)
- [Glossary](../reference/glossary.md)
