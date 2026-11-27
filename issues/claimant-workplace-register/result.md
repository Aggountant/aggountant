# Result: claimant-workplace-register

A claimant's regular workplace, secondary workplace and tracked sites, their
relationship to the company and part-ownership are now dated, confirmed facts
in the personal-data store, and the travel computation takes each trip's
workplace from them.

```
aggountant claimant place add          < place.json
aggountant claimant add                < fact.json
aggountant claimant correct            < correction.json
aggountant claimant show --employee E --date D [--as-known-at T]
aggountant claimant history --employee E
aggountant claimant snapshot           < trip.json
aggountant claim compute               < claim.json   (now reads the register)
```

## Store migration

`masterdata/pii.sqlite` goes from **schema v1 to v2** the first time any
command opens a home. The migration adds two empty tables
(`claimant_place`, `claimant_fact`), an index and two append-only triggers; it
changes no existing row or table. It is one-way: an older binary refuses a v2
store ("written by a newer aggountant"), so back up both companies' stores
before installing. The CHANGELOG says the same.

Copy comparison against a build of `main` at `45f3a8f`: a synthetic home
(Esimerkki Oy: one employee, two sealed vouchers, a company fact) was created
and used by the `main` binary, copied, and 14 existing commands were run with
the `main` binary on the original and with this branch's binary on the copy
(chart list, voucher list/show/verify/vat-codes, period show, fiscal-year
list, employee list/show with and without the year-to-date, knowledge company
list, obligation list, home upgrade --dry-run). All exited 0 and the outputs
were identical except `home upgrade`'s echo of the home's own path. The
personal-data store differed only in `user_version` (1 → 2) and the two new
empty tables; every v1 row and table definition was identical, and so were
all files outside the SQLite stores. The `main` binary refuses the migrated
copy, as expected. The same property is a repository test
(`a_home_without_claims_upgrades_with_unchanged_output`, which rewinds a home
to v1, copies it and compares).

## What was built

- `crates/domain/src/workplace.rs` (pure): place kinds, workplace roles
  (`regular_primary`, `regular_other`, `secondary`, `tracked_site`, `none`),
  relationships (`employee`, `managing_director`, `board_member`,
  `unpaid_owner`, `other_fee_recipient`; `contractor` is refused,
  `CONTRACTOR_NOT_CLAIMANT`), part-ownership, and the closed list of basis
  codes, each tied to its passage of VH/630/00.01.00/2026 (checked against
  the guidance on vero.fi): `works_there_regularly` 3.1,
  `company_premises_in_home` 3.4, `agreed_home_work_occasional_visits` 3.4
  and 10.7, `mobile_work_base` 3.5, `remote_work_by_choice` 10.7,
  `employer_premises_weekly` 5.2.2, 10.7 and KHO 2017:178,
  `premises_in_another_municipality` 4.1 and 5.2.2,
  `occasional_visits` 5.2.1, `fixed_term_assignment` 5.2.2 and 5.3,
  `board_member_only` 10.1, `mobile_work_no_base` 3.5, and `other` with a
  required explanation. Each code states which roles (and for some, which
  place kinds) it can justify.
  - Bitemporal fold: `known_at` (rows recorded by T, minus those superseded
    by T, minus retractions), `register_view` for a date as known at T, with
    a `facts_digest` over the rows and places used.
  - Write rules: one primary regular workplace at a time; "none" excludes
    regular and secondary entries; one role per place at a time; a secondary
    workplace must be employer or interest-group premises (not a home or a
    customer site, KHO 2017:178) in another municipality than the primary
    (4.1); relationships may overlap but not repeat; part-owner facts never
    overlap; a row is corrected once; a retraction is not corrected; a
    replaced place cannot be named; a clock behind the register refuses.
  - `trip_snapshot` for a trip's start date refuses without a relationship
    (`CLAIMANT_RELATIONSHIP_UNKNOWN`) or a regular-workplace entry
    (`REGULAR_WORKPLACE_UNKNOWN`), for the home as a destination
    (`TRIP_TO_HOME`), and for an unknown or replaced place; it classifies
    each place, and `TripSnapshot::workplace` gives the trip's workplace.
