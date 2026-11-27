# Review: claimant-workplace-register

- Skill: `/llm-review`, rigor **critical**: the slice adds a one-way
  migration of `masterdata/pii.sqlite` that every command applies to every
  existing home, and it decides whether a trip earns tax-free allowances.
- Two rounds, because the calculator landed on `main` mid-slice and the
  integration was new code:
  1. the register, its migration, snapshot and CLI (`9808ba7`..`8dd0d95` on
     the rebased branch, before the review fixes);
  2. the integration with `domain::travel` (`d6c4c2f`).
- Finder: `gpt-6-astra`, one call per round, with the diff, the issue and
  ADR-11 (round 1) or `domain::workplace` (round 2). Verifier:
  `claude-fable-5-2`, one call per round, asked to refute every surviving
  candidate. Four model calls in total.
- Risk class: irreversible store migration; silently wrong tax treatment.

## Round 1

### R1-F1 (must-fix): the home as a destination passed as a work trip

- Location: `crates/domain/src/workplace.rs`, `trip_place` (role-less
  place → special work site) and the trip-level classification, which read
  destinations only.
- Claim: start `office` (regular) and destination `home` (registered, no
  role) classified the home as a special work site with full scope, so the
  commute home could be computed as a work trip.
- Evidence: local read; no input contract kept the home out of
  `destination_place_ids` and no test listed it. Verifier: survives.
- Fix: the snapshot refuses a home destination (`TRIP_TO_HOME`); a home that
  is the regular workplace is classified as one and refused by the
  computation.

### R1-F2 (must-fix): a stale place id after a place correction

- Location: `validate_place`'s prescribed correction (new place id) and
  `trip_place`.
- Claim: after an address typo was corrected by recording a new place and
  moving the entry to it, the old id still resolved, had no role, and passed
  as a special work site.
- Evidence: local read; nothing marked a place replaced. Verifier: survives,
  narrowly (needs a caller to keep using the old id).
- Fix: a place can name the place it `replaces`; a replaced place can no
  longer be named by an entry or a trip (`PLACE_REPLACED`), and "as known
  before" still sees it. The column is part of the same, unreleased v2
  migration.

### R1-F3 (must-fix by the finder): municipality compared as free text

- Location: `same_municipality` in `workplace.rs`.
- Claim: "Helsinki" and "Helsingfors" (one municipality) pass as different,
  so a same-municipality office can be recorded as a secondary workplace.
- Evidence: local read; no canonicalisation anywhere. Verifier: survives;
  fails open toward tax-free treatment, mitigated only by the user's
  confirmation.
- Not fixed here: a sound fix needs a bundled municipality classification
  (Finnish and Swedish names, yearly mergers) and a decision for places
  abroad. Filed as `claimant-municipality-codes`.

### R1-F4 (should-fix): "as known now" read with a clock behind the register

- Location: `claimant_view` (ledger) and `trip_snapshot` (domain) used the
  wall clock as the cutoff of a default query.
- Claim: a clock stepped back after a correction silently drops the
  correction from "as known now".
- Evidence: local read; `check_clock` guarded writes only. Verifier:
  survives, low likelihood.
- Fix: one `resolve_as_known_at` refuses a default query with
  `CLAIMANT_CLOCK_BEHIND`, as writes already did; an explicit instant is
  answered.

Round 1 found no defect in the migration itself: the finder's probe and the
local tests confirmed that v1 rows and table definitions survive and that the
erasure cascade removes a correction chain.

## Round 2 (integration with `domain::travel`)

### R2-F1 (must-fix by the finder): `paid` amounts on a register-refused trip

- Location: `crates/domain/src/travel/fi/trip.rs`, `payable()` still ran for
  an unclassified trip.
- Claim: a stated `paid` reached the trip's payable amounts and the totals.
- Evidence: local read confirms the mechanism. Verifier: **refuted as a
  must-fix**: every such trip carries a refusal, so the claim is not
  approvable, has no Incomes Register lines or postings, and nothing reads
  the totals; the same holds for any trip paid above its maximum.
- Tidy-up applied anyway, so the output matches "computes nothing": an
  unclassified trip has nothing payable.

### R2-F2 (must-fix): a null regular-workplace distance with a recorded regular workplace

- Location: `distance_test` treats a null `from_regular_workplace` as passing;
  the integration kept only the destination classification.
- Claim: a claimant whose register records a regular workplace could skip the
  5 km test of decision 11 § by sending null.
- Evidence: the integration's own test expected a per diem in exactly that
  case. Verifier: fix verified.
- Fix: null with a regular workplace in force is `TRAVEL_INPUT_INVALID`;
  a claimant recorded with role `none` still computes.

### R2-F3 (should-fix): secondary workplace plus an unregistered customer

- Location: `TripSnapshot::workplace` sees listed destinations only, and the
  contract does not list unregistered ones.
- Claim: such a trip is classified secondary instead of being split.
- Evidence: local read. Verifier: survives, low severity: the secondary
  result is never above the special-site result in any category, so it can
  only under-pay, and the note already says no per diem was computed.
- Fix: the secondary-workplace note tells the user to split such a trip.
  Listing every destination belongs to the draft slice
  (`expense-claim-draft`), which builds trips; recorded as a proposal there.

## Dropped

None outright; R2-F1 was downgraded by the verifier.

## Coverage

Read: the whole diff of both rounds, the stores' migration framework, the
calculator modules the integration touches, the CLI help and the guide pages.
Tested: domain unit and golden tests (including the guidance examples),
ledger store tests (migration additivity, append-only triggers, erasure
cascade), CLI integration tests (register, refusals, `claim compute` through
the register, the no-claims regression), `scripts/ci-local.sh` (stable and
MSRV, guide gate, practice month), and a copy comparison against a build of
`main` (`result.md`). Outside scope: the reconfirmation triggers, which need
trip history (`result.md`).
