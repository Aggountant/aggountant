# Assessment of the expense-claim-draft review

Assessed against the branch with the review fixes applied.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Missing destination list became "special work site" | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX — applied |
| F2 | A stored trip's own input not validated on read; drives unchecked on unclassified trips | CONFIRMED | RARE | IMPROVES | MINOR | HIGH | FIX — applied |
| F3 | Receipt missing from the artifact store could pass | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX — applied |

FIX: 3   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: data integrity. An agent that forgets one key would make a secondary
  workplace trip earn per diems; the same "absent is unanswered" check the
  draft already used for the start place closes it.
- F2: no wrong amount was reachable (the trip always carried a refusal), but
  the acceptance criterion says a draft that fails validation is refused on
  read. The fix also moves drive validation ahead of the register's
  classification in the calculator, so `claim compute` now rejects a
  malformed drive on every trip; noted in the CHANGELOG.
- F3: needs a broken restore or a hand edit; the fix is a refusal in the
  review plus narrowing the write-time check to newly named files.
- The verifier's residual about corrupt-but-present blobs belongs to the
  approval slice, which reads receipt bytes; it is recorded in `result.md`
  for `expense-claim-approve-seal`, not filed.

No confirmed data-integrity finding remains open.

## Simplicity

- Reused: the travel computation's input types as the draft's trip schema
  (the draft stores the trip JSON and validates it by deserializing into
  `TripInput`, so there is one trip schema), `compute_claim` for every
  amount and register question, the calculator's refusal codes as findings,
  `trip_snapshot` through the computation for the receipt-line rule, the
  artifact store and `artifact attach` for receipts, the ledger's voucher
  snapshot for "already booked", `CliError`/`parse_stdin_json` plumbing.
- Repetition removed: drive validation ran inside `mileage::compute`; it now
  runs once in `trip::check`, used by the computation and the draft alike.
- Remaining repetition, justified: the draft's amount parser is a few lines
  like the calculator's private one, but for a different field (a receipt
  amount, with an upper bound); sharing would make the calculator's parser
  public for one caller.
- Left out on purpose: the ADR-5 accepted-change log (the issue says the
  draft inherits it when it is built; the discard log keeps only id, last
  revision hash and reason), approval and sealing, a "claims in force"
  overlap source (no sealed claims exist yet), the register's reconfirmation
  triggers (need sealed-claim history and a reconfirmation record; proposed
  as a spin-off), a typed list of every destination, and any store
  migration.
