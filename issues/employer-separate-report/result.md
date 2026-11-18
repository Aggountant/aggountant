# Result: employer-separate-report

aggountant files the monthly employer's separate report (työnantajan
erillisilmoitus, *PayerSummaryReport*) to the Incomes Register through the same
own-organisation channel `file-wage` uses:

```
aggountant file-wage employer-report preview  --month YYYY-MM --payments none|tax-free-reimbursements-only|wages --timestamp T [--replace]
aggountant file-wage employer-report submit   ... --record-hash H
aggountant file-wage employer-report status   --month YYYY-MM
aggountant file-wage employer-report missing  [--as-of DATE]
aggountant file-wage employer-report complete-manual --month YYYY-MM --ir-report-id R [--ir-delivery-id D] [--report-version N]
```

## Provenance of this slice

The first worker died on 2026-11-16 (disk full and model quota at the same
time). Its preserved branch was harvested: every file was checked for
truncation, the vendored XSDs were compared byte-for-byte with a fresh download
of the official ZIP, and the workspace built and passed its tests before
anything was added. The harvesting worker then fixed a clippy error, ran the
review, fixed two confirmed findings (`review.md`, `assessment.md`) and wrote
this file. The earlier worker left no notes of its own.

## What was built

- `crates/domain/src/payroll.rs`: `ReportMonth` (deadline: 5th of the next
  month), `MonthPayments`, `EmployerReportContent` (101 "No wages payable" or
  102 health insurance contribution), `EmployerReportAction` (new or
  replacement of `IRReportId` + version), `EmployerSeparateReport` with
  validation.
- `crates/broker/src/tulorekisteri_xml.rs`: `payer_summary_record` builds the
  unsigned `PayerSummaryReportRequestToIR`; signing and the SOAP envelope reuse
  the wage-report path.
- `crates/broker/src/tulorekisteri.rs`: operations
  `PayerSummaryReportService.svc` / `SendPayerSummaryReport` and the signed
  `StatusService.svc` / `GetDeliveryDataStatus` query, with feedback matched
  against the expected report and version; the company setting
  `employer_report` (`registered_from`, contact, `pension_insurances`) in the
  existing `.aggountant/tulorekisteri.json` profile.
- `crates/process/src/file_employer_report.rs`: the run model. A Run's id is
  the month plus the hash of the exact unsigned record; `submit` rebuilds and
  re-hashes before dispatch and refuses any other hash. `EffectExecuting` is
  journalled before the send, so a crash or timeout leaves the month
  `outcome_unknown`, which blocks every further send for the month until the
  outbox, the status query or e-service evidence settles it. An accepted Run
  fences the month; the only further send is `--replace`.
- `crates/cli/src/wage.rs`: the `employer-report` subcommands, additive.
- Vendored schemas in `crates/broker/schemas/tulorekisteri/2025/` and a test
  that validates every report shape against them.
- User guide (fi, en): `tasks/file-employer-report.md`, linked from the
  travel-allowance task, the monthly routine, commands, status, error codes and
  troubleshooting. Skill: `skills/vero-fi/SKILL.md` section "Työnantajan
  erillisilmoitus (joka kuukausi)" says the report is monthly, how to catch up
  missing months oldest-first with a separate approval each, and how to record
  e-service filings; `skills/aggountant-init/SOUL.md` points to it.

## Acceptance

- Preview / approval / single send / receipt:
  `no_wages_month_previews_offline_submits_once_and_the_receipt_fences_it`
  (preview writes nothing and uses no network).
- Second submit refused; unknown outcome resolved read-only:
  `unknown_outcome_blocks_every_send_until_the_status_query_settles_it`,
  `a_status_query_without_a_definitive_answer_keeps_the_month_parked`,
  `crash_after_an_accepted_send_recovers_from_the_outbox_without_network`,
  CLI `EMPLOYER_REPORT_ALREADY_FILED` in `crates/cli/tests/file_wage_cli.rs`.
- Missing months, read-only, with deadlines:
  `missing_months_list_unreported_months_with_deadlines_and_write_nothing`.
- Mock/scripted coverage of no-wages, reimbursements-only, receipt, rejection,
  unknown outcome: the tests above plus
  `reimbursements_only_month_reports_zero_contribution_and_never_no_wages`,
  `rejection_is_definitive_and_a_corrected_report_may_be_sent`,
  `replacement_targets_the_accepted_report_and_its_version`, and the two
  replacement-evidence tests. Nothing submits to the production register.
- Schema: `crates/broker/tests/payer_summary_schema.rs`
  (`every_report_shape_validates_against_the_official_schema`, plus a negative
  test that the validator rejects records the schema forbids).
- Guide fi+en with a link from the travel-allowance task: done.

