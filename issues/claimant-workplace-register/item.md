---
created: 2026-11-24
updated: 2026-11-24
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:claimant-workplace-register
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
---

# Dated regular-workplace register and claimant facts in the personal-data store

## Description

## Goal

A claimant's regular workplace (varsinainen työpaikka), secondary workplace
and tracked customer sites are recorded as dated, confirmed facts, and so are
the claimant's relationship to the company and part ownership. The user can
tell the agent "the company's premises are in my home since 2024-01-01" and
see it recorded with its basis, and ask what was in force on a given date.
Decided in ADR-11 (`docs/site/adr-11-expense-claim.md`, sections 1 and 3).

## Scope

- New append-only, bitemporal tables in `masterdata/pii.sqlite` (additive
  migration): places (kind, label, location, municipality), role
  assignments (regular primary/other, secondary, tracked site, explicit
  "none") with `valid_from`/`valid_to`, `recorded_at`, a closed list of basis
  codes tied to the guidance (`other` needs text), confirmer and evidence
  hash, `supersedes` + reason; relationship (employee, managing director,
  board member, unpaid shareholder, other personal-fee recipient) and
  part-owner facts, dated.
- CLI: `claimant workplace add|correct|show --date` (names indicative), JSON
  on stdin, `--json` output; a correction never edits a row.
- The query "roles in force for person P on date D, as known now".
- The travel computation refuses tax-free travel allowances and trip receipt
  costs without an entry for the trip's date (`REGULAR_WORKPLACE_UNKNOWN`),
  refuses a trip to a regular workplace, and limits a secondary workplace to
  travel and lodging costs, excluding the lodging-to-workplace journey
  (decision 970/2025, 4 § 2 mom).
- A contractor or consultant cannot be a claimant.

## Acceptance criteria

- [ ] A home without claims opens and upgrades unchanged; the migration is additive.
- [ ] Bitemporal query tested: a backward correction changes "as known now" but not "as known at T".
- [ ] Refusal codes covered by tests with synthetic people (Esimerkki Oy).
- [ ] No personal data goes to git or the fiscal store.
- [ ] User guide uses the legal terms with the plain explanation in parentheses (ADR-11 section 11) and adds them to the glossary.

## Likely files

`crates/ledger/src/store/sqlite_pii.rs`, `crates/ledger/src/employee.rs` (or a
new `claimant.rs`), `crates/ledger/src/lib.rs`, `crates/cli/src/main.rs`,
`crates/domain/src/travel/`, `docs/user/{fi,en}/`.

## Dependencies

The refusal integration needs the calculator slice
(`travel-allowance-rates-and-calculator`); the register itself does not.
Related: `employee-pii-entry` (tax card optional for travel-only claimants).

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Proposed 2026-11-24); decision issue `expense-claim-model`. Dependencies above are prose; the conductor sets `blocked_by` after triage.
