---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: open
priority: normal
preparation: ready
lane: agent-surface
lane_seq: 1
collision: [crates/cli/src/main.rs]
---

# vero-fi configure merges into the stored profile instead of replacing it

## Description

`vero-fi configure` replaces the whole stored profile with the stdin JSON.
A partial update silently drops stored evidence (attestations, verification
results); the `vero-fi` skill carries a warning about it because nothing in
the program prevents it.

## Wanted

`configure` merges the given fields into the stored profile, or refuses an
input that would drop stored evidence unless an explicit flag says so, and
`--dry-run` shows what would change. Then the skill's warning can go. Help and
guide updated.
