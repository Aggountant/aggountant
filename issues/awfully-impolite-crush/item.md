---
created: 2026-11-18
updated: 2026-11-18
type: chore
status: untriaged
priority: high
provenance: other
provenance_detail: observed while running the local CI gate
source_ref: taskfleet:01m7q4nbywn89zhtgd1bgv0819/observation:cargo-deny-chrono-unmaintained
originating_run: 01m7q4nbywn89zhtgd1bgv0819
originating_run_kind: spinoff
---

# cargo-deny fails on main: RUSTSEC-2026-0543 (chrono unmaintained)

## Description

## Problem

`scripts/ci-local.sh` stops at `cargo deny check advisories`: the RustSec
database now carries RUSTSEC-2026-0543, "chrono is unmaintained; all versions
are affected". `deny.toml` has `[advisories] ignore = []`, so the unmaintained
advisory fails the gate. This happens on `main` as it is (checked at
`e82307d` on 2026-11-18 in a clean detached worktree), so every branch's local
gate, and presumably the GitHub Actions run, is red regardless of its change.

chrono is a workspace dependency (`Cargo.toml`, features `clock`, `serde`)
used by broker, cli, domain, ledger, model and process.

## Impact

The gate cannot go green, so workers either cannot merge under "a required step
that fails blocks merge" or must run the remaining steps by hand.

## Options

- Record a reasoned, dated ignore for RUSTSEC-2026-0543 in `deny.toml` while a
  replacement is chosen (chrono is still functionally correct; the advisory is
  about maintenance, not a vulnerability).
- Replace chrono (for example with `jiff` or `time`) across the workspace.
