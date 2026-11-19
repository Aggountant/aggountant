# Result: filing-receipt-in-output

Authority filing effects now carry the authority's answer in the Run view, so
an agent reads the receipt from the product's own output instead of the
journal files:

```json
{"key":"submit-filing","effect_type":"SubmitFiling","status":"succeeded",
 "outcome":{"state":"accepted","receipt":{"authority":"vero",
   "unique_identifier":"…","accepted_timestamp":"…"}}}
```

## What was built

- `crates/process/src/authority.rs`: `AuthorityOutcome` (`accepted` with an
  optional `receipt`, `outcome_unknown` with `next_command` and `message`,
  `rejected` with `error_code`) and `AuthorityReceipt` (`vero`:
  `unique_identifier`, `accepted_timestamp`; `tulorekisteri`:
  `ir_delivery_id` = IRDeliveryId, `ir_report_id` = IRItemId, `version`,
  `source` `api`/`e_service`). Computed for `file-vat` submit-filing,
  `file-wage` submit-wage and `file-employer-report` effects; every other
  effect has no `outcome` field, so existing output is unchanged apart from
  the added field.
- VAT receipts come from the Ledger's stored receipt, and only for the Run
  whose approved request it acknowledges (`vat_receipt_of_run`, reusing
  `file-vat status`'s attribution rule). A filing adopted by `reconcile` has
  no receipt; `next_command` then points at `file-vat status`.
- Incomes Register receipts are parsed from the effect's journal note with the
  existing `EmployerReportReceipt` parser (a travel-expense manual completion
  has no version).
- Unknown outcomes name the command that settles them:
  `file-vat reconcile --run-id …`, `file-wage reconcile …`,
  `file-wage employer-report status --month …`; an interrupted VAT dispatch
  points at `file-vat status --period …`.
- `ProcessEngine::show` adds the outcomes; `file-vat approve-filing`,
  `approve-payment`, `retry`, `reconcile` and resume return through it, and
  the `file-wage` commands already did. `run show --run-id wr-…` is the
  offline, credential-free read of a travel-expense report's receipt (no new
  command).
- `--help` (`run show`, `file-vat approve-filing`, `file-wage approve`,
  `file-wage reconcile`), `skills/vero-fi` (where the receipt is read from,
  and a fix to its `run show` example, which lacked `--run-id`) and the
  user guide's VAT and travel-allowance pages (fi, en).

## Verification

- Tests: `receipt_is_in_approve_output_and_run_show` (wage: unknown → reconcile
  → accepted, manual), `happy_path_files_and_pays` and
  `run_with_unknown_outcome` (VAT accepted / unknown),
  `a_found_return_is_adopted_and_nothing_is_sent` (no receipt),
  `a_receipt_is_not_attributed_to_a_run_whose_request_it_does_not_acknowledge`,
  the employer-report happy path, the CLI `run show` of a wage run, and a unit
  test of the note parser.
- `scripts/ci-local.sh` passed (fmt, clippy, cargo-deny, stable and MSRV).
- Review and assessment: `review.md`, `assessment.md` (one finding fixed, one
  dropped).
