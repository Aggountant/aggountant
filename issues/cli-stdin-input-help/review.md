# Review: cli-stdin-input-help

- Skill: `/llm-review`, rigor **routine**: help text and tests only, no change
  to what any command accepts or writes. The risk is a help text that
  misdescribes a stdin shape and so leads an agent into wrong bookkeeping.
- Scope: commit `3501adb` against its parent (`crates/cli/src/input_help.rs`,
  `main.rs`, `knowledge.rs`, `wage.rs`, the two tests, the commands reference
  page of the guide).
- Finder: `gpt-6-astra`, one call with the diff and read access to the
  repository, asked to check each INPUT text against the Rust types and
  handlers it describes. Verifier: none; every surviving candidate was
  decided by local evidence (a reproduced run or the exact handler code), which
  a second model could not overturn. One model call in total.
- Risk class: documentation of bookkeeping input (no write path changed).

## F1 (must-fix): the capture-receipt start example sealed VAT the return leaves out

- Location: `crates/cli/src/input_help.rs`, `CAPTURE_RECEIPT_START` example.
- Claim: the example draft had no `vat_code`. Capture's pre-seal check sums
  input-VAT account legs for an untagged draft, so the Run seals it with
  EUR 25.50 assessed, but filing counts only tagged postings or generated
  expansions (`crates/domain/src/vat/roles.rs`,
  `derive_filing_figures_with_role_table`), so the deduction would be missing
  from the return.
- Evidence: local read of `draft_deductible_vat_with_role_table` (legacy branch
  for untagged drafts) and the filing projection; the shipped skill already
  tells agents to tag the base posting.
- Fix: the example tags the expense posting `purchase-domestic-standard`, the
  field text says to tag the VAT base and leave the VAT leg untagged. New test
  `capture_receipt_start_example_reaches_the_vat_return` runs the printed
  example through the three gates, closes the period and asserts
  `file-vat preview` shows purchase VAT 25.50, reconciled.

## F2 (must-fix): the employee help promised an encrypted store

- Location: `EMPLOYEE_ADD`, and the existing `employee` / `employee add`
  descriptions in `crates/cli/src/main.rs`.
- Claim: the store uses `IdentityCipher`, which stores plaintext
  (`crates/ledger/src/store/cipher.rs`).
- Evidence: local read; the open bug `employee-store-plaintext-help` reports
  the same, and the user guide already said the store is not encrypted.
- Fix: the help states the separate gitignored PII store and that it is not
  encrypted at rest; `employee add` names `employee_id` (the other half of
  that bug). The guide's "the help says encrypted" sentences are updated.

## F3 (should-fix): import help promised unknown fields are refused inside vat_treatment

- Location: `IMPORT_CAMT053_SEAL`.
- Claim: `VatTreatment` lacks `deny_unknown_fields`, so a `deductibility`
  member on `domestic-input` is dropped and full deduction is sealed.
- Evidence: reproduced: deserializing that object gives
  `Ok(DomesticInput { rate_class: "domestic-standard" })`.
- Fix here: the help says unknown fields are refused at the top level and in
  each assignment, and warns that a partial deduction needs the `-partial`
  type because the full types ignore `deductibility`. The code fix is in
  `crates/domain`, outside this slice's file set: filed as
  `vat-treatment-deny-unknown-fields`.

## F4 (should-fix): wrong error code for a mismatching acknowledgement

- Location: `FILE_VAT_START`, `vat_reconciliation_acknowledgement`.
- Claim: a wrong voucher set, an empty reason or an unneeded acknowledgement
  is `VAT_RECONCILIATION_ACK_MISMATCH`; `VAT_ACCOUNTS_UNRECONCILED` is only the
  missing acknowledgement.
- Evidence: `require_reconciled` in `crates/process/src/vat_reconciliation.rs`.
- Fix: the text names both codes and when each occurs.

## Dropped

None. All four candidates survived local checks.

## Coverage

Read: every stdin reader in `crates/cli/src` (`read_stdin_json` and
`read_stdin_text` callers) and the types each parses into. Tested: the unit
tests parse every example into its handler's type; the integration test runs
the offline examples end to end. Not run end to end: `close-year start`,
`file-vat start`, `hunt-receipts start` and `import camt053 seal` examples
(they need a sealed year, Vero readiness, a broker script or a statement
file); their examples are checked by parsing only.
