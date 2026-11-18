---
created: 2026-11-18
updated: 2026-11-18
type: bug
status: open
priority: high
provenance: ai-review
source_ref: taskfleet:01m7q4nbceha52vkzwr4ahjzej/review-finding:V1
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
lane: close-year
lane_seq: 4
collision: [crates/process/src/close_year.rs]
preparation: ready
---

# close-year approval or seal-correction after another run sealed the year books a voucher into the next year

## Description

A close-year Run that is answered or given a correction after another Run has
already sealed the same fiscal year seals a voucher into the next year and then
fails to attribute it, so a closing-year entry ends up in the next year's result.

Where: `crates/process/src/close_year.rs`, `seal_attributed` (seal, then
attribute) reached from `run_seal_adjustments` via `answer_close_year`, and
from `seal_close_year_correction`. Neither caller checks that the fiscal year
is still Open; `resume_close_year` does. The ledger accepts a posting in the
next year's open period after the year is Sealed, then refuses the attribution
to the sealed year.

Sequence (Esimerkki Oy, made-up figures): Run A starts with assets kone-1 and
kone-2 and stops at ApproveAdjustments. Run B (only kone-1) completes and seals
2026. `close-year answer` on A, ApproveAdjustments approve, returns
PERIOD_TRANSITION_CONFLICT, but the depreciation voucher for kone-2 is already
sealed in 2027 with no attribution and counts in 2027's statements. The same
happens with `close-year seal-correction` on an abandoned Run that has an
unsealed correction. A sealed voucher cannot be edited, so the fix needs a
correcting voucher.

Fix sketch: one check that the year is Open, called before anything is
journaled or sealed in `answer_close_year` (approve), in
`seal_close_year_correction` and first in `seal_attributed`; error
CLOSE_YEAR_BLOCKED saying the Run can only be rejected.

Found by the independent verifier (claude-fable-5-2) during the review of
close-year-gates-show-content, confirmed by a probe test there. Predates that
change.
