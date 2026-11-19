# Review: perfectly-uppity-knife

- Skill: `/llm-review`, rigor **critical**: the change decides when an
  interrupted VAT filing may be closed as not filed, which releases the period
  for a new, irreversible FileVATReturn, and when a filing is recorded as the
  Run's own.
- Scope: commit `d878704` against `3b02c0d` (`crates/process/src/file_vat.rs`,
  `file_vat/recovery.rs`, `file_vat/status.rs`, `lib.rs` seams,
  `crates/cli/src/main.rs`, tests, guide, skill); the verifier also saw the
  first fix commit.
- Finder: `claude-fable-5-2`, one call with the diff, `recovery.rs`,
  `status.rs`, `file_vat.rs` and `crates/broker/src/outbox.rs`; it reproduced
  both its candidates with throwaway tests. Verifier: `gpt-6-astra`, one call,
  asked one focused false-negative question on the high-risk boundary: can any
  path (a) release the period while a request may have been accepted,
  (b) record a filing the authority did not accept from this Run, or (c) let
  `file-vat status` write or send. Two model calls in total.
- Risk class: authority filing state (double filing / a filing wrongly
  claimed).

## F1 (should-fix): reconcile adopted a manual filing as the interrupted Run's own

- Location: `crates/process/src/file_vat/recovery.rs`, `reconcile_file_vat_at`
  (`vat_filed` on an executing Run routed to `FiscalLanded`).
- Claim: `complete-manual` may file the period while a Run is unresolved when
  remote-status evidence is given (`file_vat.rs` `complete_manual_vat`). The
  interrupted Run then reconciled as `adopted`, its journal claiming a filing it
  never sent, with `ApprovePayment` opened.
- Evidence: finder reproduced it; local read of `complete_manual_vat` confirms
  the evidence-gated path.
- Fix: a period with manual evidence closes the Run with a `SUPERSEDED` skip
  reason and the new outcome `superseded`; `file-vat status` names
  `reconcile` for such a Run. Test
  `a_manual_filing_after_an_interruption_closes_the_run_without_claiming_it`.

## F2 (should-fix): interrupted and parked paths disagreed on "may have been accepted"

- Location: `recovery.rs`, `interrupted_dispatch`.
- Claim: only an `Unknown` outbox attempt counted as possibly arrived; the
  journalled-rejection rule (`filing_outcome_may_be_accepted`) also counts
  `MALFORMED…` and `HTTP_2…`, which older binaries used for a receiptless 2xx.
- Evidence: finder reproduced the divergence; real-world likelihood low.
- Fix: the same rule applies to a `Failed` attempt's code. Test
  `an_interrupted_request_is_judged_by_the_outbox_answer_it_left` (verified to
  fail without the fix).

## F3 (must-fix): a clock step could hide an attempt that may have arrived

- Location: `recovery.rs`, `interrupted_dispatch` (attempts filtered by
  `provenance.ts >= ` the Run's first `EffectExecuting` time).
- Claim (verifier): both timestamps are wall-clock; a backward clock
  correction between the journal append and the outbox intent drops an
  `Unknown` attempt, so one `Missing` closes the Run and a new Run may send
  again.
- Evidence: local read; the filter served only to ignore an earlier closed
  Run's attempts, which carried the same bytes.
- Fix: every attempt under the period's key counts. An earlier closed Run's
  attempt now keeps an interrupted Run on the two-answer rule, which only
  costs a wait. Test
  `an_earlier_unknown_attempt_keeps_an_interrupted_run_from_closing_on_one_answer`.

## F4 (should-fix): a Run that sent nothing adopted someone else's processed return

- Location: `recovery.rs`, `reconcile_with_authority` (`Finding::Matches` for
  `Reach::Refused`) and `status.rs` `query_interrupted_filing`.
- Claim (verifier): with no request accepted from the Run, a processed return
  with matching values is a filing made some other way, yet reconcile recorded
  it under this Run's approved hash and opened the payment gate.
- Evidence: local read; the matcher checks values and a date, not provenance.
- Fix: for an interrupted Run whose request never reached the authority, a
  matching return is recorded as an answer and reported as filed elsewhere:
  confirm in OmaVero, record with `complete-manual`, then reconcile closes the
  Run (F1). Covered in the F1 test.

## Unresolved / accepted

- Verifier (a)2: two `Missing` answers a bank day apart are a policy threshold,
  not proof. Pre-existing product decision for unknown outcomes; the new path
  inherits it on purpose. Not changed.
- Verifier (c): `file-vat status` opens the ledger (which may run schema
  migrations) and, for an interrupted Run, the outbox (`Outbox::open` creates an
  empty database when none exists). Neither records a filing, a journal event or
  an attempt; the ledger open is pre-existing behaviour of every status call.
  Not changed.
- Behaviour change noted by the finder: the parked path now compares
  `SubmittedDate` with the attempt's Finnish calendar date rather than its UTC
  date prefix. Stricter and fail-safe; matches the Finnish-date assumption in
  `status.rs`.

## Dropped

None beyond the accepted items above.

## Coverage

Read: the whole diff, `outbox.rs`, `broker.rs` submit/fetch, `complete_manual_vat`,
`authority.rs` next-command routing. Tested: `cargo test` for the workspace and
`scripts/ci-local.sh` (fmt, clippy, deny, stable + MSRV) after each fix.
Outside scope: the pre-existing parked path adopting a matching return after a
stated rejection, and real Vero visibility timing.
