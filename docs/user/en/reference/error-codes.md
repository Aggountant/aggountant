# Error codes

This page lists the error codes you are most likely to meet and what is done
about them. The codes are aggountant's own, and they appear as they are in the
agent's output.

## How an error shows

When a command fails, it prints the error with its code, for example:

```json
{"error_code":"PERIOD_SEALED","error_kind":"user","message":"PERIOD_SEALED: period 2026-03 is Sealed; corrections go to the next open period","hint":"kirjaa seuraavalle avoimelle kaudelle"}
```

- `error_code` is a stable code that tells the agent what to do.
- `error_kind` is `user` when the cause is in the input or the state of the
  books and can be fixed (exit code 1), or `system` when the cause is in the
  environment, such as a file or a connection (exit code 2).
- `message` describes the cause in words.
- `hint` is the "Mitä tehdään" (what is done) column of this page's table
  in Finnish. It is present only for codes the table lists.
- `test_seam` is present only when the test variable `AGGOUNTANT_BROKER_SCRIPT`
  answered for the authority
  ([When something goes wrong](../troubleshooting.md#a-return-was-marked-filed-although-nothing-was-sent)).

The agent can ask for the whole explanation with `aggountant explain CODE`
(`--lang en` for English). The program reads the explanations from this
page, so they are the same as below.

Most errors are deliberate stops: the program refuses to do something that
would make the books wrong. A refusal leaves the books as they were. The
Vero.fi readiness blocker codes (`VERO_…`) have their own page:
[Vero.fi readiness blockers](vero-fi-blockers.md).

## Codes

| Code | What happened | What is done |
|---|---|---|
| `ACCOUNT_NOT_FOUND` | an entry refers to an account that is not in the chart of accounts | the agent picks an account from the chart |
| `ALREADY_INITIALIZED` | setting up a home was run on an existing home | nothing; the home already exists and was not touched |
| `APPROVAL_REQUIRED` | an entry was attempted without an approved preview | preview and approval first |
| `CAPTURE_VAT_CLASS_UNKNOWN` | the receipt proposal names a VAT rate class that aggountant does not know; nothing was recorded | the agent chooses the class that matches the receipt's VAT treatment; the message lists the known classes |
| `CAPTURE_PREPARATION_CHANGED` | the receipt details, the proposal or the chart of accounts changed after approval | a new preview and a new approval ([Sealing and approval](../concepts/sealing-and-approval.md)) |
| `EMPLOYER_REGISTER_MISMATCH` | the first month given for the separate report disagrees with the employer register in YTJ; nothing was written | check the comparison; use the register's month, or confirm your own month to the agent ([File the employer's separate report](../tasks/file-employer-report.md#set-up-the-separate-report-once)) |
| `EMPLOYER_REPORT_ALREADY_FILED` | the month already has an employer's separate report that Tulorekisteri accepted | no new report; a correction is a replacement report ([File the employer's separate report](../tasks/file-employer-report.md#if-something-goes-wrong)) |
| `EMPLOYER_REPORT_FILED_OUTSIDE` | an imported transcript 630 shows the month already reported outside aggountant | no new report; correct it in the e-service or record its reference first ([File the employer's separate report](../tasks/file-employer-report.md#when-an-earlier-payroll-service-filed-reports)) |
| `EMPLOYER_REPORT_INVALID_EVIDENCE` | the e-service evidence given with `complete-manual` does not settle the month: the report reference is not the 32-character reference, or for an uncertain replacement the version is missing or not newer than the replaced one | give the references and version the e-service shows; an old version there means the replacement has not arrived and the month waits; if the replaced version was not recorded, ask with `file-wage employer-report status` |
| `EMPLOYER_REPORT_INVALID_SETTINGS` | a pension insurance in the separate report settings has a provider code or policy number the report does not accept | check the pension company code and the policy number |
| `EMPLOYER_REPORT_NOT_CONFIGURED` | the Tulorekisteri profile has no separate report settings | [File the employer's separate report](../tasks/file-employer-report.md#set-up-the-separate-report-once) |
| `EMPLOYER_REPORT_NO_REFERENCE` | the month to replace is known only from transcript 630, which has no report reference | replace it in the e-service, or record the reference first (`complete-manual`) |
| `EMPLOYER_REPORT_PREVIOUS_STATED_NEEDLESSLY` | the previous report's pension insurances were stated, but aggountant already knows them (it sent the report) or nothing is being replaced | leave out `--previous-pension-insurance` and `--previous-no-pension-insurance` |
| `EMPLOYER_REPORT_PREVIOUS_UNKNOWN` | the report to replace was given outside aggountant and only its reference was recorded, so its pension insurances, which a replacement must repeat, are not known | read them from the report in the e-service and give them (`--previous-pension-insurance`, or `--previous-no-pension-insurance`), or correct the report in the e-service ([File the employer's separate report](../tasks/file-employer-report.md#when-an-earlier-payroll-service-filed-reports)) |
| `EMPLOYER_REPORT_OUTCOME_UNKNOWN` | the outcome of an earlier sending for the month is still unclear | ask for the state (`employer-report status`), do not send again ([When something goes wrong](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain)) |
| `EMPLOYER_REPORT_PAYMENTS_CONFLICT` | "No wages payable" for a month that has a travel allowance report | report the month as having only travel allowances |
| `EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED` | a replacement report would change the pension insurance details | cancel the report in the e-service and send a new one there |
| `EMPLOYER_REPORT_STATUS_UNKNOWN` | Tulorekisteri gave no final state | ask again later or check the e-service; do not send again |
| `EMPLOYER_REPORT_TRANSCRIPT_CONFLICT` | imported transcripts 630 disagree about the month | check the month in the e-service and record the report reference (`complete-manual`) |
| `EMPLOYER_REPORT_WAGES_UNSUPPORTED` | wages were paid in the month, and aggountant does not compute the health insurance contribution | file the report in the e-service ([What is ready and what is not](status.md#tulorekisteri-and-employees)) |
| `FISCAL_YEAR_CONFLICT` | a fiscal-year definition would move recorded vouchers or a closed year to another fiscal year | read the message; a definition cannot change what has already been recorded |
| `FISCAL_YEAR_UNDEFINED` | the date is before the company's first fiscal year, or no fiscal year ends in the given year | check the date or the year; `fiscal-year list` shows the definitions |
| `GATE_NOT_OPEN` | the approval has already been answered or is not open | ask the agent to show the run's state |
| `INVALID_FISCAL_YEAR` | the fiscal year is longer than 18 months, does not start on the first or end on the last day of a month, or does not follow the earlier fiscal years | correct the dates (Accounting Act 1:4) |
| `INVALID_INPUT` | the input has the wrong shape or an unknown field | the agent fixes the input |
| `MASTERDATA_CORRUPT` | the home's chart of accounts, its version or the VAT roles are missing or disagree with each other | after an upgrade: `home upgrade --dry-run` and `--apply` ([Upgrade aggountant](../tasks/upgrade.md)); otherwise a file was edited by hand, restore it from version control |
| `MASTERDATA_NOT_INITIALIZED` | the home has no chart of accounts | wrong home, or the home has not been set up ([When something goes wrong](../troubleshooting.md#vouchers-seem-to-have-disappeared)) |
| `MISSING_ASSIGNMENT` | a transaction on the bank statement has no entry, so the whole statement was rejected | the agent completes the entries ([Import the bank statement](../tasks/import-bank-statement.md)) |
| `NATURAL_KEY_ALREADY_SEALED` | warning from `voucher validate`: the same content has already been sealed under this `natural_key` | nothing; sealing returns the existing voucher and writes nothing new. Do not change the key to get past it |
| `OPEN_OBLIGATION` | an open obligation blocks the period's close | deal with the matter and mark the obligation resolved ([Periods](../concepts/periods.md#obligations)) |
| `OUTPUT_EXISTS` | the report file already exists | give a new file name |
| `PERIOD_CLOSING` | the period's close is in progress | approve the close or record it in the next period ([Close the month](../tasks/close-month.md)) |
| `PERIOD_NOT_SEALED` | a VAT return was attempted for a period that has not been closed, or a report needs the previous fiscal year's closing balances and that year has not been closed | close the period or the previous fiscal year first; if the previous fiscal year holds only the opening balances from setup, see how to date them in [Set up your company's books](../start/set-up-company.md) |
| `PERIOD_SEALED` | the period is closed, or the date falls in a closed fiscal year or one before it | record it in the next open period |
| `PERIOD_TRANSITION_CONFLICT` | the period's state does not allow the requested change, for example the period has already been filed, another close is in progress, or months and quarters are being mixed in the same year, or a quarter is closed in a fiscal year that is not the calendar year | read the message; a filed period is not filed again ([When something goes wrong](../troubleshooting.md)) |
| `RUN_NOT_FOUND` | the run cannot be found | check that the command hit the right home |
| `TRANSCRIPT_UNKNOWN_HEADER` | the file to import is not a Finnish-language transcript 630 from the Tulorekisteri e-service with the known columns | order transcript 630 again in Finnish; nothing was stored ([File the employer's separate report](../tasks/file-employer-report.md#when-an-earlier-payroll-service-filed-reports)) |
| `TULOREKISTERI_REJECTED` | the status query confirmed that Tulorekisteri rejected the report; the run stays waiting for attention and `run show` shows the rejection | find the error in the e-service, correct it there and record the result (`complete-manual`) ([When something goes wrong](../troubleshooting.md#tulorekisteri-rejected-the-report)) |
| `VAT_ACCOUNTS_UNRECONCILED` | a VAT account has postings without a VAT code, and tax would be missing from the return | file in OmaVero with corrected figures, or give the reason if the postings do not belong in the return ([File the VAT return in OmaVero](../tasks/file-vat-omavero.md#if-something-goes-wrong)) |
| `VAT_CURRENCY` | the VAT return figures cannot be formed; also when the period's VAT is negative because of corrections | [What is ready and what is not](status.md#vat-return) |
| `VAT_FIGURES_MISMATCH` | the VAT return figures do not match the books | the agent uses the preview's figures |
| `VAT_FILING_UNRESOLVED` | an earlier filing of the same period may have got through, and the Tax Administration has not yet answered "not found" twice at least one bank day apart | nothing was sent; the agent asks with `file-vat reconcile`, or first with `file-vat status` when the earlier run was interrupted while sending ([Uncertain outcome](../concepts/filings.md#uncertain-outcome), [interrupted filing](../troubleshooting.md#a-vat-filing-was-interrupted-while-sending)) |
| `VAT_POSTING_UNTAGGED` | warning from `voucher validate`: a posting on a VAT account has no VAT code, so the VAT return would leave its tax out | put the VAT code on the line that carries the VAT base before approval, unless the posting is a VAT payment or another transfer that does not belong in the return |
| `VAT_RECONCILIATION_ACK_MISMATCH` | the acknowledgement has no reason, its voucher numbers differ from the preview's, or there is nothing to acknowledge | the agent uses the preview's voucher numbers |
| `VAT_SETTLEMENT_ROLES_MISSING` | shown in the VAT return preview beside `VAT_ACCOUNTS_UNRECONCILED`: the vouchers' chart of accounts has no settlement accounts marked, so VAT payments and refunds cannot be recognised | upgrade the home's master data and preview again, or acknowledge the payments ([File the VAT return in OmaVero](../tasks/file-vat-omavero.md#if-something-goes-wrong)) |
| `VAT_STATUS_UNKNOWN` | the VAT return's status could not be read from the Tax Administration (connection, credentials or readiness) | with an acknowledgement the return has still been filed; for an interrupted filing nothing was changed. Query again later, do not send again ([The return does not show in OmaVero](../troubleshooting.md#the-return-does-not-show-in-omavero)) |
| `VERO_NOT_READY` | direct filing readiness is not in order | [Vero.fi readiness blockers](vero-fi-blockers.md) |
| `VERO_TEST_NOT_READY` | `vero-fi verify --target test` was refused because readiness for the test environment is not in order, for example the credential is for the production target | the message names the blockers ([Vero.fi readiness blockers](vero-fi-blockers.md)); the test verification is optional and production does not need it |
| `VERO_VAT_PERIOD_NOT_FILEABLE` | shown in the message of a failed `file-vat start` (its `error_code` is `PROCESS_STORE`) and on the run's `reconcile` step in `run show`: the Tax Administration shows the VAT period in a state other than unfiled or missing, for example already filed; nothing was sent | check the period in OmaVero; after the cause is fixed, start a new run instead of retrying the same one |
| `VOUCHER_NOT_FOUND` | there is no voucher with the given number | check the number or the home |
| `WORKFLOW_VERSION_MISMATCH` | a run in progress was started with an older version of the program | [Upgrade aggountant](../tasks/upgrade.md) |
| `YTJ_UNAVAILABLE` | the YTJ open data did not answer the registration query | the registration is unknown, not absent; try later or look it up on ytj.fi |

In other cases `message` gives the cause. Ask the agent to explain it.

## See also

- [When something goes wrong](../troubleshooting.md)
- [Vero.fi readiness blockers](vero-fi-blockers.md)
