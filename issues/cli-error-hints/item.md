---
created: 2026-11-21
updated: 2026-11-21
type: feature
status: done
priority: normal
preparation: ready
lane: agent-surface
lane_seq: 2
collision: [crates/cli/src/main.rs]
closed: 2026-11-21
closed_by: claude-opus-5-5
---

# Error hints and aggountant explain <CODE> from the guide's error-code reference

## Description

The skills explain some 30 error codes in advance, although an agent needs the
explanation at the moment a code arrives. The guide already has the
explanations (`docs/user/{fi,en}/reference/error-codes.md`).

## Wanted

The program offers the explanation when it matters: a short `hint` in the
error JSON for codes that have one, and/or `aggountant explain <CODE>`
printing the guide's entry (bundled, offline). One source: generate from or
test against `error-codes.md` so the two cannot drift. Then the skills can drop
their code explanations.

## Resolution

### 2026-11-21T07:46:39Z · @claude-opus-5-5

Implemented: aggountant explain and the error JSON hint, both read from the bundled error-code guide page. See result.md.
