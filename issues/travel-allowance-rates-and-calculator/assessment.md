# Assessment of the travel-allowance-rates-and-calculator review

Assessed against the working tree in which the fixes were made; the slice
lands as one commit with them.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| A | Two meal allowances accepted on a short trip | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX — applied (necessary condition only) |
| B | Meal allowance despite an employer-arranged meal | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | HIGH | FIX — applied |
| C | Special-industry daily journeys earned a per diem | CONFIRMED | RARE | NEUTRAL | MINOR | HIGH | FIX — applied as a refusal |
| D | Omitted regular-workplace distance passed the 5 km test | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX — applied |
| E | Per-trip `paid` checks let the claim exceed its maximum by a cent | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX — applied |
| F | Monthly vouchers did not add up to the claim | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX — applied |
| G | Kilometres and types reported for unpaid allowances | CONFIRMED | RARE | NEUTRAL | NONE | MED | FIX — applied |

FIX: 7   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- A: only the trip-length bound can be enforced; whether the working day was
  over 11 hours remains the caller's answer, as the free meals are. A refusal
  would need a working-time input the issue does not ask for.
- B: the issue named two answers for decision 14 §; guidance 8.6 adds the
  employer-arranged meal. Added as a third required answer rather than folded
  into "usual eating place", so a missing answer refuses. An arranged meal
  with `meals: 2` gives zero (the conservative side) instead of a guess at one
  meal allowance.
- C: refused, not computed, because computing it means a meal-allowance-only
  variant whose details (3 §: "varsinainen työnantajan järjestämä
  työpaikkaruokailu") the issue does not settle; `crates/domain/AGENTS.md`
  requires a source or product decision for that.
- D, E, F, G: silent defaults or cent-level mismatches between the claim, its
  vouchers and its Incomes Register lines. Each is data integrity in a small
  way, so each is fixed and tested.

No confirmed finding remains open and no follow-up issue was filed.

## Simplicity

- Reused: `model::Instant` and `model::Date` for all input times and dates,
  `domain::bank_days::finnish_local_time` (the hand-written EU summer-time
  rule) for Finnish local time and years, the `include_str!` + `OnceLock`
  dated-table pattern of `payroll.rs`, the `DomainError` → `CliError` path, and
  the CLI's `parse_stdin_json`/`read_stdin_text`. The Incomes Register line
  shape follows the existing `TravelLine` (311 with kilometres, 331 with
  per diem types).
- Repetition: one place per rule. The year-boundary "lower amount" is one
  function (`lowest_over_years`) used by per diems, night and meal
  allowances; the halving threshold is one method on the per diem basis;
  every refusal goes through `Context::refuse`; rounding is one function used
  for totals, `paid` checks and vouchers.
- Left out on purpose: a country or plug-in abstraction (ADR-11 says not yet),
  storing anything (the draft slice owns storage), looking up the home's chart
  for posting accounts (the approval slice resolves accounts from the active
  chart; the preview shows the general chart's accounts), the
  regular-workplace register (a parallel slice; the trip carries the
  classification), the border-crossing allowance and other unsettled cases
  (refused), and local wall times without an offset (the caller gives RFC 3339
  with an offset; the output prints Finnish time back for checking).
