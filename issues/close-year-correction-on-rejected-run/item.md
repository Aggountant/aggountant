---
created: 2026-11-18
updated: 2026-11-18
type: bug
status: untriaged
priority: normal
provenance: ai-review
source_ref: taskfleet:01m7q4nbceha52vkzwr4ahjzej/review-finding:V2
review_source: ai-review
originating_run: 01m7q4nbceha52vkzwr4ahjzej
originating_run_kind: spinoff
labels:
- ai-review-model:claude-fable-5-2
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_confidence: high
review_severity: must-fix
review_target: main..HEAD
---

# close-year seal-correction seals a correction for a rejected run

## Description

`close-year seal-correction` seals and attributes a D7 correction for a Run
that has already been rejected, so an entry from a declined Run reaches the
closing year's books.

Where: `crates/process/src/close_year.rs`, `seal_close_year_correction`, only
checks that the correction effect exists and has not succeeded; it does not
look at the Run's status or whether ApproveAdjustments is still open.

Sequence (Esimerkki Oy, made-up figures): Run A starts with a 100.00
correction; its ApproveAdjustments is rejected and the Run is Rejected.
`seal-correction` on A succeeds and the voucher is attributed to 2026. Run B
with no corrections then completes; its ApproveAdjustments view lists no
corrections, yet the result it seals includes A's 100.00. The amount shows in
B's statements, but no gate names the entry.

Fix sketch: refuse unless the ApproveAdjustments gate is open and unanswered
(GATE_NOT_OPEN).

Found by the independent verifier (claude-fable-5-2) during the review of
close-year-gates-show-content, confirmed by a probe test there. Predates that
change.
