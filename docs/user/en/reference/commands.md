# Commands at a glance

The table shows which task each of aggountant's command groups belongs to, so
you can recognise what the agent runs. The program itself tells you the
options and input formats: `aggountant --help` and, for example,
`aggountant voucher seal --help`. A command that reads JSON input shows the
fields and an example in the INPUT section of its help. Every command accepts
the options `--home` (which home) and `--json` (machine-readable output for
the agent).

| Command | What it does | Where in the guide |
|---|---|---|
| `init` | sets up a company home and the default chart of accounts | [Set up your company's books](../start/set-up-company.md) |
| `home upgrade` | adds the master data a new version needs to an existing home | [Upgrade aggountant](../tasks/upgrade.md) |
| `config` | shows which home is in use and where that came from, and where the configuration file is | [How aggountant finds the home](../concepts/company-home.md#how-aggountant-finds-the-home) |
| `skill` | lists, installs and prints the agent's skills | [Install aggountant and its skills](../start/install.md) |
| `explain` | explains an error code: what happened and what to do | [Error codes](error-codes.md) |
| `chart` | lists the accounts in the chart of accounts | [Set up your company's books](../start/set-up-company.md) |
| `knowledge` | reads and maintains the company's saved facts and practices (`knowledge company`) | [The company knowledge store](../concepts/company-home.md#the-company-knowledge-store) |
| `import camt` | reads a bank statement (camt.053 or camt.052) and seals its entries; the old name `import camt053` still works | [Import the bank statement](../tasks/import-bank-statement.md) |
| `artifact` | archives a source file, such as a receipt or a confirmation | [Record receipts and purchase invoices](../tasks/record-receipts.md) |
| `capture-receipt record` | records an ordinary purchase receipt through a preview and an approval | [Record receipts and purchase invoices](../tasks/record-receipts.md) |
| `voucher` | drafts a voucher, validates and seals it, searches sealed vouchers, checks the chain and lists the VAT codes valid on a date | [Record a sales invoice, payroll or another manual entry](../tasks/record-other-entries.md), [Correct a wrong voucher](../tasks/correct-voucher.md) |
| `report statements` | produces the monthly report in HTML or JSON | [See how the month went](../tasks/monthly-report.md) |
| `obligation` | lists obligations and marks an open one resolved | [Obligations](../concepts/periods.md#obligations) |
| `run` | previews the month's close, starts the close run, passes your approval to it, resumes an interrupted run, shows a run's status and moves a run to a new version | [Close the month](../tasks/close-month.md) |
| `period` | shows a period's state (`open`, `closing`, `sealed`) and whether its VAT return has been filed | [Periods, closing and the fiscal year](../concepts/periods.md) |
| `fiscal-year` | shows a fiscal year's dates and state (`open`, `sealed`) and defines a fiscal year that is not the calendar year | [Fiscal-year status](../concepts/periods.md#fiscal-year-status) |
| `file-vat` | previews the VAT return, sends it directly, sends a replacement return, shows the status of a filed return, settles a submission whose outcome is unknown and records a return made by hand | [File the VAT return in OmaVero](../tasks/file-vat-omavero.md), [Send the VAT return directly to the Tax Administration](../tasks/file-vat-direct.md) |
| `tulorekisteri` | saves the Tulorekisteri channel profile and checks readiness | [Report travel allowances to Tulorekisteri](../tasks/report-travel-allowances.md) |
| `vero-fi` | saves the direct filing settings, checks readiness and runs the acceptance tests | [Set up direct VAT filing](../tasks/set-up-vero-api.md) |
| `employee` | adds an employee to the employee register and shows the register's data | [Report travel allowances to Tulorekisteri](../tasks/report-travel-allowances.md) |
| `file-wage` | sends the travel allowance report to Tulorekisteri, and `file-wage correction` replaces or cancels an accepted report | [Report travel allowances to Tulorekisteri](../tasks/report-travel-allowances.md) |
| `file-wage employer-report` | files the employer's separate report, lists missing months, checks the registration in YTJ and imports transcript 630 | [File the employer's separate report](../tasks/file-employer-report.md) |
| `close-year` | the year-end run: records depreciation, accruals and corrections and seals the fiscal year | [Prepare the financial statements](../tasks/year-end.md) |

## In the program, but not in use

These commands cannot yet be used for real bookkeeping. The reason and what to
do instead are on [What is ready and what is not](status.md).

| Command | What it would do |
|---|---|
| `run-payroll` | payroll run |
| `hunt-receipts`, `intake` | automatic receipt search and its findings |
| `capture-receipt start`, `answer`, `resume` | consultation for an unusual receipt |

## See also

- [Glossary](glossary.md)
- [Error codes](error-codes.md)
- [What is ready and what is not](status.md)
