---
created: 2026-11-11
updated: 2026-11-11
type: task
reporter: jari
status: open
priority: normal
preparation: ready
lane: skills-contract
lane_seq: 1
collision: [skills, crates/cli/tests]
---

# Run the bundled skills' examples against the shipped binary

## Description

The skills teach the agent with example commands, JSON inputs and enum
values. Packaging tests only prove the skills are bundled byte for byte, not
that the examples still work. A stale example teaches the agent a wrong
bookkeeping call.

Extend the existing skill packaging test path to execute every documented
command and strict JSON example against the built binary, with no-write
dry-runs, including against a home written by an older binary.

## Acceptance Criteria

- [ ] Every command example in `skills/` is executed by a test, or explicitly marked illustrative.
- [ ] A deliberately wrong enum value in a skill fails the test.
