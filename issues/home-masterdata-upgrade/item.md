---
created: 2026-11-11
updated: 2026-11-11
type: feature
reporter: jari
status: open
priority: high
preparation: ready
lane: home-upgrade
lane_seq: 1
collision: [crates/cli/src/main.rs, crates/ledger]
---

# Upgrade an existing company home's masterdata for a newer binary

## Description

A newer aggountant can need masterdata that an older `init` never wrote:
new bundled chart snapshots and account role metadata. Today the only way to
get them into an existing home is to copy files from a freshly initialised
temporary home, which an ordinary user cannot be expected to do and which can
silently replace company-approved masterdata.

Add an idempotent upgrade: a dry-run that lists what would be added, and an
apply that adds only missing bundled snapshots and role metadata. It never
changes the company's active chart choice and never breaks verification of
vouchers sealed under older masterdata.

## Acceptance Criteria

- [ ] Dry-run lists missing masterdata without writing; apply adds exactly that list and a second run is a no-op.
- [ ] The active chart and existing masterdata files are byte-identical after apply.
- [ ] Vouchers sealed before the upgrade still verify; a test opens a home written by an older binary.
