---
created: 2026-11-18
updated: 2026-11-18
type: chore
status: in-progress
priority: high
provenance: other
provenance_detail: observed by CI gate in a taskfleet worker
source_ref: taskfleet:01m7q4nbnvz9hmns7rtdqygffv/ci-gate:rustsec-2026-0543
originating_run: 01m7q4nbnvz9hmns7rtdqygffv
originating_run_kind: spinoff
lane: unlaned
collision: [deny.toml]
preparation: ready
---

# cargo-deny fails on main: chrono unmaintained (RUSTSEC-2026-0543)

## Description

`scripts/ci-local.sh` fails at its `cargo deny check advisories bans licenses sources`
step on current `main` (observed 2026-11-18, cargo-deny 0.20.2):

    error[unmaintained]: chrono is unmaintained
      ID: RUSTSEC-2026-0543
      The chrono crate is unmaintained; all versions are affected.
      Recommended alternatives: jiff, time
      Solution: No safe upgrade is available!

`deny.toml` has `[advisories] ignore = []`, and chrono 0.4.45 is a direct
dependency of the CLI, broker, domain and process crates. The script stops at
this step (`set -e`), so the build and test steps on stable and MSRV after it
never run locally, and GitHub Actions runs the same check, so `main` is red for
every change regardless of content.

Decision needed: either add RUSTSEC-2026-0543 to `ignore` in `deny.toml` with a
reason and a follow-up to migrate (to `jiff` or `time`), or migrate now. An
unmaintained-crate advisory is not a vulnerability, so an ignore with a tracked
migration is the usual short-term choice.
