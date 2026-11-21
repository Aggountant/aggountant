# Assessment: fixed-asset register and `close-year start --dry-run`

Source review: [review.md](review.md) (finder claude-fable-5-2, verifier gpt-6-astra). Assessed at `b043891` plus the working tree.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | A close could start, and seal depreciation, while the previous year was still open[^1] | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX (Rule 5; applied) |
| F2 | First close after a year sealed by an older binary carries no assets and gives no warning[^2] | CONFIRMED | RARE | NEUTRAL | MINOR | MED | DROP (Rule 1b: RARE, no readability gain) |

[^1]: review line 13
[^2]: review line 41

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 1

## Evidence

**F1.** No fiscal_year_data call on the start path at b043891. Test a_close_does_not_start_while_the_previous_year_is_open (2025 with activity and closed quarters, unsealed; 2026 closed) now gets the blocker from both start and dry run. Verifier traced the same path. Fix already applied in the working tree; 40 close-year process tests pass.

**F2.** Reachable by reading plan_asset_year(opening=None) and confirmed by the verifier. It needs a home that sealed a fiscal year through close-year with a binary older than this change. The brief for this work states the first real year-end close is in January 2027, after this change, so no home the project knows of is or will be in that state; before this change the same omission (leaving an asset out of the input) was equally silent every year. The fix adds a second, voucher-key-derived notion of 'assets the home has' beside the register, plus an acknowledgement path, for a one-time transition nobody goes through. The close-year skill tells the agent that a home's first close gives every asset.

No issue was filed for F2: it has no observed occurrence, and its trigger is a
home state that the project's known use does not produce. If a home that
sealed a year with an older binary appears, the finding is described in full
above and in the review.

## Simplicity

- **Reused:** the dry run and the start share `close_year_blockers`,
  `plan_assets`, `adjustment_period` and `build_state`, extracted from the old
  start body; nothing is computed twice. `plan_asset_year` wraps the existing
  `compute_depreciation`. The register is one fiscal table written through the
  existing year-seal transaction and request.
- **One rule, one place:** continuity of the register is
  `AssetRegister::check_continues`, called by the plan (domain), the year
  seal (ledger) and the Run's pre-approval check.
- **Remaining repetition:** `plan_asset_year` refuses a carried asset given
  with other figures with a message naming the carried values, and
  `check_continues` would refuse the same register more tersely. The first is
  what the user reads; the second is the write path's guard.
- **Left out on purpose:** per-asset depreciation plans, disposal entries, a
  register command, checking typed opening values against the opening
  balance, binding the register into the year's transition hash, and
  backfilling a register from an older binary's depreciation vouchers (F2).
