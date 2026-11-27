---
created: 2026-11-27
updated: 2026-11-27
type: bug
status: open
priority: high
provenance: other
provenance_detail: observed during the pii migration copy comparison of claimant-workplace-register
source_ref: taskfleet:01m8ehz1eezcc03sc8dzm8xy55/observation:downgrade-reset-message
originating_run: 01m8ehz1eezcc03sc8dzm8xy55
originating_run_kind: spinoff
lane: store-safety
lane_seq: 10
collision: [crates/ledger/src/store]
---

# Store version refusals tell the operator to reset a home that holds live books

## Description

## Problem

When an older aggountant binary opens a store a newer binary has migrated,
`crates/ledger/src/store/migrations.rs` (`check_version_range`) refuses it
with:

    STORE: pii database schema is v2 but this binary only knows up to v1 — the
    database was written by a newer aggountant; upgrade the binary (or reset
    this home, its data is resettable)

The same wording ("its data is resettable", "reset this home") appears for a
pre-framework database and in the fiscal store's messages
(`crates/ledger/src/store/sqlite_fiscal.rs`). It dates from the pre-production
phase. Companies now keep their live books in these stores: the fiscal store
holds sealed vouchers and filed returns, the personal-data store employee and
claimant records. Telling an operator who rolled back a binary to "reset this
home" invites destroying the only copy of real bookkeeping, and the error JSON
is what an agent reads and may act on.

Observed while testing the one-way pii v1 -> v2 migration of
`claimant-workplace-register`: a synthetic home (Esimerkki Oy) migrated by the
new binary and then opened by the previous release printed exactly this
message.

## Expected

The downgrade refusal says only what is safe: the home was written by a newer
aggountant, install that version (or newer) again, and do not modify or reset
the home. The pre-framework refusal likewise stops suggesting a reset for
homes that may hold real books; a reset, if it ever applies, belongs in the
user guide's troubleshooting page with the conditions under which it is safe.

## Decisions

### 2026-11-27T17:52:58Z · @conductor

Product owner 2026-11-27: accepted for immediate execution; both production homes now hold a pii v2 store that the previous build refuses with this message.
