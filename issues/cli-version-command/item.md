---
created: 2026-11-10
updated: 2026-11-10
type: improvement
status: open
priority: normal
labels: [cli-canon]
lane: cli-canon
lane_seq: 0
collision: [crates/cli/src/main.rs]
---

# Add a version --json subcommand

## Problem

`aggountant version --json` fails with `unrecognized subcommand 'version'`; only
clap's `--version` text exists. The AI-first CLI canon lists `version` as a
lifecycle verb with a JSON drift contract (package version, commit, schema
versions).

Without it, release tooling cannot match a built binary to the commit it was
built from, so a README command check against the built binary cannot run, and
skills have no machine-readable way to read the binary's commit. The commit is
already embedded at build time (`AGGOUNTANT_GIT_COMMIT`, used for provenance in
`crates/process/src/lib.rs`).

## Expected

A `version` subcommand that emits the canon's JSON envelope: package version,
git commit, JSON contract version and the number of bundled skills.

## Acceptance criteria

- [ ] `aggountant version --json` returns the envelope with exit code 0.
- [ ] Text output without `--json` stays human-readable.
- [ ] A CLI test asserts the fields.
