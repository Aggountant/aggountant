---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: open
priority: normal
preparation: ready
lane: year-end-surface
lane_seq: 1
collision: [crates/cli/src/main.rs]
---

# close-year output: message and next_command naming the open gate and what its approval seals

## Description

start, answer, seal-correction and resume return only the run view. SOUL tells agents to follow `next_command`; here they must infer the next gate from the gates list. Wanted: `message` and `next_command` naming the open gate and what approving it seals, so the skill can drop its own labelling of which approvals are permanent.

Found by the `skill-close-year` worker (run 01m7yftv83pzx3er3gew04q7s9) while fact-checking the new skill against a synthetic year-end close.
