# Result: expense-claim-draft

An expense claim can now be collected as a program-owned draft, one trip or
receipt at a time, and every write answers with what is still missing.

```
aggountant claim trip add    --new-for E [--claim-id ID] | --claim ID  [--trip T] < trip.json
aggountant claim trip revise --claim ID --trip T < trip.json
aggountant claim trip remove --claim ID --trip T
aggountant claim line add    --new-for E [--claim-id ID] | --claim ID  [--line L] < line.json
aggountant claim line revise --claim ID --line L < line.json
aggountant claim line remove --claim ID --line L
aggountant claim show        --claim ID [--format json|markdown]
aggountant claim list
aggountant claim discard     --claim ID --reason TEXT
```

No store migration. The fiscal store and `masterdata/pii.sqlite` are
untouched; the only new files are under `intake/expense-claims/`, created the
first time a claim is written.

## What was built

- `crates/domain/src/expense_claim.rs` (pure): the draft (`ClaimDraft`, trips
  and receipt lines with `recorded_at`, `revised_at` and `source`), its
  operations, full validation for a draft read back from disk, generated
  claim ids (`<claimant>-<first content date>`, numbered when taken) and
  `review`, which computes the trips with `travel::fi::compute_claim` and the
  claimant register and returns `open_questions`, `refusals` and `checks`
  with machine-readable codes, the subject (trip, line, day, field) and a
  message. A trip is stored in the travel computation's own input shape, so
  there is one trip schema.
  - Questions: EPA 17 § statutory fields (purpose, destination, travel mode,
    and route when there are drives), start place and destination places
    (absent key, not `null`/`[]`), free and company-paid meals per computed
    per diem day, vehicle ownership per drive, a missing receipt, the cost
    kind of a line on a secondary-workplace trip, and the computation's own
    unanswered refusals (meals, country per day, meal allowance, lodging,
    relationship, regular workplace).
  - Refusals: overlapping trips within the draft and across the claimant's
    other drafts, a receipt already attached to a sealed voucher or repeated
    on another line, a vehicle not the claimant's, a line on a trip the
    register does not make a work trip, a non-travel cost on a
    secondary-workplace trip, a receipt or evidence file missing from the
    artifact store, and every other computation refusal.
  - Checks: recorded afterwards, kilometres below the one-way distance on a
    domestic trip or above 1.5 × the return journey + 20 km, a receipt
    outside every trip, a possible duplicate (same date and amount), another
    draft that could not be read.
- `crates/domain/src/travel/fi`: `check_trip` runs a trip's own input checks
  and every drive's without the register; `compute_claim` runs it first, so a
  malformed drive is now `TRAVEL_INPUT_INVALID` also on a register-refused
  trip or a trip to the regular workplace (it used to pass unchecked there).
- `crates/process/src/expense_claim.rs`: the draft files (atomic write
  through a synced temp file and rename, the directory's own `.gitignore`
  with `*` kept last, a discard log of id, last revision hash and reason) and
  the commands: read or start, apply one operation, check newly named
  artifacts, review, write. Reading, listing and showing create nothing.
- `crates/ledger/src/lib.rs`: two read helpers, `artifact_exists` and
  `vouchers_attaching` (one pass over the sealed vouchers).
- `crates/cli/src/claim_draft.rs`: the commands, the JSON view (draft,
  review, totals, `next_step`, `written`) and the Markdown view rendered on
  request; INPUT help with examples, registered in `input_help.rs`.
- Skill `expense-claim` (Finnish, no calculation), guide page "Kokoa
  kululasku" / "Collect an expense claim", commands, status, error codes,
  company-home and backup pages, install table; CHANGELOG.
- `crates/domain/AGENTS.md`: the draft keeps the computation's trip schema;
  an absent key is a question; questions block approval, not the write.

## Acceptance criteria

- [x] A synthetic monthly claim and a synthetic single-trip claim can be
  built, shown and discarded (`claim_draft_cli.rs`: both tests).
- [x] No draft content reaches git, also in a home whose root `.gitignore`
  predates the directory (git test with a pre-directory root file; the root
  file is never rewritten; operator rules in the directory's own file are
  kept).
- [x] Hand-edited draft JSON that fails validation is refused on read
  (`CLAIM_DRAFT_INVALID` for a changed kind, an unknown field, revision 0, a
  backwards trip and broken JSON; the file is not rewritten; it can still be
  discarded).
- [x] Open questions are machine-readable codes the skill can branch on.
- [x] User guide (fi/en) and the claims skill describe collecting a claim;
  the skill keeps no calculation.
- [x] Companies without claims: `commands_unrelated_to_claims_do_not_read_the_rate_table`
  now also runs `claim list` and checks that no draft directory and no file
  appear; no other command reads the drafts.

## Decisions taken here (pre-authorized)

- **"Recorded afterwards" threshold:** a trip recorded (Finnish date) after
  the last day of the calendar month following the month it ended. Claims
  are collected monthly, so recording by the end of the next month is the
  ordinary cycle; later than that the record rests on recollection or a
  reconstruction. No source gives a number, so the rule follows the claim
  cycle instead of a day count (it gives 31–61 days, independent of month
  length).
- Command names as above; `--new-for` starts a draft so a typo in `--claim`
  cannot silently create a second one.
- Draft JSON shape: `schema_version` 1, `claim_id`, `employee_id`,
  `created_at`, `revision`, `trips[] {trip_id, recorded_at, revised_at,
  record}`, `lines[] {line_id, recorded_at, revised_at, record}`; the
  revision hash is the sha256 of the file.
- A write is refused only for input that cannot be stored or computed; a
  trip that becomes uncomputable later (a register change) shows as a
  refusal so the draft stays writable.
- The discard reason is kept in `intake/expense-claims/discarded.jsonl`
  until the ADR-5 accepted-change log exists.

## Not done here

- **Reconfirmation triggers** from the register slice (trip pattern
  contradicting the entry, new recurring destination, return after a
  six-month gap): they need trip history that survives the draft (sealed
  claims) and a way to record a reconfirmation in the register. Proposed as
  a spin-off after `expense-claim-approve-seal`.
- **Claims in force** in the overlap check: none exist until approval is
  built; the check covers open drafts.
- **A corrupt but present receipt file** passes `artifact_exists`; the
  approval slice reads receipt bytes and should do so through the store's
  verified read.
- **Municipality identity** is left to `claimant-municipality-codes`, as the
  brief says.

## Review

`review.md` and `assessment.md`: three findings, all confirmed and fixed.
