---
created: 2026-11-13
updated: 2026-11-16
type: improvement
status: untriaged
priority: normal
provenance: other
provenance_detail: found during spinoff run
source_ref: taskfleet:01m7a1259e86vgr944j9z8tfnh/close-year-blind-approval
originating_run: 01m7a1259e86vgr944j9z8tfnh
originating_run_kind: spinoff
---

# close-year approval steps do not show what the user approves

## What goes wrong for the user

The year-end close (`close-year`) asks the user to approve four things in turn:
the adjusting entries, the balance sheet breakdowns (tase-erittelyt), the notes
(liitetiedot) and the financial statements. The last approval seals the fiscal
year, which cannot be undone. Yet only the first step shows anything to look
at: `close-year start` lists the calculated depreciation. After that every
command answers with the run's step states and opaque hashes. The user approves
breakdowns, notes and statements they have not been shown.

Today they can partly work around it. Once the adjusting entries are approved,
the monthly report for the fiscal year's last month
(`report statements --period <YYYY-12>`) includes them, and the user guide tells
the user to look at it first. That report is a different projection from the
one the gate binds, and the breakdowns and notes are not visible anywhere
until the year is sealed (they are then written to
`<home>/archive/<year>/tilinpaatos.json`).

The fix: whenever a close-year gate is open, the command's output shows the
content that gate's hash covers, so the approval is given on what was seen.

## How the gates bind content now

All in `crates/process/src/close_year.rs`:

- **`ApproveAdjustments`** (opened in `start_close_year`): `adjustments_hash`
  is sha256 over the JSON of the depreciation lines followed by the JSON of the
  accrual inputs. `CloseYearStart` returns the depreciation lines but not the
  accruals. D7 corrections are not in this hash; each is sealed by its own
  `close-year seal-correction` call, all of them before this gate can be
  approved (`assert_corrections_sealed`), so by the time the user approves the
  adjustments the corrections are already in the books.
- **`ApproveTaseErittelyt`** and **`ApproveLiitetiedot`** (opened in
  `open_gate`): `tase_erittelyt_hash` and `liitetiedot_hash` build the content
  from the ledger at gate-open time and hash it with the type's
  `statements_hash()` (sha256 of its serde JSON, `crates/domain/src/statements.rs`).
  The content is then thrown away. `write_archive_stub` builds the same
  tase-erittelyt again with identical code, and `build_liitetiedot` serves both.
- **`ApproveStatements`** (a Preview gate): the hash of `state.statements`,
  computed once right after `ApproveAdjustments` and persisted in the run's
  `close_year_state.json`. Approval re-checks that a fresh recompute still
  matches (`assert_statements_binding`), and `run_seal_year` checks again.

The CLI (`CloseYearCmd` and its dispatch in `crates/cli/src/main.rs`) serialises
`CloseYearStart` for `start` and the generic `RunView` for `answer`,
`seal-correction` and `resume`. `RunView.gates[].snapshot_hash` is the only
trace of the content.

## What to build

The content shown for a gate has to come from the same value whose hash the
gate recorded. The right way is to build it once and derive both the hash and
the output from it. Recomputing it for display and hoping it matches is the
wrong way. The Preview binding on `ApproveStatements` exists because the
statements must be the exact bytes approved. Showing a recomputation would
reopen that gap for the user even if the hash is fine.

The user may come back to an open gate in a later session, so the content of
the gate that is currently open needs to be readable at any time, not only in
the output of the call that opened it. Either attach it to every close-year
command's output or add a read-only `close-year show --run-id`. A read command
fits the CLI canon better than relying on `resume`, which drives the run
forward. Pick one and say why in the result.

The breakdowns and notes are rebuilt from the ledger. If they are shown
from a fresh build rather than from what was hashed, the shown content can
differ from the bound hash after the ledger changes. Either persist the
built content with the run state, as is done for `statements`, or recompute
and refuse to show content whose hash no longer matches the gate.

Removing the duplicated tase-erittelyt construction (`tase_erittelyt_hash`
and `write_archive_stub`) belongs to this change, because the display needs a
third copy otherwise.

Whether the `ApproveAdjustments` view also lists the already-sealed
corrections is the implementer's call. They are not in the hash, but the user
approving year-end entries would expect to see all of them.

The user guide changes in the same commit, fi and en: the limitation bullet
in `docs/user/*/reference/status.md` (year-end section), the step and the
"Limitations" paragraph in `docs/user/*/tasks/year-end.md`, and
`docs/user/*/reference/commands.md` if the command surface changes.

## Acceptance

- For each of the four gates, while it is open, a close-year command shows its
  content: depreciation lines and accruals; tase-erittelyt; liitetiedot;
  statements.
- A test in `crates/process/tests/close_year.rs` shows, for every gate, that
  hashing the shown content the way the gate does gives exactly the gate's
  `snapshot_hash`.
- The open gate's content can still be read after the process that opened it
  has exited, including after an interrupted run is resumed.
- The tase-erittelyt are built in one place.
- The user guide (fi and en) no longer tells the user to check the monthly
  report instead of the gates. It says what each approval step shows.
