# Review: tax-payment-references

- Skill: `/llm-review`, rigor **routine** with a high-risk surface: the change
  decides the account and reference a VAT payment proposal tells the user to
  pay with. A wrong reference allocates the payment to another debt; a broken
  read of stored Runs would change what earlier approvals bound.
- Scope: the first commit of this slice ("tax-payments: typed Tax Administration payment details …") against its parent (`crates/model/src/payment_reference.rs`,
  `crates/process/src/tax_payments.rs`, `crates/process/src/file_vat.rs`,
  `crates/cli/src/tax_payments.rs`, `crates/cli/src/main.rs`, tests, guide,
  skill).
- Finder: `gpt-6-astra`, one call on the diff with the issue's contract.
  Verifier: `claude-fable-5-2`, one focused call on the stored-Run boundary
  (old Runs read unchanged; the arrangement reference never reaches VAT; what
  is shown equals what is bound). Two model calls in total.

## F1 (should-fix): payee IBAN accepted letters after `FI`

- Location: `crates/process/src/tax_payments.rs`, `validate_payee_iban`.
- Claim: the check required `FI`, 18 characters and the mod-97 checksum, but
  `Iban::new` allows letters in the BBAN, so a mistyped Finnish account such
  as `FI211234A600000785` passed.
- Evidence: local, the mod-97 remainder of `FI211234A600000785` is 1 (computed
  in the session); two such one-letter variants of the fixture account exist.
- Fix: require digits after `FI`; regression in `payee_iban_must_be_a_finnish_iban`.

## F2 (should-fix): a Run filed with a reference corrected after its start

- Location: `crates/process/src/file_vat.rs`, `answer_file_vat_seam`
  (ApproveFiling approve).
- Claim: `file-vat start` snapshots the account and reference into the
  payment proposal; if the user corrects them with `tax-payments set` before
  approving, the filing went ahead and `run show` kept telling the user to pay
  with the superseded reference, while the filed period's preview names none.
- Evidence: verifier (cited the start snapshot, the approve path that checks
  only the filing profile, and the post-filing message); confirmed locally.
- Fix: `refuse_superseded_payment_details` refuses the approval of a payable
  return whose bound account or reference differs from the current details
  (`TAX_PAYMENT_DETAILS_CHANGED`); nothing is sent and the Run can be
  rejected. Runs without a bound reference (earlier builds) are not checked.
  To make the master data the single source, the reference is now resolved
  inside the engine (`start_file_vat`, `start_file_vat_replacement`), not
  assembled by the CLI. Test
  `a_payment_detail_corrected_after_the_start_refuses_the_filing_approval`.

## F3 (should-fix, low likelihood): duplicate guard missed leading-zero spellings

- Location: `crates/model/src/payment_reference.rs`, `validate_domestic`;
  `TaxPaymentDetails::validate` compares references as text.
- Claim: leading zeros do not change the 7-3-1 check digit, so
  `012345672` and `12345672` were both valid and compared unequal.
- Evidence: verifier, by reading; confirmed locally.
- Fix: a domestic reference with a leading zero is refused (Finanssiala:
  "Etunollia ei tulosteta"), so one reference has one spelling.

## Uncertain

- None.

## Dropped (2)

- RF and domestic spellings of the same digits compare unequal: the user would
  have to type the same reference in two forms into two fields; no source we
  use gives both. Not fixed.
- `require_payment_route` still says the reference is shown in OmaVero: the
  path is reached only by a Run an earlier build left at `ApprovePayment`,
  which carries no reference. Wording only.

## Coverage

Read: the whole diff, `FileVatState` persistence and every `payment_body`
reader (`rebuild_payment_env` at answer, retry and resume;
`file_vat_payments`; `user_payment_reason`), the CLI input assembly. Tested:
the full workspace through `scripts/ci-local.sh`. Not covered: a bank payment
adapter actually consuming the new `reference` key (none exists; the direct
Vero route never pays).
