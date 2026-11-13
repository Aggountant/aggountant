# Error codes

This page lists the error codes you are most likely to meet and what is done
about them. The codes are aggountant's own, and they appear as they are in the
agent's output.

## How an error shows

When a command fails, it prints the error with its code, for example:

```json
{"error_code":"PERIOD_SEALED","error_kind":"user","message":"PERIOD_SEALED: period 2026-03 is Sealed; corrections go to the next open period"}
```

- `error_code` is a stable code that tells the agent what to do.
- `error_kind` is `user` when the cause is in the input or the state of the
  books and can be fixed (exit code 1), or `system` when the cause is in the
  environment, such as a file or a connection (exit code 2).
- `message` describes the cause in words.

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
| `CAPTURE_PREPARATION_CHANGED` | the receipt details, the proposal or the chart of accounts changed after approval | a new preview and a new approval ([Sealing and approval](../concepts/sealing-and-approval.md)) |
| `FISCAL_YEAR_CONFLICT` | a fiscal-year definition would move recorded vouchers or a closed year to another fiscal year | read the message; a definition cannot change what has already been recorded |
| `FISCAL_YEAR_UNDEFINED` | the date is before the company's first fiscal year, or no fiscal year ends in the given year | check the date or the year; `fiscal-year list` shows the definitions |
| `GATE_NOT_OPEN` | the approval has already been answered or is not open | ask the agent to show the run's state |
| `INVALID_FISCAL_YEAR` | the fiscal year is longer than 18 months, does not start on the first or end on the last day of a month, or does not follow the earlier fiscal years | correct the dates (Accounting Act 1:4) |
| `INVALID_INPUT` | the input has the wrong shape or an unknown field | the agent fixes the input |
| `MASTERDATA_NOT_INITIALIZED` | the home has no chart of accounts | wrong home, or the home has not been set up ([When something goes wrong](../troubleshooting.md#vouchers-seem-to-have-disappeared)) |
| `MISSING_ASSIGNMENT` | a transaction on the bank statement has no entry, so the whole statement was rejected | the agent completes the entries ([Import the bank statement](../tasks/import-bank-statement.md)) |
| `OPEN_OBLIGATION` | an open obligation blocks the period's close | deal with the matter and mark the obligation resolved ([Periods](../concepts/periods.md#obligations)) |
| `OUTPUT_EXISTS` | the report file already exists | give a new file name |
| `PERIOD_CLOSING` | the period's close is in progress | approve the close or record it in the next period ([Close the month](../tasks/close-month.md)) |
| `PERIOD_NOT_SEALED` | a VAT return was attempted for a period that has not been closed, or a report needs the previous fiscal year's closing balances and that year has not been closed | close the period or the previous fiscal year first; if the previous fiscal year holds only the opening balances from setup, see how to date them in [Set up your company's books](../start/set-up-company.md) |
| `PERIOD_SEALED` | the period is closed, or the date falls in a closed fiscal year or one before it | record it in the next open period |
| `PERIOD_TRANSITION_CONFLICT` | the period's state does not allow the requested change, for example the period has already been filed, the close was rejected, or months and quarters are being mixed in the same year, or a quarter is closed in a fiscal year that is not the calendar year | read the message; a filed period is not filed again ([When something goes wrong](../troubleshooting.md)) |
| `RUN_NOT_FOUND` | the run cannot be found | check that the command hit the right home |
| `VAT_CURRENCY` | the VAT return figures cannot be formed; also when the period's VAT is negative because of corrections | [What is ready and what is not](status.md#vat-return) |
| `VAT_FIGURES_MISMATCH` | the VAT return figures do not match the books | the agent uses the preview's figures |
| `VERO_NOT_READY` | direct filing readiness is not in order | [Vero.fi readiness blockers](vero-fi-blockers.md) |
| `VOUCHER_NOT_FOUND` | there is no voucher with the given number | check the number or the home |
| `WORKFLOW_VERSION_MISMATCH` | a run in progress was started with an older version of the program | [Upgrade aggountant](../tasks/upgrade.md) |

In other cases `message` gives the cause. Ask the agent to explain it.

## See also

- [When something goes wrong](../troubleshooting.md)
- [Vero.fi readiness blockers](vero-fi-blockers.md)
