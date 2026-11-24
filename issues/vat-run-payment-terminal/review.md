# Review: vat-run-payment-terminal

- Skill: `/llm-review`, rigor **routine** with a high-risk check: the change
  decides when a VAT Run is finished and what it says about the tax payment.
  A wrong answer either claims a payment that was never made or skips one a
  bank may have carried out.
- Scope: the working-tree diff against `f5c72a9` (`crates/process/src/file_vat.rs`,
  `file_vat/recovery.rs`, `lib.rs`, `error.rs`, `crates/broker/src/broker.rs`
  `Broker::can_submit`, `crates/cli/src/main.rs` `file-vat user-pays`, tests,
  guide, skill).
- Finder: `gpt-6-astra`, one call (diff, `file_vat.rs`, `lib.rs`,
  `recovery.rs`, the issue, `crates/AGENTS.md`). Answer: "No findings."
- Verifier: `claude-fable-5-2`, one focused false-negative call on the
  high-risk boundary: can a Run end completed with the payment skipped
  although a bank payment may have been sent, can a route with a payment
  adapter behave differently, can the status become untruthful. Two model
  calls in total.
- Risk class: payment and filing state of a production bookkeeping Run.

## F1 (should-fix): a lost outbox let `user-pays` skip a payment that was sent

- Location: `crates/process/src/file_vat.rs`, `leave_file_vat_payment_to_user`
  (the outbox check).
- Claim: the only evidence of an earlier bank payment attempt was the outbox
  entry, and `integration/outbox/outbox.db*` is gitignored. A home used
  without that file (another checkout, a lost file) after an
  `OUTCOME_UNKNOWN` payment would let `user-pays` record "aggountant did not
  pay it" over a payment that may have gone through.
- Evidence: local read; `Outbox::open` writes `/outbox.db*` into the
  directory's `.gitignore`. Every payment rejection except the local
  `PAYMENT_EXCEEDS_RECONCILED_AMOUNT` check is journalled after
  `Broker::submit` returned, so the journal already proves the attempt.
- Impact: needs a broker with `pankki-maksu`; in the shipped CLI only the
  test seam registers one, so no real bank payment is affected today.
- Fix: refuse also when the payment effect is `Rejected` with any code other
  than `PAYMENT_EXCEEDS_RECONCILED_AMOUNT`. Test
  `user_pays_trusts_the_journal_when_the_outbox_is_not_at_hand` deletes the
  outbox and expects the refusal.

## F2 (should-fix): `user-pays` on a zero-payment Run recorded "left to the user"

- Location: same function.
- Claim: a refund or replacement Run that crashed between the filing's
  success and the no-payment record is `running` with the payment `Planned`
  and no gate. `user-pays` is the only CLI command that moves it, and it
  recorded "payment left to the user: 0.00 EUR" instead of the
  "no bank payment required" record.
- Evidence: local read of `finish_filing_postcondition` and the crash seam
  `CrashAfterFilingSuccess`; there is no CLI caller of `resume_file_vat`.
- Fix: on a zero amount, `user-pays` converges the ordinary no-payment record
  (`finish_filing_postcondition`). Test
  `user_pays_on_a_refund_records_no_payment_required_instead`.

## Uncertain / not fixed

- The `payment` view of a Run waiting at `ApprovePayment` offers
  `next_command: file-vat user-pays` also on a route that could pay. Its
  message says approval needs a bank payment adapter and that the command is
  for a user who pays themselves, which is true on any route; only the test
  seam has such a route today. Left as is.
- A payment effect left `Executing` after the outbox recorded an attempt (a
  crash inside `Broker::submit`) has no CLI command that moves it: `retry`
  wants a parked effect and `resume` has no CLI. This predates the change and
  needs a payment adapter; `user-pays` refuses it and its message no longer
  promises `retry` will work.

## Dropped candidates

None from the finder. The verifier's own answer found no sequence for
"completed while the filing is not accepted" and confirmed that adapter
routes keep gate opening, approval, retry and resume unchanged.

## Coverage

Read: the whole diff, `file_vat.rs`, `lib.rs` (fold and status), `recovery.rs`
callers of the postcondition, `Broker::submit` and the outbox. Tested:
`cargo test --workspace` and `scripts/ci-local.sh` (fmt, clippy, deny,
stable and MSRV build and test, user-guide gate). Outside scope: the live Vero
adapters, which the change does not touch.
