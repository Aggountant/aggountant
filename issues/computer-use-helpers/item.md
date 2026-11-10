---
created: 2026-11-10
updated: 2026-11-10
type: feature
status: open
priority: normal
---

# Computer-use helpers for other accounting programs in their own directory

## Description

aggountant needs a home for helpers that let an agent work with other
accounting and banking programs through computer use (browser automation),
for example to fetch statements, vouchers and reports from a previous
bookkeeping system or a bank when moving books into aggountant. Each program
gets its own subdirectory (for example `helpers/briox/`, `helpers/holvi/`) with
its notes and code, and program-specific knowledge lives there, not in the core
crates.

A first set exists outside this repository: a macOS Chrome driver over
AppleScript, a Holvi download checker with a browser PDF fallback, and a Briox
attachment saver, standard-library Python with synthetic tests. Bring them in
with neutral instructions: what each helper does, what the user does (signs in,
MFA, permission dialogs), read-only use of the other system, no cookies or
tokens extracted, raw data kept outside git, completeness checked against an
inventory and SHA-256.

Also move Briox-specific explanations out of core code comments (for example
`crates/domain/src/vat/expand.rs`) into the helper's notes where they explain a
source chart rather than aggountant behaviour.

## Acceptance Criteria

- [ ] `helpers/<program>/` layout with an `AGENTS.md` explaining the shared rules.
- [ ] Chrome, Holvi and Briox helpers and their tests present and passing in `scripts/ci-local.sh`.
- [ ] Core comments no longer depend on one vendor's chart.
