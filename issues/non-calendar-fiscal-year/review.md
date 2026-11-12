# Review: non-calendar fiscal years (final diff)

- Skill: `/llm-review`, rigor **critical** (ledger core: year attribution of
  sealed vouchers, a fiscal-store migration, sealed-year integrity).
- Finder: `gpt-6-astra` (the other provider from the implementing agent).
  Verifier: `claude-fable-5-1`, one focused false-negative question on the
  define/attribution/period-transition boundary. Two model calls.
- Scope: `git diff 505aa12` over `crates/` and `skills/` (model fiscal
  calendar, domain statements and depreciation, ledger store and projections,
  process close-year, CLI), with the plan and `crates/ledger/AGENTS.md`.
- Date: 2026-11-12.

Every confirmed finding below was reproduced by a failing test before it was
fixed. Those tests are in `crates/ledger/tests/fiscal_year_definition.rs`.

## F1 (must-fix, data integrity): a definition could remove the year an attribution names

**Location:** `crates/ledger/src/store/sqlite_fiscal.rs`, `define_fiscal_year`.

**Claim:** the definition checks compared sealed vouchers' labels and the dates
of years with a `fiscal_year` row, but not the targets of adjustment
attributions. In a calendar home with sealed 2026 and a 2028-01-01 voucher
attributed to 2027, defining `2027-01-01..2028-06-30` was accepted. Fiscal year
2027 then no longer existed, and `fiscal_year_data(2028)` folded the attributed
expense into the 2028 opening balance as prior-year activity instead of the
year's result: silently wrong statements.

**Evidence:** test `a_definition_may_not_remove_or_redate_an_attributed_year`
failed before the fix.

**Fix:** the "year keeps its exact dates" check now covers every distinct
attribution target as well as year rows.

## F2 (must-fix, data integrity): an attribution could name a fiscal year that does not exist

**Location:** `crates/ledger/src/lib.rs`, `attribute_adjustment`.

**Claim:** only `effective_year <= posting_year` was checked. In a July company
whose first year is 2027, attributing a voucher to 2026 was accepted and
immutable; the voucher left its own year's statements without entering any.

**Evidence:** test `an_attribution_must_name_an_existing_fiscal_year` failed
before the fix.

**Fix:** the target label is resolved through the calendar first; an undefined
target is refused with an audited `FISCAL_YEAR_UNDEFINED` rejection.

## F3 (must-fix, year becomes unclosable): a definition could strand active quarter coverage

**Location:** `crates/ledger/src/store/sqlite_fiscal.rs`, `define_fiscal_year`.

**Claim:** with `2027-Q1` closed (and possibly VAT-filed) in a calendar home,
defining the change year `2027-01-01..2027-06-30` was accepted. A non-calendar
year closes by month, a calendar year cannot mix active months and quarters,
and a filed quarter cannot reopen, so fiscal year 2027 could never be sealed.
The same held for a mid-year first definition over a closed quarter.

**Evidence:** tests `a_definition_may_not_strand_active_quarter_coverage` and
`a_definition_starting_mid_year_may_not_orphan_an_active_quarter` failed before
the fix.

**Fix:** a definition is refused when an active quarter's calendar year would
not be wholly one calendar fiscal year (`quarters_allowed_in`).

## F4 (must-fix, year becomes unclosable; verifier): a quarter could be closed inside a non-calendar year

**Location:** `crates/ledger/src/store/sqlite_fiscal.rs`, `transition_period`.

**Claim:** F3's guard only covered the order "quarter first, definition
second". After defining 1.7.2026–30.6.2027, `begin_close(2026-Q3)`,
`seal_period` and `file_vat` were all accepted, and fiscal year 2027 could then
never be sealed.

**Evidence:** reproduced by `quarters_cannot_be_closed_inside_a_non_calendar_year`
(failed before the fix).

**Fix:** a period transition to `Closing`/`Sealed` of a quarter is refused
(audited) unless its calendar year is wholly one calendar fiscal year. The same
helper serves both paths. `year_close_periods` now checks all three months of a
quarter rather than its first and last months (a one-month fiscal year inside a
quarter's middle month was missed); with both guards it is a defensive
duplicate.

## Unresolved uncertainty

None.

## Dropped candidates

None. The finder raised three candidates; all three survived the local check and
were reproduced. The verifier raised one new sequence (F4), also reproduced, and
listed the paths it traced and found closed: label moves of sealed vouchers,
sealed-year range and predecessor, attributions to removed labels, the prior-year
gate across skipped labels, period transitions under a sealed non-calendar year,
statement filtering and re-dating, and close-year target periods.

## Coverage and residual risk

Read and exercised: the full diff, the existing ledger, process and CLI tests
(all green), the new model, ledger, process and CLI tests, and a home written by
the `main` binary (fixture), which opens, migrates to schema v7, verifies, seals,
closes and reports with unchanged calendar-year figures.

Residual: a calendar year's statement bytes are unchanged by construction (the
new `period` field is omitted for calendar years), shown by a domain test that
checks the serialized bytes. No golden statement hash from before the change was
compared, because none exists in the repository.
