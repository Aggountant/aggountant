---
created: 2026-11-10
updated: 2026-11-10
type: improvement
status: open
priority: normal
labels: [cli-canon]
lane: cli-canon
lane_seq: 50
collision: [crates/cli/src/main.rs]
---

# Add a read-only doctor self-diagnostic

## Problem

There is no cheap way for an agent to ask whether aggountant and the current
home are healthy and correctly configured. `aggountant doctor` is an
unrecognized subcommand. The AI-first CLI canon (§18) expects a read-only
self-diagnostic.

## Expected

A read-only `doctor` that checks the resolved home and its configuration (for
example: home resolution and source, store opens and verifies, chart activated,
installed skills match the binary, Vero.fi configuration readable). `--json`
emits per-check `{id, status, message, fix_suggestion}` plus a
`summary{ok, warn, fail}`; the exit code is 1 when any check fails. `--fix` is
optional and only for repairs that are safe to automate.

## Acceptance criteria

- [ ] `aggountant doctor --json` reports each check and a summary.
- [ ] Exit code 1 on a failed check, 0 otherwise.
- [ ] `doctor` writes nothing without `--fix`.
- [ ] Documented in `docs/user/`.
