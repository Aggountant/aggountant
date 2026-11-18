---
created: 2026-11-18
updated: 2026-11-18
type: bug
status: open
priority: high
provenance: ai-review
source_ref: taskfleet:01m7q4nbceha52vkzwr4ahjzej/review-finding:V3
review_source: ai-review
originating_run: 01m7q4nbceha52vkzwr4ahjzej
originating_run_kind: spinoff
labels:
- ai-review-model:claude-fable-5-2
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_confidence: high
review_severity: should-fix
review_target: main..HEAD
lane: close-year
lane_seq: 6
collision: [crates/process/src/close_year.rs]
preparation: ready
---

# close-year liitetiedot can omit depreciation an earlier run already booked

## Description

The liitetiedot a close-year Run archives describe the depreciation in that
Run's asset list, not the depreciation in the books, so the sealed notes can
disagree with the sealed income statement.

Where: `crates/process/src/close_year.rs`, `build_liitetiedot` derives the
depreciation notes from the Run's own depreciation lines.
`assert_run_vouchers_attributed` checks that the Run's planned vouchers are
attributed, but not the converse: close-year vouchers already attributed to the
year by an earlier Run that are not in this Run's plan.

Sequence (Esimerkki Oy, made-up figures): Run A (asset kone-1) approves
ApproveAdjustments, so its 2,000.00 depreciation is sealed and attributed, and
is then abandoned. Run B starts with no assets because the depreciation is
already booked. B completes; the archive has 2,000.00 depreciation in the
income statement and empty depreciation notes. The user approved those empty
notes, but they do not match the books.

Fix sketch: at start and in the statements binding, refuse when a
`closeyear:<year>:` voucher attributed to the year is not in this Run's plan;
the user re-supplies the asset or correction, the seal replays by natural key,
and the gates and notes cover it.

Found by the independent verifier (claude-fable-5-2) during the review of
close-year-gates-show-content, confirmed by a probe test there. Predates that
change.