## Primary sources

- Technical interface ZIP for application developers 2025,
  <https://www.vero.fi/globalassets/tulorekisteri/dokumentaatio-2025/tekninen-rajapinta-zip-sovelluskehittajalle-2025.zip>
  (SHA-256 `ca9b8c32…eaf8`, published 8 January 2025, re-downloaded and
  verified 2026-11-18). XSDs `PayerSummaryReportsToIR.xsd`,
  `PayerSummaryReportsToIRTypes.xsd`, `IRCommonTypes.xsd`,
  `xmldsig-core-schema.xsd` vendored unmodified; WSDLs
  `PayerSummaryReportService.wsdl` (`SendPayerSummaryReport`) and
  `StatusService.wsdl` give the service and operation names.
- *Data delivery – Schemas – Employer's separate reports 2025*, version 1.0,
  3 June 2024,
  <https://www.vero.fi/globalassets/tulorekisteri/dokumentaatio-2025/data-delivery-schemas-employers-separate-reports-2025.pdf>
  (codes 101/102, `ReportVersion`, §2.9 pension insurances in replacements).
- *Reporting data to the Incomes Register: employer's separate report*,
  VH/6456/05.00.00/2025, valid from 1 January 2026,
  <https://www.vero.fi/en/incomes-register/companies-and-organisations/detailed-guidance/63595/reporting-data-to-the-incomes-register-employers-separate-report9/>
  (§2–3 monthly obligation and deadline, §4.2–4.3 "No wages payable" vs EUR 0,
  §5 corrections).
- Form instructions TULOR 6103e,
  <https://www.vero.fi/en/incomes-register/companies-and-organisations/employers/earnings-payment-data/reporting-on-paper/forms/employers-separate-report-tulor-6103e/>,
  which state the same reimbursements-only rule.

**Reimbursements-only month:** the documentation settles it. A regular
employer that paid only tax-free reimbursements of expenses does not report
"No wages payable"; it reports the employer's health insurance contribution
as EUR 0 (guidance §4.3, TULOR 6103e). Implemented that way; `none` is refused
for a month that has a `file-wage` report.

## Decisions for later review

- **CLI shape**: `file-wage employer-report preview|submit|status|missing|complete-manual`,
  consistent with `file-wage` (pre-authorized). `status` runs the signed query
  only when the month is in doubt; otherwise it is offline.
- **What was paid is stated by the caller** (`--payments`), not inferred from
  accounts. Alternative: derive it from the ledger. Chosen because the books do
  not reliably separate wages from reimbursements yet, and the approver must
  look at the month anyway. The only cross-check is against `file-wage` runs.
- **Month with wages refused** (`EMPLOYER_REPORT_WAGES_UNSUPPORTED`), with the
  e-service plus `complete-manual` as the route (pre-authorized default).
- **Employer-registration start month** is `employer_report.registered_from`
  in the existing Tulorekisteri profile (pre-authorized).
- **Deadline shown is always the 5th** of the following month; weekend/holiday
  relief is not computed, so `overdue` can turn true a day or two before the
  legal last day. Alternative: compute Finnish banking days. Chosen as
  conservative; it never makes a report late. (Review F3.)
- **`missing` ends at the previous month**; the current month is not yet due.
- **Replacement pension insurances** repeat the previous version's (register
  rule §2.9); a changed policy is refused and sent to the e-service (guidance
  §5). Untested against the register whether 102 with `PensionInsurances` is
  accepted; if not, the result is a definitive rejection. (Review F2.)
- **Settling an in-doubt replacement from e-service evidence** needs a version
  newer than the replaced one; when the replaced version is unknown only the
  status query settles it. (Review F1.)
- **The record hash is shown to the user**: the report carries no personal
  data, unlike the wage report.
- **Timestamp freshness**: `submit` refuses a preview timestamp more than 24 h
  from now, because it becomes the report date.
- **2027 schema** (published 29 May 2026, applies from reporting period
  2027-01) is not vendored; the vendored README says to review it before the
  first 2027 report.
- `crates/process` was changed (new module, one helper in `file_wage.rs`,
  one error variant) because the run model needed it; `vero-vat-api-mock`
  routes the new service path to its scripted SOAP answers.

## Gates

- `scripts/ci-local.sh`: all gates green after rebasing onto current `main`
  (fmt, clippy, cargo-deny, build and test on stable and 1.89, user guide).
  Before the rebase, cargo-deny failed on RUSTSEC-2026-0543 (chrono
  unmaintained); `main` has since added a dated ignore pending
  `migrate-from-chrono`, so this worker's duplicate intake was dropped.

## Follow-ups filed (unlaned)

- `employer-report-record-cancellation`
