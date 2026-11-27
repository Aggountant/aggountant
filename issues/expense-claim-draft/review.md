# Review: expense-claim-draft

- Skill: `/llm-review`, rigor **routine**: the slice writes no fiscal state,
  files nothing and migrates no store; it adds intake files outside git and
  decides which open questions block a claim, where a missed question would
  later make an allowance too large.
- Scope: the branch diff against `a642c1a` (`b3f2595`, `13d0444`): the draft
  model and review (`crates/domain/src/expense_claim.rs`), the draft files
  and command orchestration (`crates/process/src/expense_claim.rs`), the CLI
  and Markdown view (`crates/cli/src/claim_draft.rs`) and two ledger read
  helpers (`artifact_exists`, `vouchers_attaching`).
- Finder: `gpt-6-astra`, one call with the diff, the four source files, the
  travel computation's input types and the issue. Verifier:
  `claude-fable-5-2`, one call with the current files and the fix diff,
  asked to refute each candidate and to test whether the fix closes it. Two
  model calls.
- Risk class: silently wrong allowances (an unanswered fact defaulting to an
  answer), personal data reaching git.

## F1 (must-fix): a missing destination list became "special work site"

- Location: `crates/domain/src/expense_claim.rs`, `review` (the start-place
  question had no destination counterpart); `TripPlaces.destination_place_ids`
  is `#[serde(default)]` in `crates/domain/src/travel/fi/mod.rs`, and
  `TripSnapshot::workplace` classifies an empty list as a special work site.
- Claim: a trip to a registered secondary workplace whose record left
  `trip.places.destination_place_ids` out was computed with per diems that
  the secondary workplace would have excluded.
- Evidence: local read, no guard on the key's absence anywhere in the draft
  path. Verifier: survives.
- Fix: open question `TRIP_DESTINATION_PLACES_UNANSWERED` when the key is
  absent; `[]` stays a valid answer. The `crates/domain/AGENTS.md` rule on
  defaulted fields names the key (verifier residual).

## F2 (should-fix): a stored trip's own input was not validated on read

- Location: `crates/domain/src/expense_claim.rs`, `trip_input` (deserialize
  only); `crates/domain/src/travel/fi/trip.rs`, drive validation inside the
  classified-workplace branch only.
- Claim: a hand-edited draft with a trip ending before it starts passed
  `ClaimDraft::validate` and showed only as a review refusal; a drive outside
  its trip on a register-refused trip or a trip to the regular workplace was
  never checked, by the draft or by `claim compute`.
- Evidence: local read and a test. Verifier: survives, narrower than worded
  (a backwards trip was already refused at the CLI write, and no affected
  draft could become ready; the defect is the storage and read invariant).
- Fix: `travel::fi::check_trip` runs the trip's and every drive's own checks
  without the register; the draft calls it on every write and read, and
  `trip::compute` calls it first, so the drive rule lives in one place.
  Error messages keep the drive number (verifier residual).

## F3 (must-fix): a receipt missing from the artifact store could pass

- Location: `crates/process/src/expense_claim.rs`, `show` → `review_draft`
  (artifact existence was checked only on writes).
- Claim: a draft whose receipt blob is gone (a home restored without
  `artifacts/`, or a hand edit to another valid hash) reported
  `ready_for_approval: true`.
- Evidence: local read. Verifier: survives.
- Fix: the review refuses each missing receipt or evidence file
  (`ARTIFACT_MISSING`); a write checks only the files it newly names, so one
  lost file does not block unrelated writes. CLI test with `artifacts/`
  emptied.

## Unresolved, outside the findings

- A blob that is present but truncated or undecryptable passes
  `artifact_exists`, which checks the path only. The approval slice binds the
  receipt bytes and should read them through the store's verified `get`.
- A reflexive `[]` for destinations, or a list that omits the secondary
  workplace, still classifies as a special work site; no presence check can
  catch a wrong answer. The skill tells the agent to ask.

## Dropped

None of the finder's three candidates was dropped.

## Coverage

Read: the four changed source files, the travel computation's input types,
`trip.rs`, `mileage.rs`, `workplace.rs` (snapshot), the CLI tests. Run: the
domain, process and CLI suites (`scripts/ci-local.sh`). Not reviewed by a
model: the user-guide pages and the skill text (focused self-review against
`--help` and the code).
