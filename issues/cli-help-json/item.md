---
created: 2026-11-10
updated: 2026-11-10
type: improvement
status: open
priority: normal
labels: [cli-canon]
lane: cli-canon
lane_seq: 60
collision: [crates/cli/src/main.rs]
---

# Machine-readable help with --help --json

## Problem

Text `--help` works and drills down per subcommand, but there is no
machine-readable help. An agent has to scrape prose to discover subcommands,
flags and environment variables. The AI-first CLI canon (§14) expects
`--help --json`.

## Expected

`--help --json` (on the root and on every subcommand) emits a structured help
document: subcommands, flags with types and defaults, `examples[]`, and
environment-variable mappings.

## Acceptance criteria

- [ ] `aggountant --help --json` and `aggountant <subcommand> --help --json` emit valid JSON.
- [ ] The document is generated from the clap definition, not maintained by hand.
- [ ] A test checks one subcommand's flags and examples.
