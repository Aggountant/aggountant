---
created: 2026-11-21
updated: 2026-11-21
type: feature
status: open
priority: normal
preparation: ready
lane: year-end-surface
lane_seq: 3
collision: [crates/cli/src/main.rs]
---

# close-year: show depreciation rates and computed depreciation without starting a run

## Description

Rate and method per asset class are visible only in the output of `close-year start`. Wanted: a read or `--dry-run` that shows them, as the CLI canon expects.

Found by the `skill-close-year` worker (run 01m7yftv83pzx3er3gew04q7s9) while fact-checking the new skill against a synthetic year-end close.
