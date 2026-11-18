---
created: 2026-11-18
updated: 2026-11-18
type: chore
status: duplicate
priority: high
provenance: other
provenance_detail: observed gate failure in a taskfleet worker
source_ref: taskfleet:01m7q4nbceha52vkzwr4ahjzej/observed:cargo-deny-chrono
originating_run: 01m7q4nbceha52vkzwr4ahjzej
originating_run_kind: spinoff
closed: 2026-11-18
---

# cargo-deny fails on RUSTSEC-2026-0543 (chrono unmaintained), blocking the local and GitHub CI gate

## Description

`scripts/ci-local.sh` stops at the cargo-deny step on current `main`:
`cargo deny check advisories` reports RUSTSEC-2026-0543, "chrono is
unmaintained; all versions are affected" (chrono 0.4.45 in `Cargo.lock`),
recommending `jiff` or `time`. Bans, licenses and sources pass, and so do the
build and test steps after it when run by hand. GitHub Actions runs the same
check, so pull requests and pushes to `main` are red until this is decided.

Observed on 2026-11-18 by a close-year worker whose diff does not touch
dependencies; `Cargo.lock` was identical to `main`.

Options: record a reasoned `ignore` for RUSTSEC-2026-0543 in `deny.toml` while
the migration is planned, or replace chrono (directly and in whichever
dependencies pull it in) with `jiff` or `time`. An "unmaintained" advisory is
not a known vulnerability.

## Resolution

### 2026-11-18T11:10:12Z · @issuectl

Duplicate of terribly-steadfast-birds.
