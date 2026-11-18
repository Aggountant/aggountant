# Review: close-year-gates-show-content

- Finder: gpt-6-astra (`consult-llm --task review`, diff `main..HEAD` plus
  `close_year.rs`, its tests and the issue). Verifier: claude-fable-5-2, one
  focused false-negative question on the seal/archive boundary. Model calls: 2
  (plus one connectivity probe after the first finder call exited 1 with no
  output).
- Rigor: critical. The change decides what `SealYear` seals and archives,
  which cannot be undone.
- Scope: `crates/process/src/close_year.rs`, `crates/process/tests/close_year.rs`,
  `crates/cli/src/main.rs` (close-year show), user guide fi+en.

## R1 (must-fix): resuming a superseded run overwrites the sealing run's archive

- Location: `crates/process/src/close_year.rs`, `resume_close_year`, reconcile
  branch (`own_hash == fy.statements_hash`), then `write_archive_stub`.
- Claim: resume adopted a sealed year when the statements hash matched. Now
  that each run persists its own approved tase-erittelyt, an abandoned run
  whose statements equal the sealing run's but whose breakdowns differ (a
  transfer between two accounts on one balance-sheet row) rewrote
  `archive/<year>/tilinpaatos.json` with its stale breakdowns and its own run id.
- Evidence: regression test
  `resuming_a_superseded_run_leaves_the_sealing_runs_archive_alone` fails with
  the old condition (archive bytes change) and passes with the fix.
- Fix: also require `fiscal_year_view.sealed_by_run_id == run_id`. Applied.

## R2 (should-fix): a legacy run resumed after approval seals recomputed depreciation unchecked

- Location: `run_seal_adjustments` → `depreciation_lines` fallback when
  `state.depreciation` is absent; the hash check ran only on the answer path.
- Claim: a run whose state predates the persisted depreciation, resumed after
  ApproveAdjustments was answered, would seal a recomputation not compared with
  the gate hash. Not triggerable today (the computation is unchanged), but the
  invariant "the sealed depreciation is the approved one" should hold.
- Fix: `run_seal_adjustments` calls `gate_view(ApproveAdjustments)` (the hash
  check) before anything is sealed. Applied; covered by the existing
  hash-mismatch tests of `gate_view`, no dedicated legacy-resume test.

## Pre-existing paths found by the verifier (outside this change)

Each was confirmed by the verifier with a probe test, and none is in code this
branch changes. They were filed as unlaned intake items:

- V1 `close-year-seal-after-year-sealed`: approving or sealing a correction in
  a run after another run sealed the year books the voucher into the next year.
- V2 `close-year-correction-on-rejected-run`: `seal-correction` works on a
  rejected run.
- V3 `close-year-liitetiedot-miss-booked-depreciation`: liitetiedot come from
  the run's asset list, not the books.

## Verifier: no path on the changed chain

On persisted gate content → seal/archive, the verifier found no path that
seals or archives unapproved content: approval checks persisted content against
the gate hash; at ApproveStatements and again before SealYear the statements,
tase-erittelyt and liitetiedot must agree as persisted, as rebuilt and as
bound; the archive is written only after this run's own seal or an owned
reconcile; crash points between state write and GateOpened rebuild on resume.

## Dropped

- A crash between seal-year `EffectSucceeded` and `write_archive_stub` leaves
  a sealed year without an archive (the write error is ignored). Missing
  content, not wrong content; the window is milliseconds and predates this
  change. Not filed.
