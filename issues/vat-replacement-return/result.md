# Result: vat-replacement-return

A filed VAT return is now corrected through aggountant:

```
aggountant voucher seal            # draft carries "corrects_vat_period": "2026-10"
aggountant file-vat preview --period 2026-10
aggountant file-vat replace < replacement.json
aggountant file-vat approve-filing --run-id fv-…
aggountant file-vat complete-manual --period 2026-10 --channel omavero \
    --evidence-ref … --replacement
```

Existing inputs and outputs are unchanged: `file-vat start`, `complete-manual`
without the flag, the Run state files and sealed vouchers written before this
slice read and behave as before. New output keys were only added (`filings`,
`unfiled_corrections`, `replacement`).

## The design decision: where a correction's VAT belongs

A correction voucher goes to the next open period, so something has to say
which VAT return it belongs to. It is a field of the sealed voucher,
`corrects_vat_period`, set at the seal and part of the hash:

- with it, the voucher's VAT is in the named period's return and not in the
  return of the period it is posted in (`VoucherBody::in_vat_period`, used by
  every VAT derivation);
- it is accepted only for a period that is sealed and whose return is filed
  (`VAT_PERIOD_NOT_FILED` otherwise), and only on a voucher posted after that
  period;
- without it nothing changes: the voucher is in its posting period's return,
  which is also the Tax Administration's allowed route for a financially minor
  error.

It is decided at the seal, like every other VAT treatment, and there is no way
to attribute afterwards: by then the posting period's return may have counted
the voucher. Bookkeeping reports and statements go by posting date and do not
read the field.

## What was built

- `model`, `domain`: the field on `VoucherDraft`/`VoucherBody` (optional,
  absent from the bytes when unset) and on the `voucher seal` draft input;
  VAT derivation and reconciliation select vouchers through `in_vat_period`.
- `ledger`: the seal gate; `vat_period_vouchers`;
  `vat_corrections_sealed_after`; `vat_filings` (every filing of a period,
  oldest first, the last in force); `ReplacementAuthority` and
  `replace_vat_filing`, a further `Sealed→Sealed` event bound in the CAS to the
  number of filings it replaces. `acquire_filing_authority` still refuses a
  filed period, so a second "first" filing stays impossible. `verify` accepts
  repeated filing events and still refuses anything else after a filing.
- `broker`: `VatReplacementReason` (`CLC`, `LGL`, `TXA`, `LAW`) and
  `FileVatReturn::into_replacement`.
- `process`: `start_file_vat_replacement`. Same Run as a first return, with:
  a preflight that reads the return the authority holds, refuses unless it is
  `Processed` (`VERO_VAT_RETURN_NOT_PROCESSED`) and unless the replacement
  changes an amount (`VAT_REPLACEMENT_CHANGES_NOTHING`), and shows the changed
  amounts before the approval; its own outbox key; no payment (the difference
  is settled on the tax account). Recovery: an open outcome blocks every new
  dispatch for the period; `reconcile` adopts a processed return with the
  approved values, and counts the unchanged earlier return, twice and at least
  a bank day apart, as the replacement's "not found". `status` and `preview`
  list every filing and the corrections no filing covers
  (`VAT_REPLACEMENT_DUE`); a preview of the posting period names the vouchers
  it leaves out (`VAT_CORRECTIONS_OF_OTHER_PERIODS`).
- `complete-manual --replacement` records a replacement made in OmaVero as a
  further filing with its evidence, with the same "resolve an open outcome
  first" rule; entered twice it is recorded once.
- `cli`: `file-vat replace` with an INPUT section, the flag, the draft field in
  the voucher help.
- `vero-vat-api-mock`: the documented replacement rules (`VAT106`, and a return
  after the first must be a replacement).
- Skills (`vero-fi`, `e-services`) and the guide in both languages
  (`tasks/file-vat-direct.md` "Korjaa jo annettu ilmoitus" / "Correct a return
  that has been filed", status, error codes, glossary, the pages that sent the
  user to OmaVero).
- Rules the code cannot show: `crates/broker/AGENTS.md` (the API facts and the
  recovery reasoning), `crates/ledger/AGENTS.md` (attribution and the filing
  sequence).

## API facts verified

From the Vero API developer portal (`api-developer.vero.fi`, SelfAssessedTaxes),
read 2026-11-21:

- `ReplacementReason` is one or more of `CLC` (calculation error), `LGL` (legal
  practice), `TXA` (tax audit guidance), `LAW` (law interpretation), separated
  by a comma; `ReplacementReturn` and `ReplacementReason` go together (error
  `VAT106`); "Every return after the first one needs to be a
  ReplacementReturn", and the first must not be one.
- A replacement answers like a first return: `UniqueIdentifier` and
  `AcceptedTimestamp`.
- GetFiledVATReturn/v2: "Only the values of the latest return are shown. If a
  return is being processed the values of the last accepted return are shown."
  The portal does not say which `Status` accompanies that; see the assessment.
- GetVATPeriods: status `Expired` means the period is outside the adjustment
  period; a replacement is refused for it.

## Not done

- No replacement has been sent to the Tax Administration; the first real one
  should be watched (assessment, F4).
- The Run neither computes nor pays the difference of a replacement.
- `voucher correct` (ADR-5) is still not built; the correction is an ordinary
  `voucher seal`.

## Verification

`scripts/ci-local.sh` passes. New tests: `crates/ledger/tests/vat_replacement.rs`,
`crates/process/tests/file_vat_replacement.rs`, two end-to-end cases in
`crates/cli/tests/file_vat_cli.rs`, one in the mock's `service.rs`. Review and
assessment: `review.md`, `assessment.md`.
