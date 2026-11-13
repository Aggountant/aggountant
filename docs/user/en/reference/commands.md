# Commands at a glance

This page shows which task each of aggountant's command groups belongs to, so
you can recognise what the agent runs. Options and input formats are not here:
the program tells you those itself, with `aggountant --help` and, for example,
`aggountant voucher seal --help`. Every command accepts the options `--home`
(which home) and `--json` (machine-readable output for the agent).

| Command | What it does | Where in the guide |
|---|---|---|
| `init` | sets up a company home and the default chart of accounts | [Set up your company's books](../start/set-up-company.md) |
| `home upgrade` | adds the master data a new version needs to an existing home | [Upgrade aggountant](../tasks/upgrade.md) |
| `config` | shows which home is in use and where that came from; the configuration file's location | [The company home](../concepts/company-home.md) |
| `skill` | lists, installs and prints the agent's skills | [Install aggountant and its skills](../start/install.md) |
| `chart` | lists the accounts in the chart of accounts | [Set up your company's books](../start/set-up-company.md) |
| `knowledge` | the company's saved facts and practices (`knowledge company`) | [The company home](../concepts/company-home.md#the-company-knowledge-store) |
| `import camt053` | reads a bank statement and seals its entries (also camt.052) | [Import the bank statement](../tasks/import-bank-statement.md) |
| `artifact` | archives a source file, such as a receipt or a confirmation | [Record receipts and purchase invoices](../tasks/record-receipts.md) |
| `capture-receipt` | records an ordinary purchase receipt through a preview and an approval | [Record receipts and purchase invoices](../tasks/record-receipts.md) |
| `voucher` | voucher draft, validation, sealing, search and the chain check | [Record a sales invoice, payroll or another manual entry](../tasks/record-other-entries.md), [Correct a wrong voucher](../tasks/correct-voucher.md) |
| `report statements` | the monthly report in HTML or JSON | [See how the month went](../tasks/monthly-report.md) |
| `obligation` | open obligations and resolving them | [Periods](../concepts/periods.md#obligations) |
| `run` | the month's close run: start, approval, resume, status and moving to a new version | [Close the month](../tasks/close-month.md) |
| `period` | a period's state (`open`, `closing`, `sealed`) and its VAT filing status | [Periods](../concepts/periods.md) |
| `fiscal-year` | a fiscal year's dates and state (`open`, `sealed`); defining a fiscal year that is not the calendar year | [Periods](../concepts/periods.md#fiscal-year-status) |
| `file-vat` | VAT return preview, direct submission, the status of a filed return, status reconciliation and recording a return made by hand | [File the VAT return in OmaVero](../tasks/file-vat-omavero.md), [directly](../tasks/file-vat-direct.md) |
| `vero-fi` | direct filing settings, the readiness check and the acceptance tests | [Set up direct VAT filing](../tasks/set-up-vero-api.md) |
| `employee` | the employee register | [Report travel allowances to Tulorekisteri](../tasks/report-travel-allowances.md) |
| `file-wage` | the travel allowance report to Tulorekisteri | [Report travel allowances to Tulorekisteri](../tasks/report-travel-allowances.md) |

## In the program, but not in use

These commands are in the program, but they cannot yet be used for real
bookkeeping. The reason and what to do instead are on
[What is ready and what is not](status.md).

| Command | What it would do |
|---|---|
| `run-payroll` | payroll run |
| `hunt-receipts`, `intake` | automatic receipt search and its findings |
| `capture-receipt start`, `answer`, `resume` | consultation for an unusual receipt |
| `close-year` | year-end run |

## See also

- [Glossary](glossary.md)
- [Error codes](error-codes.md)
