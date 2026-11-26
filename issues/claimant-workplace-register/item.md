---
created: 2026-11-24
updated: 2026-11-26
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
Follows ADR-11 (`docs/site/adr-11-expense-claim.md`, sections 1 and 3);
this issue owns the register's shape, its basis codes and the refusals.

## Scope

- New append-only, bitemporal tables in `masterdata/pii.sqlite` (additive
  migration): places (kind: company premises, premises of a company in the
  same interest group, home, self-arranged space, customer site, venue,
  other; label, location for distances, municipality), role assignments
  (regular primary/other, secondary, tracked site, explicit "none" for
  example for an outside board member) with `valid_from`/`valid_to`,
  `recorded_at`, a basis code, confirmer and evidence hash, `supersedes` +
  reason; relationship (employee, managing director, board member, unpaid
  shareholder, other personal-fee recipient, EPL 13 §) and part-owner facts
  (the Incomes Register's "osakasasemassa maksajaan"), dated, never inferred.
- Basis codes: a closed list, each tied to the guidance passage it rests on,
  for example *company premises in the owner's home* (ohje 3.4), *employer
  premises visited weekly* (ohje 5.2.2, KHO 2017:178), *remote work by
  choice* (ohje 10.7); `other` needs an explanation. The list is settled in
  this slice; adding a code later is additive.
- One-way migration of `pii.sqlite` (allowed by the product owner
  2026-11-26): back up the store first and compare the migrated copy with
  the original before the slice is accepted.
- Snapshot input: for a date, the roles, the start place's role, the
  destination's classification (special work site, secondary workplace,
  regular workplace), the basis codes and the digest of the rows used, so a
  draft can freeze it into the trip.
- Asking again only with a reason: a trip pattern contradicting the entry
  (weekly visits to a place recorded as occasional), a new recurring
  destination, a changed assignment end, a return after a six-month gap. No
  yearly routine question.
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
- [ ] User guide uses the legal terms with a plain explanation in parentheses (ADR-11 section 9; product owner 2026-11-24) and adds them to the glossary. Proposed wording, settled in the guide: varsinainen työpaikka (paikka, jossa teet työtä vakituisesti, esimerkiksi oma toimisto tai kotitoimisto); toissijainen työpaikka (oman yhtiön toinen vakituinen toimipaikka toisessa kunnassa); erityinen työntekemispaikka (tilapäinen paikka, kuten asiakkaan luona); matkavuorokausi (24 tunnin jakso matkan alkamisesta, ei kalenteripäivä); ilmainen ateria (työnantajan tai esimerkiksi asiakkaan tarjoama ateria, ei hotellin aamiainen).

## Likely files

`crates/ledger/src/store/sqlite_pii.rs`, `crates/ledger/src/employee.rs` (or a
new `claimant.rs`), `crates/ledger/src/lib.rs`, `crates/cli/src/main.rs`,
`crates/domain/src/travel/`, `docs/user/{fi,en}/`.

## Dependencies

The refusal integration needs the calculator slice
(`travel-allowance-rates-and-calculator`); the register itself does not.
Related: `employee-pii-entry` (tax card optional for travel-only claimants).

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Accepted 2026-11-26); decision issue `expense-claim-model`. The ADR states the principle; the details above are this issue's own. Dependencies above are prose; the conductor sets `blocked_by` after triage.
