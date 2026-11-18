# Assessment of the vat-filing-resent-by-new-run-after-unknown review

| # | Finding | Classification | Likelihood | Outcome | Notes |
|---|---|---|---|---|---|
| A1 | An interrupted filing blocks new Runs but `reconcile` cannot resolve it | CONFIRMED | RARE (a crash inside the dispatch window) | SPIN_OFF, already filed | The recovery is `perfectly-uppity-knife`, the next issue in the lane; its acceptance requires a new Run to stay refused while the check fails. Exit until then: `complete-manual` with remote-status evidence. Nothing new to file |
| V1 | Two "not found" answers across midnight close the Run minutes after sending | CONFIRMED | OCCASIONAL (an agent retrying reconcile around midnight) | FIX, applied | `bank_days::one_bank_day_after` gives the same Finnish clock time on the next bank day, never before that day starts; the streak compares instants. `next_check_from` is now that instant. Unit tests for midnight, weekend, Easter, Midsummer, New Year and the October switch; the integration test shows 09:59 Monday is too soon after 10:00 Friday and 10:00 counts. The date-only predicate was removed |
| V2 | A GetFiledVATReturn answer without `Status` can adopt a filing | CONFIRMED (code); authority behaviour unverifiable | RARE | FIX, applied | Data integrity (a period marked filed without proof), and the fix is one condition: only `Status == "Processed"` can match. Regression assertion in the broker unit test; all other tests unchanged |

No confirmed data-integrity finding remains open. After the fixes no path sends
a second FileVATReturn while an unknown is unresolved, and a Run is closed only
after two "not found" answers a full bank day apart.

## Simplicity

- Reused: the `filing_outcome_may_be_accepted` predicate from
  `staggeringly-aspiring-sky`, the existing journal events (an authority check
  is an `Observe(GetFiledVATReturn)` effect planned and succeeded; closing skips
  the filing and payment effects), `file-vat reconcile` as the one command, and
  the `Clock` trait the engine already injects for durable waits. No new journal
  event type, Run status or command.
- Repetition removed: `file-vat status`, `complete-manual` and the dispatch guard
  had separate copies of "is a filing unresolved" (`status.rs` and the removed
  `period_has_unresolved_unknown_filing`); all three now call
  `unresolved_filing`.
- Left out on purpose: recovery of an interrupted Run (next issue), a separate
  command for "record a not-found answer" (reconcile already asks), a
  per-Run outbox key that would let a different request be sent after a closed
  attempt (it would weaken the broker's same-bytes rule), and a general
  business-calendar abstraction (one module of free functions; only Finnish bank
  days are needed).
