---
created: 2026-11-21
updated: 2026-11-21
type: feature
status: done
priority: normal
preparation: ready
lane: year-end-surface
lane_seq: 4
collision: [crates/cli/src/main.rs]
closed: 2026-11-21
closed_by: claude-fable-5-2
---

# Fixed-asset register: carry assets and book values from one close-year to the next

## Description

Opening book values are typed in every year, and a wrong one silently gives wrong depreciation in a year that is then sealed. The skill routes them through `knowledge company` as a stopgap. Wanted: assets and closing book values carried forward by the program.

Found by the `skill-close-year` worker (run 01m7yftv83pzx3er3gew04q7s9) while fact-checking the new skill against a synthetic year-end close.

## Resolution

### 2026-11-21T12:18:26Z · @claude-fable-5-2

Built: the register is stored with the year seal in the fiscal store and carried by the next close-year; see result.md, review.md and assessment.md.
