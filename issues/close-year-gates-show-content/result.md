# Result: close-year gates show what they bind

## What changed

- New read command `aggountant close-year show --run-id <cy-…>` returns the Run
  view plus `open_gate`: `gate_id`, `gate_kind`, `snapshot_hash`, `kind`
  (`adjustments` | `tase_erittelyt` | `liitetiedot` | `statements`) and
  `content`, the exact value the gate's hash covers. For `ApproveAdjustments`
  it also lists the Run's D7 corrections with `sealed: true|false`.
- The content is built once and persisted in `close_year_state.json` before the
  gate opens (`depreciation` at start, `tase_erittelyt` and `liitetiedot` when
  their gates open; `statements` already was). The gate hash is computed from
  that value through one function, `GateContent::snapshot_hash`, which is also
  what `show` checks against.
- `show` refuses content that no longer hashes to the gate's `snapshot_hash`:
  `JOURNAL_CORRUPT` for persisted content (the state file was changed),
  `CLOSE_YEAR_BLOCKED` for a rebuilt one (a Run started before this change whose
  books changed after the gate opened). Approving `ApproveAdjustments`,
  `ApproveTaseErittelyt` or `ApproveLiitetiedot` runs the same check, so an
  approval is never given on content that cannot be shown. `ApproveStatements`
  keeps its stronger existing binding (persisted and fresh recompute).
- `seal-adjustments` seals the persisted (approved) depreciation lines instead
  of recomputing them; the archive writes the persisted (approved)
  tase-erittelyt and liitetiedot. Runs started before this change have none of
  the new state fields and fall back to the previous recomputation.
- The tase-erittelyt are built in one place (`build_tase_erittelyt`); the
  depreciation lines (`depreciation_lines`) and the statement provenance
  (`statement_provenance`) likewise, replacing four and three copies.
- User guide fi+en: the year-end step lists what each approval shows, the
  monthly-report workaround and the limitation bullet in `status.md` are gone,
  and the command list shows `close-year show` before each answer.

## Simplicity

Reuses the existing run state file and Preview-binding pattern of `statements`
for the other gates instead of adding a content store. Removes the duplicated
tase-erittelyt construction and the repeated `compute_depreciation` and
provenance code. Deliberately leaves out: showing answered gates' content,
attaching content to every command's output, and a separate legacy migration
(old state files just lack the optional fields).

## Decisions for later review

- **`close-year show` instead of content in every output.** A read command is
  composable, drives nothing (the canon; `resume` moves the run forward), and is
  needed anyway for a user returning in a later session. Attaching the content
  to `answer`/`resume` would duplicate it and make every output large
  (statements). Alternative: embed `open_gate` in all outputs.
- **Corrections listed on `ApproveAdjustments`, outside `content`.** They are
  not in the hash (sealed on their own calls), so they are a separate
  `corrections` field rather than part of what the hash covers. The user still
  sees all year-end entries at that gate.
- **Approval of TE/LT/ADJ refused when the shown content does not match the
  hash.** Previously these were informational bindings. Rejecting the gate
  stays possible, so a Run cannot wedge.
- **Seal the persisted depreciation lines.** Closes the gap where a rate-table
  change in a binary upgrade between start and approval would seal figures
  other than the approved ones. The workflow version string did not change, so
  in-flight Runs continue without `run migrate`.
- `close-year start` output is unchanged (still `depreciation` only); `show`
  is the full view, including accruals.
