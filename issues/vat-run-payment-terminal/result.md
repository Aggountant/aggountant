# Result: vat-run-payment-terminal

The defect reproduced on `main` at `f5c72a9`: the direct Vero.fi broker
(`build_direct_vero_broker_at`) registers no `pankki-maksu` adapter, and an
accepted payable filing still opened `ApprovePayment`. Approving it would
also have journalled the approval and `EffectExecuting` before the broker
refused the unknown service, leaving the Run `running` instead.

## What changed

- **New Runs finish.** After an accepted filing, a payable return on a broker
  without a bank payment submit adapter (`Broker::can_submit`) skips the
  `submit-payment` effect instead of opening `ApprovePayment`, so the Run is
  `completed`. The journal reason says the payment is left to the user and
  that aggountant did not pay it.
- **`run show` says what to pay.** The `submit-payment` effect carries a
  `payment` object: `state` (`left_to_user`, or `awaiting_approval` for a Run
  waiting at the gate), `amount`, `currency`, `payee_iban`, `due_date` (from
  GetVATPeriods at start), `message`, and for a waiting Run `next_command`.
  Nothing says the tax is paid; the payment reference is not known to
  aggountant and the message points to OmaVero for it.
- **Parked Runs have a supported way out:** `aggountant file-vat user-pays
  --run-id <id>`. It answers the open `ApprovePayment` gate `user-pays`, skips
  the payment effect and returns the completed Run. It needs no credentials
  or network. It is refused before the filing is accepted, after the payment
  completed, after a rejected payment gate, and whenever a bank payment may
  have been sent (an outbox entry, or a payment rejection journalled after the
  broker returned). On a zero-payment Run it writes the ordinary
  "no bank payment required" record instead. Repeating it changes nothing.
- **Approval that cannot work is refused first.** `approve-payment`, the
  payment retry and the payment resume refuse with
  `VAT_PAYMENT_ROUTE_UNAVAILABLE` on a route without a payment adapter,
  before anything is journalled.
- Unchanged: refund and zero-payment returns, replacement returns, and routes
  with a payment adapter (today only the `AGGOUNTANT_BROKER_SCRIPT` test
  seam). No store migration: existing journal events only, and
  `FILE_VAT_WORKFLOW` is unchanged so parked Runs keep passing the version
  check. Sealed vouchers, receipts and filings are not touched.

## Tests

`crates/process/tests/file_vat.rs`:
`a_payable_direct_route_filing_completes_with_the_payment_left_to_the_user`,
`a_refund_on_the_direct_route_still_completes_as_no_payment_required`,
`a_run_parked_at_approve_payment_is_completed_with_user_pays` (the parked
production state; also checks the filings and their receipts are unchanged and
the chain verifies), `user_pays_finishes_a_payment_approved_but_never_sent`,
`user_pays_is_refused_once_a_bank_payment_was_sent`,
`user_pays_trusts_the_journal_when_the_outbox_is_not_at_hand`,
`user_pays_on_a_refund_records_no_payment_required_instead`,
`user_pays_is_refused_before_the_filing_is_accepted`.
`crates/cli/tests/file_vat_cli.rs`:
`user_pays_completes_a_run_waiting_at_approve_payment_without_a_broker`.
`scripts/ci-local.sh` passed (fmt, clippy, deny, stable and MSRV build and
test, user-guide gate).

## Docs

`skills/vero-fi/SKILL.md`, the direct filing task page, troubleshooting,
"What is ready" and the error-code table (fi and en), and `--help` for
`approve-payment` and `user-pays`.

Review and assessment: `review.md`, `assessment.md` beside this file.
