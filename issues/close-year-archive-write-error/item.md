---
created: 2026-11-21
updated: 2026-11-21
type: bug
status: fixed
priority: normal
preparation: ready
lane: year-end-surface
lane_seq: 5
collision: [crates/cli/src/main.rs]
closed: 2026-11-21
---

# close-year: a failed archive write after SealYear is silent

## Description

`run_seal_year` discards the result of `write_archive_stub`, so a failed write leaves a sealed year with no `archive/<year>/tilinpaatos.json` and no message. Rare, and the books are right either way. Wanted: report the failure and offer a way to rewrite the archive.

Found by the `skill-close-year` worker (run 01m7yftv83pzx3er3gew04q7s9) while fact-checking the new skill against a synthetic year-end close.