- `crates/ledger/src/claimant.rs` and `store/sqlite_pii.rs`: the store port
  runs the Domain's validation inside a `BEGIN IMMEDIATE` transaction and
  appends; rows are read back through the strict types; an UPDATE aborts; the
  person's erasure cascades places, facts and corrections.
- `crates/cli/src/claimant.rs`: the commands above, JSON on stdin, `--json`
  output, INPUT help with examples.
- Integration (`crates/domain/src/travel/fi`): `ClaimInput` names the
  claimant and each trip its registered `places`; `compute_claim` takes the
  register rows and classifies every trip from the snapshot for the Finnish
  date it starts on. A register refusal is a per-trip refusal in the result
  and the trip computes and pays nothing; a regular workplace is refused
  (`TRAVEL_TO_REGULAR_WORKPLACE`); a secondary workplace earns travel costs
  only, and its note says the lodging-to-workplace journeys are not tax-free
  travel (decision 970/2025 4 § 2 mom, guidance 4.3); a secondary workplace
  together with a registered special site is refused as a workplace change; a
  null regular-workplace distance is accepted only when the register records
  no regular workplace. The `workplace_from_trip_input` note is gone, and a
  caller can no longer supply the classification. `claim compute` now reads
  the company home.
- Guide (fi, en): new task page "Kirjaa varsinainen työpaikka" / "Record your
  regular workplace"; the glossary gained varsinainen työpaikka, toissijainen
  työpaikka, erityinen työntekemispaikka, matkavuorokausi and ilmainen ateria
  with the plain explanations proposed in the issue; error codes, command map,
  status, company-home table, and the calculator's page updated for the
  register.

## Acceptance criteria

- [x] A home without claims opens and upgrades unchanged; the migration is
  additive (store test, CLI test, copy comparison above).
- [x] Bitemporal query: a backward correction changes "as known now" but not
  "as known at T" (domain, ledger and CLI tests).
- [x] Refusal codes covered by tests with synthetic people (Esimerkki Oy's
  Maija Meikäläinen), including the travel computation's refusals through
  the register.
- [x] No personal data goes to git or the fiscal store: the register lives in
  the gitignored PII store; a CLI test scans every other file of the home for
  a recorded address.
- [x] User guide and glossary use the legal terms with the plain
  explanations.
- [x] Copy-comparison test; the regular-workplace question exists only on
  the claimant path (`claimant snapshot`, `claim compute`), and existing
  commands' output never mentions it.

## Decisions

- Parent decided `wait` when the calculator was not yet on `main`; it landed
  as `45f3a8f` and the integration is in this slice.
- Places are per person, so erasing a person erases their home address, and
  never edited: a corrected place names the place it `replaces`.
- One generic `claimant add` / `correct` instead of per-kind commands: the
  fact JSON carries its kind, so per-kind verbs would only repeat it.
- The register's trip-level `trip_scope` was removed in favour of the
  calculator's `Workplace` and refusal, so the rule has one home.

## Not done here, and why

- **Asking again only with a reason** (a trip pattern contradicting the entry,
  a new recurring destination, a return after a six-month gap): these need the
  claimant's trip history, which `expense-claim-draft` creates, and "a place
  recorded as occasional visited weekly" is already a warning in
  `three-year-site-clock`. Nothing in this slice asks yearly; a changed
  assignment end shows as `REGULAR_WORKPLACE_UNKNOWN` once the entry ends.
  Proposed for `expense-claim-draft`.
- **Trip receipt costs without an entry**: receipt lines are not part of the
  computation yet. The draft slice must apply the same snapshot to receipt
  lines attached to a trip.
- **Listing every destination**: a secondary workplace plus an unregistered
  customer is classified secondary (under-pays only; the note says to split).
  Proposed for `expense-claim-draft`, which builds trips.
- **Municipality codes**: filed as `claimant-municipality-codes`.
- **Store refusals that suggest resetting a home**: observed in the copy
  comparison, filed as `store-downgrade-message-suggests-reset`.

Review and assessment: `review.md`, `assessment.md` beside this file.
