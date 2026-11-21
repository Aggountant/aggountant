# Result: vat-reconciliation-before-close

`file-vat preview` now runs on an open or closing period, so the VAT
reconciliation reaches the agent while an untagged VAT posting can still be
corrected in the same period.

```
$ aggountant --json file-vat preview --period 2026-07     # period open
{"period":"2026-07","period_state":"open","fileable":false,
 "attention":[{"code":"VAT_PERIOD_NOT_SEALED","severity":"blocking",…},
              {"code":"VAT_ACCOUNTS_UNRECONCILED","severity":"blocking",…}],
 "vat_reconciliation":{"status":"unreconciled",…}, …}
```

## What changed

- `crates/process/src/file_vat.rs`: the preview derives from the period's
  sealed vouchers in any state. New output fields `period_state`
  (`open|closing|sealed`) and `fileable` (true only when sealed); existing
  fields are unchanged. For open and closing periods the `filing_instruction`
  and `payment_instruction` say the figures are a draft and what to do next
  (closing: reject the close before a correction, or approve it). The sealed
  check moved into a thin `derive_verified_sealed_filing` wrapper, which
  `file-vat start` and the Run's re-derivation still use, so filing a
  non-sealed period is refused exactly as before (`PERIOD_NOT_SEALED`).
- `crates/process/src/vat_reconciliation.rs`: `attention_items` takes the
  period state. A non-sealed period gets `VAT_PERIOD_NOT_SEALED` (blocking:
  the direct filing is stopped), and `VAT_ACCOUNTS_UNRECONCILED` names the
  remedy that fits the state: a correcting voucher in the period (open),
  rejecting the close first (closing), or OmaVero with corrected figures
  (sealed, unchanged).
- CLI: `file-vat preview --help` describes the draft; the human line says
  `(draft, not fileable)`.
- Skills: `capture-receipt` replaces the last paragraph of "Erikoiskirjaukset
  ja niiden ALV-merkintä" with a pointer to the new ALV check in "Täsmäytys",
  and the close proposal includes the reconciliation status; `vero-fi` notes
  that an open period's preview is a draft that is not sent.
- User guide (fi, en): close-month gains the VAT check as a precondition and
  in the agent's commands; error codes gain `VAT_PERIOD_NOT_SEALED` and the
  before/after-close remedy for `VAT_ACCOUNTS_UNRECONCILED`; status,
  record-other-entries and file-vat-omavero updated.

## Tests

`crates/process/tests/vat_reconciliation.rs`: an open period previews as a
read-only draft that finds the untagged VAT; a correction before the close
reconciles the draft and the sealed preview has the same figures; filing an
open period is still refused. `crates/cli/tests/vat_reconciliation_cli.rs`
walks the issue's case through open, closing and sealed. `scripts/ci-local.sh`
passes.

Review and assessment: `review.md`, `assessment.md` beside this file.
The month-close gate content itself remains `virtually-tidy-owl`.
