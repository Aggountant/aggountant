# Review: fixed-asset register and `close-year start --dry-run`

- **Finder:** claude-fable-5-2. **Verifier:** gpt-6-astra. **Model calls:** 2.
- **Rigor:** routine. **Risk class:** high (an irreversible year seal and the
  next year's depreciation), so the surviving candidates went to the verifier.
- **Scope:** the working-tree diff against `b043891`: `crates/domain/src/closeyear.rs`,
  `crates/ledger/src/lib.rs`, `crates/ledger/src/store/{mod,sqlite_fiscal}.rs`,
  `crates/process/src/close_year.rs`, `crates/cli/src/{main,input_help}.rs`,
  their tests, `skills/close-year/SKILL.md`, `crates/ledger/AGENTS.md`, the
  user guide pages. Single trusted user and one process assumed; concurrency
  and hostile input were out of scope.

## Finding 1 (should-fix, confirmed): a close could start, and seal depreciation, while the previous year was still open

- **Location:** `crates/process/src/close_year.rs`, `close_year_blockers`
  (start and dry run) and the `ApproveAdjustments` precheck.
- **Claim:** the rule that the previous fiscal year must be sealed lived only
  in `Ledger::fiscal_year_data`, which the Run first called when computing
  statements, after `ApproveAdjustments` had sealed the adjustment vouchers.
  With the previous year open, `opening_asset_register` returns `None`, so the
  start planned the assets from the input alone and the dry run reported
  `would_start: true` with nothing carried, indistinguishable from a home
  without a register.
- **Evidence (local):** no call to `fiscal_year_data` on the start path; a new
  test (`a_close_does_not_start_while_the_previous_year_is_open`) sets up a
  2025 with activity and closed quarters but no year seal and a closed 2026.
  **Evidence (verifier):** survives; traced the same path and confirmed the
  seal/attribution path does not require the previous year sealed.
- **Impact:** depreciation sealed for year N on a typed opening value; after
  year N-1 is sealed with its register the Run is refused at the last gate and
  a new Run computes another amount that the sealed voucher's natural key
  refuses. The way out restates the typed value, which is what the register
  exists to stop. The wedge after `ApproveAdjustments` existed before this
  change; the register made its consequence worse.
- **Fix (applied):** `close_year_blockers` turns the ledger's
  `PERIOD_NOT_SEALED` for the year's opening into the start blocker
  `previous-year-not-sealed:<year>`, which the dry run lists too. A first year
  and a boundary cut-over still start. Verifier: closes it for both entry
  points; does not protect a Run already started.

## Finding 2 (should-fix, confirmed reachable): the first close after a year sealed by an older binary carries nothing and says so only as `opening_register_year: null`

- **Location:** `crates/process/src/close_year.rs` `plan_assets`;
  `crates/ledger/src/lib.rs` `opening_asset_register`.
- **Claim:** a year sealed before migration v8 has no register row. If that
  year's close depreciated assets through the Run and the next close is
  started with no `assets`, no depreciation is planned and nothing refuses.
  The previous year's `closeyear:<prev>:dep:<asset_id>` vouchers show the
  assets existed, and nothing reads them.
- **Evidence (local):** `plan_asset_year` with `opening = None` plans from the
  input only. **Evidence (verifier):** survives; the unplanned-voucher check
  covers the closing year's keys only.
- **Impact:** a sealed year without depreciation, in a home that sealed a
  year with a binary older than this change and then omits its assets.
- **Fix (not applied):** when there is no opening register, list or refuse the
  previous year's depreciated asset ids missing from `assets`. A refusal needs
  a way to acknowledge a sold asset, which `removed_assets` cannot do without
  a register.

## Unresolved uncertainty

None.

## Dropped candidates (1)

- "The `crates/ledger/AGENTS.md` paragraph says a value is typed once, which
  does not hold for an asset removed for a year and given again." Not a code
  defect; the paragraph was reworded to say when a value is typed.

## Coverage

Read: the whole diff and the code it calls (`FiscalCalendar::previous`, the
year-seal transaction, `Money` equality and serialisation, the workflow
version pin, `fiscal_year_data`). The finder also checked that a Run started
by an older binary keeps its gate hash and seals, that `x - x` serialises as
`0.00`, and that the dry run creates no run directory, state or journal (it
opens the ledger like any read, which applies migration v8). Tested: the
workspace test suite and `scripts/ci-local.sh` (both passed before the
Finding 1 fix; the close-year tests were re-run after it). Not covered: the
register is outside the year's transition hash, so `voucher verify` does not
detect a changed register row (recorded in `crates/ledger/AGENTS.md` as
needing a decision).
