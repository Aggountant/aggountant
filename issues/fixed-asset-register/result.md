# Result: fixed-asset-register

A sealed fiscal year now stores its fixed assets with their opening and closing
book values, and the next year's `close-year` opens each asset at the value the
sealed year closed it at. An opening book value is typed once, when an asset
enters, and a retyped one that disagrees with the sealed year is refused.

## Where the register lives, and why

In the fiscal store (`fiscal_year_asset_register`, migration v8), one row per
year, written in the transaction that seals the year.

- The register decides the next year's depreciation, so it is books, and the
  fiscal SQLite store is the single authority for the books.
- `archive/<year>/` was ruled out by the existing rule that the archive is a
  derived copy and is never read back. It now carries a copy of the register
  for the reader.
- Git masterdata is reference data that sealed vouchers resolve against and
  that existing homes take in only by addition; the register is state that
  changes every year.
- `knowledge company` is prose the agent judges, which is the stopgap this
  issue replaces.
- Writing it with the year seal leaves no window in which a year is sealed
  without its register. `fiscal_year_definition` is the precedent for a plain
  fiscal table that is not hash-chained.

No ADR was written: the choice follows from rules already in
`crates/ledger/AGENTS.md`, and the new rule is recorded there.

## What was built

- `crates/domain/src/closeyear.rs`: `AssetRegister` / `AssetYear` and
  `plan_asset_year`, the one rule for a year's assets: the previous register's
  assets at their closing value, less `removed_assets`, plus the input's.
  A carried asset given again must repeat the carried figures exactly;
  removing it and giving it again replaces them on purpose.
  `AssetRegister::check_continues` is the invariant the year seal enforces.
- `crates/ledger`: `seal_year` takes the register, refuses one that does not
  continue the previous year's, and stores it; `asset_register(year)` and
  `opening_asset_register(year)` read it.
- `crates/process/src/close_year.rs`: the start plans the register, the
  `ApproveAdjustments` gate shows and binds it, `SealYear` stores it. The input
  gains `removed_assets`. A Run started before this change has no register in
  its state and plans one at the seal, refused if that gives other
  depreciation than the Run approved.
- `close-year start` output gains `asset_register` and `opening_register_year`.

## How a home without a register starts one

A year with no stored register before it (the first close here, a year kept in
another system, or a year sealed by an older binary) takes every asset from
the input with its book value at the start of the year, exactly as before.
Sealing that year stores the register, and later closes carry from it. The
assets an opening voucher brought in are therefore typed once, in the first
close.

## Deliberately left out

- **Depreciating less than the maximum, and other plans.** The asset is
  removed for that year, its depreciation given as a correction, and it is
  given again the next year. A per-asset plan is a feature nobody has asked
  for yet.
- **Disposals.** The run does not book a sale or scrapping; the asset is named
  in `removed_assets` and the entry is the user's voucher.
- **Checking typed opening values against the opening balance.** The input
  names the expense and accumulated-depreciation accounts, not the asset
  account, so a sum check has nothing to compare with.
- **The register in the year's transition hash.** `voucher verify` does not
  cover the register row. Binding it changes the hash preimage of the
  year-lock event and needs its own decision.
- **A separate register command.** `close-year start --dry-run` with
  `{"year": N}` shows what the register carries into year N.

## Compatibility

Existing `close-year start` inputs are accepted unchanged in a home without a
register. In a home with one, an input that retypes a carried asset with other
figures is now refused; that refusal is the point of the issue.
