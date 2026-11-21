---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: done
priority: normal
preparation: ready
lane: year-end-surface
lane_seq: 2
collision: [crates/cli/src/main.rs]
closed: 2026-11-21
---

# close-year help: plain descriptions instead of design-document labels

## Description

`--help` uses internal labels (RF6/S6, D7, SealYear reconciliation) an agent cannot resolve; the skill had to explain what a correction is. Wanted: plain descriptions.

Found by the `skill-close-year` worker (run 01m7yftv83pzx3er3gew04q7s9) while fact-checking the new skill against a synthetic year-end close.
