---
created: 2026-11-18
updated: 2026-11-21
type: chore
status: done
priority: normal
provenance: other
provenance_detail: follow-up decided with terribly-steadfast-birds
source_ref: taskfleet:01m7q5ksc4yp9nhd6egpt8chbq/followup:chrono-migration
originating_run: 01m7q5ksc4yp9nhd6egpt8chbq
originating_run_kind: spinoff
lane: dependencies
lane_seq: 1
preparation: ready
closed: 2026-11-21
closed_by: claude-opus-5-5
commits:
- hash: "2206072"
  summary: Pin date and time text with golden tests before leaving chrono
- hash: "b671964"
  summary: Replace chrono with jiff behind model::Instant and model::Date
- hash: "4527735"
  summary: Drop the chrono advisory ignore and land ADR-9
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

## Decisions

### 2026-11-19T13:59:20Z · @taskfleet:01m7t184f3t2yyew9avwgwy8xa

Decision proposed in ADR-9 (docs/site/adr-9-date-time-library.md, status Proposed, decider Jari Mustonen): replace chrono with jiff 1.x (default features off, std + serde, no tzdb). time lost on silent default formats (tuple serde, non-ISO Display), no month arithmetic and no FromStr; staying on chrono keeps a permanent deny ignore. Byte-identical output comes from one instant newtype in model reproducing chrono's AutoSi RFC 3339; probes showed 0 mismatches for both candidates with that helper. The ADR's migration plan starts with golden tests on chrono.

## Resolution

### 2026-11-21T05:17:59Z · @claude-opus-5-5

Built per ADR-9: chrono replaced by jiff behind model::Instant and model::Date, golden tests unchanged since the chrono commit, ci-local green on stable and 1.89, RUSTSEC-2026-0543 ignore removed. See result.md.
