---
created: 2026-11-18
updated: 2026-11-18
type: chore
status: untriaged
priority: normal
provenance: other
provenance_detail: follow-up decided with terribly-steadfast-birds
source_ref: taskfleet:01m7q5ksc4yp9nhd6egpt8chbq/followup:chrono-migration
originating_run: 01m7q5ksc4yp9nhd6egpt8chbq
originating_run_kind: spinoff
---

# Migrate from chrono to jiff or time

## Description

# Migrate from chrono to jiff or time

RUSTSEC-2026-0543 marks the `chrono` crate unmaintained (all versions; no safe
upgrade). It is an unmaintained-crate notice, not a vulnerability, so
`deny.toml` ignores it for now (issue @terribly-steadfast-birds). That ignore
is temporary: this item tracks the migration.

`chrono` 0.4 is a direct dependency of the CLI, broker, domain and process
crates. Replace it with `jiff` or `time` (choose one; the advisory recommends
both), then remove RUSTSEC-2026-0543 from `[advisories] ignore` in `deny.toml`.

Care points: dates drive fiscal periods, VAT periods and filing deadlines, and
timestamps appear in sealed records and authority payloads, so serialised
formats must stay byte-identical and existing tests must pass unchanged.
