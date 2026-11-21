---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: done
priority: high
preparation: ready
lane: agent-surface
lane_seq: 0
collision: [crates/cli/src/main.rs, crates/cli/src/wage.rs]
closed: 2026-11-21
closed_by: claude-opus-5-5
commits:
- hash: 3501adb
  summary: 'cli: INPUT section in --help for every stdin JSON command'
- hash: 5e0b48e
  summary: 'cli: fix INPUT help findings from review'
---

# INPUT section in --help for every stdin JSON command

## Description

Several commands read JSON on stdin but their `--help` does not show the
shape, so the skills carry it instead ("the help does not describe the stdin
JSON, so the shape is here"). `voucher draft|validate|seal` already have an
INPUT section (`VOUCHER_*_INPUT_HELP` in `crates/cli/src/main.rs`) that shows
the help can carry it.

## Wanted

The same INPUT section, with field meanings, required/optional and one
synthetic example, for every stdin command that lacks it, at least:
`capture-receipt record`, `vero-fi configure`, `file-vat start`,
`file-wage preview`/`start`, `import camt053 seal` (the per-entry assignment
shape the bank import needs), `knowledge company add|update|retire`. Check
the full command tree for others. A test asserts each stdin command has an
INPUT section. Skills are not edited here; the skill rebuild removes the
copies afterwards.

## Resolution

### 2026-11-21T07:19:19Z · @claude-opus-5-5

Done: every stdin JSON command has an INPUT section in --help, enforced by a tree-walk test; see result.md.
