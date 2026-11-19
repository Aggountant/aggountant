# Assessment of the perfectly-uppity-knife review

Assessed against the fix commits on top of `d878704`.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Reconcile adopted a manual filing as the Run's own | CONFIRMED | RARE | NEUTRAL | MINOR | HIGH | FIX — applied |
| F2 | Interrupted/parked "may have been accepted" rules differed | CONFIRMED | RARE | IMPROVES | NONE | HIGH | FIX — applied |
| F3 | Clock step could hide a possibly-arrived attempt | CONFIRMED | RARE | IMPROVES | MINOR | MEDIUM | FIX — applied |
| F4 | Run that sent nothing adopted another filing | CONFIRMED | RARE | NEUTRAL | MINOR | HIGH | FIX — applied |
| V-a2 | Two "not found" answers are a threshold, not proof | CONFIRMED | RARE | NEUTRAL | NONE | MEDIUM | DROP (product decision) |
| V-c | Status opens stores that may initialise or migrate | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | DROP (no record written) |

FIX: 4   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 2

- F1, F4 are data integrity in the sense of the journal and the payment gate
  claiming a filing the Run did not make; the period's filed state itself was
  correct in both. Fixed by never claiming a filing that is not the Run's:
  manual evidence supersedes, and a processed return without an accepted
  request of the Run is reported for `complete-manual`.
- F2, F3 guard the one irreversible direction (releasing the period while a
  request may have been accepted). Both fixes are conservative: the cost is at
  most a bank-day wait. F3 also removed the timestamp logic, so the code is
  smaller.
- V-a2 is the decided rule for unknown outcomes; the issue's direction asks the
  interrupted path to behave like `file-vat reconcile`.
- V-c: the ledger open is shared with every status call already; an empty
  outbox database is not a record. Neither changes bookkeeping.

No confirmed data-integrity finding remains open. No follow-up issue was filed.

## Simplicity

- Reused: `file-vat reconcile` as the only writer of recovery evidence (no
  resume command, as the product owner directed); the existing
  authority-check effects, not-found streak and close-as-not-filed path; the
  outbox as the record of what left the machine; `filing_outcome_may_be_accepted`
  as the one "may have been accepted" rule; `complete-manual` for a filing made
  elsewhere.
- Repetition removed: the receipt recording in `run_submit_filing` and the
  journal note are shared helpers (`record_vat_receipt`, `receipt_note`) used by
  the outbox-receipt adoption. Status and reconcile share `interrupted_dispatch`
  and `judge`, so the read-only answer and the recorded one cannot drift.
- Left out: a CLI `resume` command; writing anything from `file-vat status`;
  attributing outbox attempts to Runs (every attempt under the period's key
  counts instead); changing the pre-existing parked path that adopts a matching
  return after a stated rejection.
