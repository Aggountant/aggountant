# Result: filing-correction-unrecordable

A correction of an accepted travel report now goes through aggountant, and
evidence that cannot be recorded is refused instead of dropped.

```
aggountant file-wage correction preview wr-… --action replace|cancel --timestamp T [< corrected.json]
aggountant file-wage correction submit  wr-… --action replace|cancel --timestamp T --record-hash HASH [< corrected.json]
aggountant file-wage correction status  wr-…
aggountant file-wage correction complete-manual wr-… --action replace|cancel --ir-report-id GUID [--report-version N] [--ir-delivery-id GUID]
```

## What was built

1. **Never silent.** `file-wage complete-manual` on a Run that already holds an
   accepted report refuses with `WAGE_REPORT_ALREADY_ACCEPTED` (exit 1), says
   that nothing was recorded and names `file-wage correction`. The other
   manual paths were checked: `file-vat complete-manual` and
   `file-wage employer-report complete-manual` already refuse loudly.
2. **Corrections through aggountant (Incomes Register travel report).**
   - A correction is its own Run (`wc-…`, workflow `file-wage-correction`):
     offline preview, `submit` naming the previewed `record_hash`, one send
     through the outbox, and an unknown outcome settled only by the signed
     status query or by e-service evidence. Nothing is resent and no further
     correction is accepted while one is in doubt.
   - **Replacement**: `WageReportRequestToIR` with `ActionCode` 2, `IRReportId`
     and, when known, `ReportVersion`, in its own record (`DeliveryId` derived
     from the correction and its creation time). It is refused before sending
     when it would change the payer, the income earner, the payment date or
     the pay period, when it repeats the report, and when the income earner's
     stored identity code is not the one the accepted report was sent with.
   - **Cancellation**: `InvalidationRequestToIR` to `InvalidationService`
     (`SendInvalidation`), record type 105, by `IRItemId` and `ItemVersion`.
     It carries no personal data.
   - **E-service fallback**: `correction complete-manual` records a
     replacement or cancellation the user made in the e-service, or settles a
     correction in doubt when the evidence shows a newer version.
   - `correction status` gives what the register holds now (`accepted`,
     `replaced`, `cancelled`, `correction_outcome_unknown`), the receipt in
     force and every filing. `run show` of a Run whose receipt is no longer in
     force says `superseded` with the status command, instead of `accepted`.
3. **Guidance.** `skills/tulorekisteri`, `skills/vero-fi` and
   `skills/e-services` no longer say a correction is silently unrecorded; the
   travel report's correction is described as the program's route, with the
   e-service as the fallback and how it is recorded. The user guide (fi and
   en) has the task section, the error codes, the troubleshooting rows and the
   status page.

## API facts used

From the Incomes Register's developer documentation: the 2025 technical
interface ZIP (XSD and WSDL, vendored for the wage and cancellation schemas
with their checksums), *Data delivery – Schemas – Earnings payment reports
2026* (§2.1, §2.4, §2.12, §2.13), *Cancellation data 2026* (§2.1–2.2) and
*Processing feedback* (ValidItems).

- Replacement: `ActionCode` 2; `IRReportId` or the payer's `ReportId`
  identifies the report; `ReportVersion` is optional and, when given, must be
  the latest. The pay period, the payer identifiers and the income earner
  identifiers must match the previous version.
- Cancellation: record type 105 (earnings payment reports; 106 is the
  employer's separate report); `Source` and `FaultyControl` are mandatory; the
  item is found by `IRItemId` and/or `ItemId`, with the latest `ItemVersion`
  when given.
- Feedback: a replacement's item carries the replaced version + 1, and a
  cancellation's item the cancelled version + 1, with the report's reference.

The generated replacement and cancellation records are validated against the
vendored XSDs (`crates/broker/tests/payer_summary_schema.rs`).

## What is not in this change

- **VAT replacement return.** Filed as `vat-replacement-return` with the API
  facts the code holds and the open questions. It needs its own design: a
  filed period is sealed and its corrections are sealed in a later period, so
  the replacement's figures need an attribution the Ledger does not have, and
  `vat_filed` records one filing only. Until then `file-vat` refuses loudly on
  a filed period and the skill says the OmaVero correction is not recorded.
- **Employer's separate report cancellation** stays in
  `employer-report-record-cancellation`. The cancellation record builder takes
  the record type as an argument, so 106 needs the process and CLI side only.
- A new report after a cancellation needs a new `source_reference`: the
  cancelled report's Run keeps its identity. `wage_payment_months` still
  counts a cancelled report's month as a month with reimbursements; the
  employer's separate report then refuses "no wages payable" for it, which is
  loud, not silent.
- Nothing was run against the real register. Tests use the broker's adapter
  mocks and the mTLS mock server (which now answers `InvalidationService`).

## Simplicity

- Reused: the Run journal, gates, lease and outbox as they are; the receipt
  type and its note format; the status-feedback parser with its version rule.
- Merged: `ProcessError::EmployerReport` became `IncomesRegister` (same codes),
  and the three copies of "journal the answer to a send" became one
  `record_outcome`; `classify` and `write_state` are shared instead of copied.
- Left out: a generic "correction of any filing" abstraction across VAT and
  the Incomes Register (the two channels share rules, not shapes), batch
  cancellation, cancellation of a whole record (types 109–111), and automatic
  re-filing after a cancellation.

## Verification

`scripts/ci-local.sh` (fmt, clippy, cargo-deny, build and tests on stable and
MSRV). Review and assessment: `review.md`, `assessment.md`.
