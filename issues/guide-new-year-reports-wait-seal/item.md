---
created: 2026-11-21
updated: 2026-11-21
type: task
status: done
priority: normal
preparation: ready
lane: year-end-surface
lane_seq: 6
collision: [crates/cli/src/main.rs]
closed: 2026-11-21
closed_by: claude-opus-5-5
---

# Guide: the new year's monthly reports wait for the prior year's seal

## Description

`report statements` for a month of the new fiscal year refuses with `PERIOD_NOT_SEALED` while the prior year is open. `tasks/year-end.md` and `tasks/monthly-report.md` do not say so, and users meet it every spring. Wanted: both guide pages (fi, en) say it, and the refusal message names the cause.

Found by the `skill-close-year` worker (run 01m7yftv83pzx3er3gew04q7s9) while fact-checking the new skill against a synthetic year-end close.

## Resolution

### 2026-11-21T12:04:55Z · @claude-opus-5-5

Refusal message and guide pages name the cause and the way out; see result.md.
