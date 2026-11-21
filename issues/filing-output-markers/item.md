---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: open
priority: normal
preparation: ready
lane: agent-surface
lane_seq: 3
collision: [crates/cli/src/wage.rs, crates/process/src/file_vat.rs]
---

# Outputs mark sensitive record_hash and the active test seam; specific code for an unknown VAT class

## Description

Some facts the skills carry belong in the output itself:

- `record_hash` in `file-wage` previews is derived from personal identity data
  and is sensitive, which the skill has to say.
- When `AGGOUNTANT_BROKER_SCRIPT` is set, `file-vat` can record a period as
  filed without sending anything; only the skill warns about it.
- `capture-receipt record` with an unknown VAT class returns `INVALID_ANSWER`
  with a message repeating `VAT_CODE:`; the code is misleading on that path.

## Wanted

The preview output marks `record_hash` as sensitive (a field or note), the
filing commands say visibly when the test seam is active, and the unknown VAT
class gets a specific code and a clean message. Tests for each.
