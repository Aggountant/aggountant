# Assessment of the claimant-workplace-register review

Assessed against `882d77d` (all review fixes are in the branch).

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| R1-F1 | Home as a destination passed as a work trip | CONFIRMED | OCCASIONAL | IMPROVES | MINOR | HIGH | FIX — applied |
| R1-F2 | Stale place id after a place correction | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | MED | FIX — applied |
| R1-F3 | Municipality compared as free text | CONFIRMED | RARE | NEUTRAL | MODERATE | HIGH | SPIN_OFF — `claimant-municipality-codes` |
| R1-F4 | "As known now" with a clock behind the register | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX — applied |
| R2-F1 | `paid` on a register-refused trip reaches totals | CONFIRMED (mechanism), not a defect | RARE | IMPROVES | NONE | HIGH | tidy-up applied |
| R2-F2 | Null regular-workplace distance with a recorded regular workplace | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | HIGH | FIX — applied |
| R2-F3 | Secondary workplace plus an unregistered customer | CONFIRMED | OCCASIONAL | NEUTRAL | MODERATE | MED | DISCUSS — note applied; destination listing proposed for `expense-claim-draft` |

FIX: 4   FIX (with care): 0   SPIN-OFF: 1   DISCUSS: 1   DROP: 0 (R2-F1 kept as a tidy-up)

- R1-F1, R2-F2: data integrity. Both would let a trip earn tax-free
  allowances it is not entitled to; fixed with tests.
- R1-F2: needs a caller to keep using a replaced id; the fix is small and
  lives inside the unreleased v2 migration, so it costs no second migration.
- R1-F3: real and fails open toward tax-free treatment, but a sound fix needs
  bundled reference data with a yearly update and a design for places abroad.
  The user's confirmation is the safeguard until then. Filed unlaned for
  triage.
- R1-F4: a clock fault is rare; the fix reuses the write-side check.
- R2-F1: refuted as a defect by the verifier (no approvable claim, lines or
  postings can carry the amount); the tidy-up makes the output match the
  documented "computes nothing".
- R2-F3: errs only toward paying less, and the user is told; the complete
  answer is how the draft slice lists a trip's destinations.

No confirmed data-integrity finding remains open.

## Simplicity

- Reused: the ledger's PII store and its migration framework (one appended
  migration, no new store), `EmployeeState`'s register as the claimant
  identity (facts cascade away with the person), `model::Date`/`Instant`
  parsing, the CLI's `read_stdin_json`/`CliError` plumbing, and the
  calculator's own refusal mechanism for register refusals.
- Repetition removed: the integration deleted the register's own
  `trip_scope`/`TripScope` and `TRIP_TO_REGULAR_WORKPLACE`; the classification
  enum is defined once (`domain::workplace::Workplace`) and re-exported by
  `travel::fi`, and the regular-workplace refusal is the calculator's
  `TRAVEL_TO_REGULAR_WORKPLACE`. The bitemporal fold (`known_at`), conflict
  check and digest are written once for all three fact kinds instead of once
  per kind. One generic `claimant add`/`correct` replaces per-kind commands,
  because the JSON already carries the kind.
- Remaining repetition, justified: the basis-code list appears in the Rust
  enum and in the `claimant add` help text; a test fails when a code is
  missing from the help. SQL holds structural CHECKs only, not code lists, so
  adding a code later needs no table rebuild.
- Left out on purpose: the reconfirmation triggers (they need trip history the
  draft slice creates), municipality codes (above), encryption (the store's
  existing gap, `employee-store-plaintext-help`), and a place-correction verb
  (a new place naming `replaces` plus an entry correction is enough).
