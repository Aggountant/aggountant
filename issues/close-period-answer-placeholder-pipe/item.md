---
created: 2026-11-21
updated: 2026-11-21
type: bug
status: fixed
priority: high
provenance: ai-review
source_ref: taskfleet:01m7yzes4qee1wjprvjzqjv88g/review-finding:F1-close-period
review_source: ai-review
originating_run: 01m7yzes4qee1wjprvjzqjv88g
originating_run_kind: spinoff
labels:
- ai-review-model:gpt-6-astra
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_confidence: high
review_severity: must-fix
review_target: close-year-next-command slice diff
lane: agent-surface
lane_seq: 0
collision: [crates/process/src/close_period.rs]
preparation: ready
closed: 2026-11-21
commits:
- hash: 5a1c367
  summary: 'close-period: quoted answer placeholder in next_command'
---

# close-period next_command: --answer approve|reject runs as a shell pipe and approves

## Description

## Problem

`ProcessEngine::close_period_view` (`crates/process/src/close_period.rs`, the
`answer` string built for the open `ApproveClose` gate) returns

    next_command: aggountant run answer --run-id cp-… --gate ApproveClose --answer approve|reject

SOUL tells agents to run `next_command` rather than infer the route. Run
literally in a shell, `approve|reject` is a pipe: the left side runs
`aggountant run answer … --answer approve`, which seals the period permanently,
and the right side fails with `reject: command not found`. The approval
happens without anyone choosing it, and the shell error may make it look as if
nothing ran.

Reproduction of the shell parse (no aggountant needed):

    bash -c 'f(){ echo "argv: $*"; }; f --answer approve|reject'
    # → "reject: command not found"; f ran with --answer approve

## Wanted

The same treatment close-year got in `close-year-next-command`: a quoted
placeholder (`--answer '<approve|reject>'`), which a shell passes as one
argument that `run answer` refuses with `INVALID_ANSWER`, and a message saying
to put approve or reject in its place. Update the unit test in
`close_period.rs` that asserts `contains("approve|reject")`, and any skill or
guide text that shows the command.

Found by the `/llm-review` of the close-year next-command slice (finder
gpt-6-astra), which raised it for close-year; the same string exists here.

## Resolution

### 2026-11-21T14:54:36Z · @issuectl

Fixed: next_command carries the quoted placeholder '<approve|reject>' that run answer refuses; a|b help texts reworded. See result.md, review.md, assessment.md.
