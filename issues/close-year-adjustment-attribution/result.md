# Result: close-year attributes its adjustments to the closing year

## What changed

- `crates/process/src/close_year.rs`: every voucher the Run seals (depreciation,
  accruals, D7 corrections) goes through `seal_attributed`, which seals it into
  the next open period and immediately calls `Ledger::attribute_adjustment` for
  the closing year. Both steps replay on a repeat (the seal by natural key, the
  attribution by voucher and year), so a Run interrupted between them completes
  on resume, on a repeated `seal-correction`, or when a new Run replays the
  seal.
- All D7 corrections must be sealed before `ApproveAdjustments` is approved
  (before: before `ApproveLiitetiedot`). The statements, tase-erittelyt and
  liitetiedot are computed right after that approval, so every gate sees the
  final figures and the statements are computed once.
- `assert_run_vouchers_attributed` refuses (`CLOSE_YEAR_BLOCKED`) to compute
  the statements, to accept `ApproveStatements` and to run `SealYear` while any
  of the Run's natural keys is not attributed to the closing year.
- `run_seal_year` re-checks the full statements binding (fresh recompute equals
  the bound hash), not only the answer path. Found by the review (F1).
- Workflow definition `close-year/v2` → `close-year/v3`, so a Run started
  before this change needs `run migrate`; the guards above keep a migrated Run
  from binding statements computed without its vouchers.
- The domain's short-year depreciation refusal now points to giving the
  depreciation to close-year as a correction (an "ordinary voucher" would land
  in the next year's result, and the closing year's months are sealed).
- User guide, fi and en: the warning against using close-year for a real close
  is removed (status, year-end task, for-bookkeepers, periods). The year-end
  task now records the year-end entries with the run. The remaining limitation
  (the gates do not show what they approve) points to the last month's report,
  which shows the attributed adjustments once `ApproveAdjustments` is approved.

## Tests

`crates/process/tests/close_year.rs`, 19 tests, among them:

- `the_sealed_result_includes_depreciation_and_the_next_year_does_not_count_it_again`:
  the issue's reproduction (98.80 expense + 25.20 input VAT, twelve sealed
  months, one machine of 2 000.00). The sealed result is -598.80 (was -98.80).
- `happy_path_seals_the_year_and_verifies`: depreciation and accrual both
  attributed; sealed result 5 700.00 (was 8 000.00, without both).
- `a_d7_correction_must_be_sealed_before_approve_adjustments`: the correction is
  attributed and visible in the sealed income rows.
- `an_adjustment_sealed_but_not_attributed_is_attributed_when_the_run_continues`:
  crash between seal and attribution.
- `statements_are_not_bound_while_a_run_voucher_is_unattributed` and
  `a_resumed_run_does_not_seal_statements_another_run_has_made_stale`: the
  binding guards.
- `depreciation_in_a_short_fiscal_year_is_refused`: now also closes the short
  year with the depreciation given as a correction.

Gates: `scripts/ci-local.sh` green, `bash scripts/check-user-guide.sh` green.

## Side effects checked

- **Next year's monthly report and year-to-date.** Not counted twice. The
  attributed voucher's statement year is the closing year, so
  `fiscal_year_data(next)` treats it as prior activity. It is left out of the
  year's vouchers, and the monthly report builds both month and year-to-date
  from those. Tested: the 2027-01 report has month and year-to-date result
  0.00 and no sources.
- **Next year's opening balance.** Built from the closing year including the
  adjustments: the 500.00 accumulated depreciation is a credit on 1160 in
  `fiscal_year_data(2027).opening_balance`, and the expense goes to retained
  earnings through the same fold. The 2027-01 balance sheet totals -598.80
  and balances.
- **VAT period of a correction sealed into the next year's first period.**
  A close-year correction cannot carry a VAT code: `build_correction_draft`
  sets `vat_code: None` on both legs, and `CorrectionInput` has no VAT field.
  The same goes for depreciation and accrual vouchers (`crates/domain/src/closeyear.rs`).
  A voucher with VAT postings that is attributed would still land in the VAT period of its
  **posting date** (here 2027-01 or 2027-Q1): `file-vat` reads
  `Ledger::period_vouchers`, which filters by posting date and ignores
  attributions. For close-year that is moot (no VAT legs). For VAT itself the
  posting-date period is the right default: the closing year's VAT periods
  are already filed, and correcting VAT is a replacement return for the
  original period, not something an attribution should change silently. No
  VAT behaviour was changed and nothing was filed for it.

## Simplicity

Reused the ledger's attribution primitive and its projections; no ledger
change. The three natural-key formats now live in one function each, and
`seal_attributed` replaces two inline seal closures. Deliberately left out: a
separate `attribute-adjustments` effect in the journal (the ledger
attribution log is the record, and the guard reads it), and a ledger read
by natural key (the guard maps attributions to keys through
`Ledger::voucher`, a handful of reads per close).

## Decisions for later review

- **Corrections are sealed before `ApproveAdjustments`, not before
  `ApproveLiitetiedot`.** Corrections are fixed in the Run's input at start, so
  nothing is discovered between gates that would need a later seal. Requiring
  them earlier means the tase-erittelyt and liitetiedot gates review final
  figures. The alternative was to recompute the statements when
  `ApproveStatements` opens. That would bind the right statements, but the
  tase-erittelyt approval could then have been given on figures without the
  corrections.
- **The guard compares natural keys, not a count.** It maps each attribution
  of the year to its voucher's natural key. Attributions recorded by another
  Run for the same year (same keys replay to the same voucher) satisfy it,
  and an extra voucher attributed by another Run is caught by the binding
  check instead.
- **Vouchers left by an abandoned Run with different input stay unattributed.**
  If an earlier Run sealed an adjustment the new Run's input no longer
  contains, that voucher stays in the next year's result. It was not
  approved in the Run that seals the year. Alternative: attribute every
  `closeyear:<year>:*` voucher. Rejected because it would put unapproved
  figures into the sealed statements.
- **The user guide lists close-year as ready for the year-end entries and
  sealing.** The issue's acceptance criterion removes the warning. The
  remaining gap (`close-year-gates-show-content`, filed through intake,
  untriaged) has a workaround that the guide states: the last month's report
  shows the attributed adjustments. The official financial statements
  document, the written notes and the filings are still described as
  outside aggountant.
- **Short-year depreciation is given as a correction.** The previous advice
  ("record it as an ordinary voucher") had no correct path: the closing
  year's months are sealed and the next year would carry the expense. Both
  the guide and the domain refusal message now say to give it to the run as
  a correction. A test covers this.

## Filed through intake

- `close-year-gates-show-content` (improvement, untriaged): the gates bind
  hashes of content no command shows before approval.
