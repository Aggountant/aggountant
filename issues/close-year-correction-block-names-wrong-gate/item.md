---
created: 2026-11-16
updated: 2026-11-16
type: bug
status: in-progress
priority: normal
provenance: other
provenance_detail: found while rewriting close-year-gates-show-content
source_ref: taskfleet:01m7j4n4exnmqcdn8mzhy5safr/close-year-correction-gate-message
originating_run: 01m7j4n4exnmqcdn8mzhy5safr
originating_run_kind: spinoff
lane: close-year
lane_seq: 2
collision: [crates/process]
preparation: ready
---

# close-year names the wrong gate when unsealed corrections block ApproveAdjustments

## Description

## What goes wrong

When the user tries to approve `ApproveAdjustments` in a `close-year` run while
a D7 correction is still unsealed, the refusal says
`ApproveStatements is blocked by N unsealed D7 correction(s): [...]`.
The gate that is blocked is `ApproveAdjustments`. Since the change in
`close-year-adjustment-attribution`, corrections have to be sealed before
`ApproveAdjustments`, not before `ApproveStatements`. The message still names
the old gate, so the user or agent is told the wrong step.

## Where

`assert_corrections_sealed` in `crates/process/src/close_year.rs`. Its only
caller is the `GATE_APPROVE_ADJ` arm in `answer_close_year`. The doc comment
above it is already correct.

## Fix

Name `ApproveAdjustments` in the message, and say that the listed corrections
are sealed with `close-year seal-correction`. Add an assertion on the message in
the existing test `a_d7_correction_must_be_sealed_before_approve_adjustments`.
