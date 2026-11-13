# What is ready and what is not

This page describes every limitation: what does not work, why, and what you
do instead. Other pages point here with one sentence. The identifier of the
issue that tracks a limitation is in parentheses; the issues are in the
project's [GitHub repository](https://github.com/aggountant/aggountant) under
`issues/`. The page applies to version 0.1.

## Overview

aggountant is at version 0.1 and already keeps the books of a few Finnish
companies that have no accounting firm. Features are added as these
companies meet new situations. That is why ordinary monthly work is in good
shape, while rarer cases are filled in step by step. Commands and file
formats may still change between versions.

**Ready for real bookkeeping:**

- recording vouchers and archiving their attachments
- importing bank statements (camt.053 and camt.052)
- recording ordinary purchase receipts with their VAT treatment
- closing a month
- monthly reports
- the VAT return, directly to the Tax Administration or by hand in OmaVero
- reporting tax-free travel allowances to Tulorekisteri
- the year-end entries and sealing the fiscal year with the year-end run.

**In the program, but not yet usable for real bookkeeping:** the payroll run,
aggountant's own automatic receipt search and consultation for unusual
receipts. They are described below.

## Installation

**There are no ready-made installation packages yet.** aggountant is compiled
from source with the Rust toolchain. Do this:
[Install aggountant and its skills](../start/install.md).

## Bank statement

- **Every confirmed transaction needs an entry.** If even one is missing
  (`MISSING_ASSIGNMENT`) or refers to an account that is not in the chart of
  accounts (`ACCOUNT_NOT_FOUND`), the whole statement is rejected and nothing
  is sealed. This also applies to a row that pays a receipt already recorded;
  see below.
- **Only Holvi's statement has been tried.** Other banks' camt files may
  work, but they have not been tried. After the first import, check that the
  bank account balance matches the statement.
- **Sales VAT is not split out from the statement.** A sale that includes
  VAT is recorded as a manual entry, and the customer's payment from the
  statement against trade receivables (`vat-sales-expansion`).
- **The bank statement import has no preview hash** as receipt recording
  does. Your approval is that you look at the agent's list of entries row by
  row before sealing.
- **An empty statement imports successfully with no transactions**, and the
  program does not mention it separately. If an import gives zero
  transactions, check that the file is the statement for the right month and
  the right account (`camt-empty-report-and-rename`).

### Purchase invoice paid from the bank account

When a purchase invoice or receipt has been paid from the company's account,
the same purchase shows both in the receipt and on the bank statement. In this
version neither route is complete:

- **Recorded from the bank statement**, the voucher has the statement
  attached, not the invoice. The invoice cannot be attached to a voucher
  recorded from the statement.
- **Recorded as a receipt**, the invoice is attached, but the bank statement
  import still requires an entry for the same payment row. If the receipt was
  recorded against the bank account and the payment row is also recorded as
  an expense, **the same purchase is in the books twice**.

The program does not yet offer a way to mark a statement row as the payment of
a receipt already recorded, and the guidance does not yet give one settled way
to record it. Do this: when you approve the statement's entries, ask about
every row whose receipt has already been recorded which account the agent
records it to, and make sure the expense does not appear twice. Comparing the
bank account balance in the monthly report with the statement reveals double
booking before the close (`bank-row-for-recorded-receipt`).

## Receipts and entries

- **Consultation for unusual receipts is not in use.** The program has a
  route (`capture-receipt start`) in which only the details you approved would
  go out, but there is no connection to a consultation service behind it. The
  agent asks you or your bookkeeper.
- **aggountant's own receipt search is not in use.** The `hunt-receipts`
  command is not connected to a real email or online shop account. The agent
  can look for receipts with its own tools wherever you have given it access;
  otherwise searching is manual work.
- **The receipt route does not handle partial deduction rights or imports.**
  The agent tells you if a purchase is one of those. Ask your bookkeeper.
- **There is no separate correction command.** A correction is recorded as an
  ordinary manual entry: [Correct a wrong voucher](../tasks/correct-voucher.md).
- **A single voucher's postings are visible only to the agent.** Without
  `--json`, `voucher show` shows only the number and the hash. Ask the agent
  to show the voucher's postings.

## Periods

### Rejected close

