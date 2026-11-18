# Assessment: close-year-gates-show-content review

Source review: `review.md` beside this issue.

| id | title | confirmation | likelihood | recommendation | applied |
|---|---|---|---|---|---|
| R1 | Resuming a superseded run overwrites the sealing run's archive | CONFIRMED | RARE | FIX | yes |
| R2 | Legacy run resumed after approval seals recomputed depreciation unchecked | CONFIRMED | RARE | FIX | yes |
| V1 | Approval or seal-correction after another run sealed the year books into the next year | CONFIRMED (verifier probe) | RARE | SPIN_OFF | filed `close-year-seal-after-year-sealed` |
| V2 | seal-correction works on a rejected run | CONFIRMED (verifier probe) | RARE | SPIN_OFF | filed `close-year-correction-on-rejected-run` |
| V3 | liitetiedot can omit depreciation an earlier run booked | CONFIRMED (verifier probe) | RARE | SPIN_OFF | filed `close-year-liitetiedot-miss-booked-depreciation` |
| D1 | Crash before archive write leaves a sealed year without an archive | UNABLE_TO_VERIFY | RARE | DROP | no |

R1 is data integrity of the sealed archive, introduced by persisting the
breakdowns per run, so it is fixed here despite RARE likelihood. R2 makes the
issue's invariant hold for legacy runs at the cost of one call. V1–V3 are real
data-integrity paths in close-year code this change does not touch; fixing them
is a separate change with its own tests, so they are spun off unlaned. D1 is
missing rather than wrong content in a millisecond window.

## Simplicity

The change reuses the run state file and the existing Preview-binding pattern
for all four gates, and puts each gate's hash in one function
(`GateContent::snapshot_hash`) used both when the gate opens and when it is
shown. It removes the duplicated tase-erittelyt construction, three copies of
the depreciation computation and three of the statement provenance. The R2 fix
reuses `gate_view` rather than adding another hash check. It leaves out
showing content of already-answered gates and a migration for old state files.
