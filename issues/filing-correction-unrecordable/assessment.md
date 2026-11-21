# Assessment of the filing-correction-unrecordable review

Assessed against `6157604`; the fixes are in the commit after it.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Re-recording unversioned e-service evidence rewrites an existing correction Run | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX (Rule 5: confirmed, occasional), applied |
| F2 | `run show` of an earlier correction Run still says accepted after a later correction | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | HIGH | FIX (Rule 5), applied |
| F3 | A sent replacement settled by e-service evidence is treated as known content | CONFIRMED | RARE | IMPROVES | NONE | MED | FIX (Rule 5: rare, but the fix makes the flag mean what it says, so Rule 1b does not apply), applied |
| F4 | A replacement does not check the income earner's identity code against the one first sent | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | LOW | FIX (Rule 5: occasional on thin evidence), applied |

FIX: 4   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: the external Run's id is the evidence itself, and without a version the
  newer-version check cannot tell a repeat from a new correction. A plain retry
  of the command was enough to rewrite the Run's state and double its journal.
  Fixed by refusing (`WAGE_CORRECTION_INVALID_EVIDENCE`) when the Run already
  exists. Test: `the_same_unversioned_evidence_is_not_recorded_twice`.
- F2: the same stale-receipt presentation the issue is about, one Run further
  down the chain. A correction Run now reads `superseded` unless it is the
  latest accepted filing of its report and nothing is in doubt. Test:
  `an_earlier_correction_run_is_superseded_by_a_later_one`.
- F3: e-service evidence that settles an unknown send proves a newer version,
  not its content. `report_is_current` is now false in that case, so the next
  preview warns and does not refuse the same content as unchanged. Test:
  `evidence_that_settles_a_sent_replacement_does_not_prove_its_content`.
- F4: likelihood is the weak axis. A wrong income earner is one of the
  corrections the register's guidance sends to cancel-and-new, so it is an
  ordinary reason to correct a report, but the register's schema document
  suggests it would reject such a replacement anyway. The outcome of being
  wrong is a permanent report naming another person, and the outbox already
  holds a hash that covers the identity code, so the check stores nothing new:
  the replacement is refused when the stored code differs from the accepted
  send, and the preview warns when the report was given in the e-service and
  there is nothing to compare with. Test:
  `a_replacement_cannot_name_another_income_earner`.

Simplicity: the fixes reuse what exists (the Run directory as the duplicate
guard, the report status for supersession, the outbox hash as the witness of
the identity code) and add no stored field and no new abstraction.

Nothing was staged as a follow-up issue from the review: every finding was
fixed in this change. The two items under "Unresolved uncertainty" in the
review are stated limits, not observed problems.