**A rejected close leaves the period permanently in the state `closing`.**
Nothing can then be recorded in the period and it cannot be closed again,
and this version has no command to make the period open again. Do this:
start a close only when the period is finished, and approve a close you have
started. Things noticed afterwards are recorded in the next period. If a close
has already been rejected, see
[When something goes wrong](../troubleshooting.md#a-month-is-stuck-in-closing)
(`rejected-close-reopens-period`).

### Fiscal year and periods

- **A fiscal year other than the calendar year is closed monthly.** Quarterly
  closing is available only for calendar fiscal years.
- **Two fiscal years cannot end in the same calendar year.** A change of
  fiscal year is made with one transition year of at most 18 months.
- **Depreciation is not calculated for a fiscal year that is not 12 months
  long.** The year-end run refuses to calculate it; calculate that year's
  depreciation yourself and give it to the run as a correction, so it is
  attributed to the right year.
- **A fiscal-year definition cannot be changed.** It can only be followed by a
  new one, and not in a way that would move recorded vouchers to another
  fiscal year.
- **You cannot close both months and quarters in the same year.** Monthly is
  the recommended and tried option.

## VAT return

- **The preview's guidance text is out of date.** It always tells you to enter
  the figures by hand in OmaVero, even when direct filing is set up. Whether
  you can use the direct route depends only on whether readiness is in order
  (`vat-preview-filing-instruction`).
- **The payment step of a direct return never completes.** When there is tax
  to pay, the run opens a payment approval, but aggountant has no payment
  connection. The run keeps waiting even though the return has been filed.
  Pay the tax yourself with the OmaVero reference (`vat-run-payment-terminal`).
- **The Tax Administration's bank account number is given every time.** The
  agent gives the filing run the account number shown in OmaVero. The program
  does not know the payment reference (`tax-payment-references`).
- **Replacement returns are not made.** When the filed figures need a
  correction, make a replacement return in OmaVero.
- **Fields 309–311 for EU sales and zero-rated sales do not come from the
  books.** If your company has EU sales or exports, file the return in OmaVero
  and enter these fields yourself. aggountant does not make the EC sales list
  (yhteenvetoilmoitus) (`vat-zero-rated-sales-fields`).
- **A period whose VAT is negative because of corrections cannot be filed
  with aggountant.** For example, a month whose only sale is a credit note.
  The error misleadingly names the currency (`VAT_CURRENCY`). File the return
  in OmaVero (`vat-negative-period-filing`).
- **A VAT posting without a VAT code cannot be corrected into the return in
  aggountant.** The return is calculated from VAT-coded postings. Voucher
  validation warns about an uncoded VAT posting, and the VAT return preview
  shows it (`VAT_ACCOUNTS_UNRECONCILED`), but a closed month cannot be
  reopened for correction. File the return in OmaVero with corrected figures
  instead ([File the VAT return in OmaVero](../tasks/file-vat-omavero.md#if-something-goes-wrong)).
- **Quarterly periods have been tried less.** Direct filing has been tried
  with monthly periods.
- **aggountant does not prepare Ilmoitin.fi files.**

## Payroll

**Cash wages cannot yet be calculated, paid or reported with aggountant.**
The payroll run (`run-payroll`) is in the program, but its contribution
tables (employment pension, health insurance and unemployment insurance
contributions) are provisional values that have not been checked against
official sources. Tax card handling is simplified, and there is no real
payment or Tulorekisteri connection behind the run. Do this: calculate, pay
and report wages with a payroll service or in Tulorekisteri's e-service, and
record the payroll voucher in aggountant:
[Record a sales invoice, payroll or another manual entry](../tasks/record-other-entries.md).

## Tulorekisteri and employees

- **Only tax-free travel allowances.** aggountant reports mileage allowances
  (311) and per diems (331) for one income earner and one payment at a time.
  The employer's separate report, replacement reports and cancellations are
  made in Tulorekisteri's e-service.
- **There is no command for the Tulorekisteri profile.** The agent writes it
  as the `vero-fi` skill instructs.
- **The employee register is not encrypted.** The command's help talks about
  an encrypted store, but the data is stored in plain text. The only
  protection is that the file is kept out of git. Keep the home directory and
  its backups for your own use only (`employee-store-plaintext-help`).
- **There is no safe way to enter personal data.** `employee show` shows the
  personal identity code and bank account number in plain text, and the
  details cannot be entered without showing them. Write the input to a file
  yourself, do not type the personal identity code into the conversation, and
  delete the file after the person has been added (`employee-pii-entry`).
- **The tax card withholding percentage is required** even for a person who
  is only reported travel allowances. It does not affect the travel allowance
  report.

## Year-end financial statements

Once all the year's months are closed, the year-end run (`close-year`)
calculates depreciation and accruals. The run records them, and the
corrections you give it, in the first open period of the next fiscal year,
because the year's months are already closed, and attributes them to the
closing fiscal year. So they show in the closing year's income statement and
balance sheet, not in the next year's result, and the next year starts from
balances that include them. The run then goes through four approvals
(adjusting entries, balance sheet breakdowns, notes and statements) and seals
the fiscal year.

- **The approval steps do not show the statements.** The command's output
  tells you only the state of the steps. Once the adjusting entries are
  approved, the monthly report for the fiscal year's last month shows the
  income statement and balance sheet with the adjustments. Look at it before
  you approve the statements (`close-year-gates-show-content`).
- **Corrections are given when the run starts.** The run records them before
  the adjusting entries are approved, and only the run's own entries are
  attributed to the closing year. A voucher you record yourself in a period
  of the next year shows in the next year's result.
- **The official financial statements document** with the written part of the
  notes, the signatures, the filing with the trade register and the tax return
  are done outside aggountant.

Sealing the fiscal year cannot be undone. Do this:
[Prepare the financial statements](../tasks/year-end.md).

## Company home and setup

- **A wrong home path can create an empty home.** If a command hits a
  directory that holds no books, the program may create an empty voucher
  store there. The real books are not harmed, but the vouchers seem to have
  disappeared until you notice the mistake
  ([When something goes wrong](../troubleshooting.md#vouchers-seem-to-have-disappeared),
  `read-commands-create-home`).
- **There is no backup or export command.** A backup is an ordinary copy of
  the directory:
  [Back up and restore your books](../tasks/back-up-and-restore.md)
  (`home-backup-coverage`).
- **Importing several years of history is not ready.** The setup starts from
  opening balances (`historical-import`).
- **A test-environment variable can mark a return as filed.**
  `AGGOUNTANT_BROKER_SCRIPT` is meant only for testing the program. If it is
  set, `file-vat` can mark a period as filed even though nothing was sent.
  Make sure the variable is not set when you file a real return.

## See also

- [Is aggountant for you?](../start/is-it-for-you.md)
- [When something goes wrong](../troubleshooting.md)
