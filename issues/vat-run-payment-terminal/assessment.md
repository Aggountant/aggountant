# Assessment of the vat-run-payment-terminal review

Assessed against the final working tree (the review fixes are in the same
commit as the change).

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Lost outbox let `user-pays` skip a sent payment | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX (data integrity, cheap) — applied |
| F2 | `user-pays` on a zero-payment Run recorded "left to the user" | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX (untruthful record, cheap) — applied |
| U1 | `next_command` offered on a route that could pay | CONFIRMED | RARE | NEUTRAL | NONE | MEDIUM | DROP (message is true; test seam only) |
| U2 | `Executing` payment with an outbox row has no CLI way out | CONFIRMED | RARE | NEUTRAL | MINOR | MEDIUM | DROP (pre-existing; needs a payment adapter no shipped route has) |

FIX: 2   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 2

- F1 and F2 are both a wrong record, which this project never filters out,
  and each fix is a few lines in the one function. Both are tested.
- U1: the command is a legitimate choice on any route (the user may pay
  themselves); the message states the condition under which approval cannot
  work. No shipped route has a payment adapter.
- U2: the dead end exists without this change and only behind a payment
  adapter. If a bank adapter is ever built, its resume path is part of that
  work. No issue filed: no observed occurrence and no shipped route reaches it.

## Simplicity

- Reused: the existing `submit-payment` effect and `EffectSkipped` event (no
  new journal event, no store migration, `FILE_VAT_WORKFLOW` unchanged so
  parked Runs keep passing the version check), the `ApprovePayment` gate's
  answer for parked Runs, the run's own `file_vat_state.json` for the amount,
  account and due date, and the outbox for "was a payment sent".
- One rule, one place: "can this route pay" is `Broker::can_submit`, asked by
  `finish_filing_postcondition` and `require_payment_route`; the user-pays
  reason text is one function used by both the automatic skip and the
  command.
- Left out: tracking whether the tax is paid (tax-account work,
  `tax-account-rollforward`), an evidence-of-payment command, and any bank
  payment initiation.
