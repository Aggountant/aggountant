# What is ready and what is not

aggountant is at version 0.2 and already keeps the books of a few Finnish
companies that have no accounting firm. Features are added as these companies
meet new situations, so ordinary monthly work is in good shape and rarer
cases are filled in step by step. Commands and file formats may still change
between versions.

Below, by topic, is every limitation: what does not work, why, and what you
do instead. The identifier in parentheses is the issue that tracks the
limitation. The issues are in the project's
[GitHub repository](https://github.com/aggountant/aggountant) under `issues/`.

## Overview

These are ready for real bookkeeping:

- recording vouchers and archiving their attachments
- importing bank statements (camt.053 and camt.052)
- recording ordinary purchase receipts with their VAT treatment
- closing a month
- monthly reports
- the VAT return, directly to the Tax Administration or by hand in OmaVero
- reporting tax-free travel allowances to Tulorekisteri
- the employer's separate report to Tulorekisteri for months without wages
- the year-end entries and sealing the fiscal year with the year-end run.

The program also contains the payroll run, aggountant's own automatic receipt
search and consultation for unusual receipts, but they cannot yet be used for
real bookkeeping. They are described below.

## Installation

aggountant runs on macOS and Linux. There is a ready-made installation
package for Apple Silicon Macs and for Linux (arm64 and x86_64); on an Intel
Mac aggountant is compiled from source with the Rust toolchain. Windows has
not been tested. The
instructions are in
[Install aggountant and its skills](../start/install.md).

## Bank statement

- Every confirmed transaction needs an entry. If even one is missing
  (`MISSING_ASSIGNMENT`) or refers to an account that is not in the chart of
  accounts (`ACCOUNT_NOT_FOUND`), the whole statement is rejected and nothing
  is sealed. The payment row of a receipt already recorded needs an entry
  too: it is recorded
  [against the receipt's payable](../tasks/record-receipts.md#when-the-purchase-is-also-on-the-bank-statement).
- Only Holvi's statement has been tried. Other banks' camt files may work.
  Reading a statement shows whether its transactions take the opening balance
  to the closing balance; after the first import, also check that the bank
  account balance matches the statement.
- Sales VAT is not split out from the statement. A sale that includes VAT is
  recorded as a manual entry, and the customer's payment from the statement
  against trade receivables (`vat-sales-expansion`).
- The preview binds your approval only when the agent passes the preview's
  identifier to the seal. The seal then refuses anything that changed since
  the preview (`IMPORT_PREPARATION_CHANGED`). Without the identifier the seal
  is not checked against any preview.
- An empty statement imports successfully with no transactions, and the
  program does not mention it separately. If an import gives zero
  transactions, check that the file is the statement for the right month and
  the right account (`camt-empty-report-and-rename`).
- The program does not link a receipt to its payment row. They meet on the
  trade payables account 2700 (or 2740), but the program does not check that
  every receipt gets paid or that the same purchase has not also been
  recorded as an expense from the statement row. At the end of the month,
  check that the trade payables balance is only the sum of unpaid invoices.

## Receipts and entries

- Consultation for unusual receipts is not in use. The program has a route
  (`capture-receipt start`) in which only the details you approved would go
  out, but there is no connection to a consultation service behind it. The
  agent asks you or your bookkeeper.
- aggountant's own receipt search is not in use, because the `hunt-receipts`
  command is not connected to a real email or online shop account. The agent
  can look for receipts with its own tools wherever you have given it access.
  Otherwise searching is manual work.
- The receipt route does not handle partial deduction rights or imports. The
  agent tells you if a purchase is one of those, and you then ask your
  bookkeeper.
- There is no separate correction command. A correction is recorded as an
  ordinary manual entry: [Correct a wrong voucher](../tasks/correct-voucher.md).
- A single voucher's postings are visible only to the agent, because without
  `--json`, `voucher show` shows only the number and the hash. Ask the agent
  to show the voucher's postings.

## Fiscal year and periods

- A fiscal year other than the calendar year is closed monthly. Quarterly
  closing is available only for calendar fiscal years.
- You cannot close both months and quarters in the same year. Monthly is the
  recommended and tried option.
- Two fiscal years cannot end in the same calendar year. A change of fiscal
  year is made with one transition year of at most 18 months.
- A fiscal-year definition cannot be changed. It can be followed by a new
  one, but not in a way that would move recorded vouchers to another fiscal
  year.
- Depreciation is not calculated for a fiscal year that is not 12 months
  long. Calculate that year's depreciation yourself and give it to the
  year-end run as a correction, so it is attributed to the right year. Assets
  carried from the previous year are marked as removed for that year and
  given again, with their book values, in the next year-end run.

## VAT return

- aggountant does not pay the tax. When a direct return has tax to pay, the
  run ends once the return is filed and shows the amount, the account and the
  due date; pay the tax yourself with the OmaVero reference.
- The Tax Administration's bank account number is given every time: the
  agent gives the filing run the account number shown in OmaVero. The program
  does not know the payment reference (`tax-payment-references`).
- A replacement return does not pay. A filed return
  [is corrected with aggountant](../tasks/file-vat-direct.md#correct-a-return-that-has-been-filed),
  but the program does not calculate the difference to pay or to be refunded:
  it shows on the tax account in OmaVero. A replacement return has not yet
  been sent to the Tax Administration's production service.
- Fields 309–311 for EU sales and zero-rated sales do not come from the
  books. If your company has EU sales or exports, file the return in OmaVero
  and enter these fields yourself. aggountant does not make the EC sales list
  (yhteenvetoilmoitus) (`vat-zero-rated-sales-fields`).
- A period whose VAT is negative because of corrections cannot be filed with
  aggountant. One example is a month whose only sale is a credit note. The
  error misleadingly names the currency (`VAT_CURRENCY`). File the return in
  OmaVero (`vat-negative-period-filing`).
- A VAT posting that lacks its VAT code and is found after the close cannot
  be corrected into the return in aggountant, because the return is
  calculated from VAT-coded postings. Voucher validation warns about an
  uncoded VAT posting, and the VAT return preview shows it
  (`VAT_ACCOUNTS_UNRECONCILED`) already on the open month, when a correcting
  voucher still fixes it. A closed month cannot be reopened for correction,
  so then
  [file the return in OmaVero with corrected figures](../tasks/file-vat-omavero.md#if-something-goes-wrong).
- Direct filing also works for quarterly periods, but in real use it has
  been tried with monthly periods.
- aggountant does not prepare Ilmoitin.fi files.

## Payroll

Cash wages cannot yet be calculated, paid or reported with aggountant. The
payroll run (`run-payroll`) is in the program, but its contribution tables
(employment pension, health insurance and unemployment insurance
contributions) are provisional values that have not been checked against
official sources. Tax card handling is simplified, and there is no real
payment or Tulorekisteri connection behind the run.

Calculate, pay and report wages with a payroll service or in Tulorekisteri's
e-service, and record the payroll voucher in aggountant:
[Record a sales invoice, payroll or another manual entry](../tasks/record-other-entries.md).

## Tulorekisteri and employees

- aggountant reports only tax-free travel allowances to Tulorekisteri:
  mileage allowances (311) and per diems (331) for one income earner and one
  payment at a time. The
  [replacement report and the cancellation](../tasks/report-travel-allowances.md#correct-an-accepted-report)
  of an accepted report are also made with aggountant.
- aggountant files
  [the employer's separate report](../tasks/file-employer-report.md) only for
  months without wages: "No wages payable" and, for months in which only
  tax-free travel allowances were paid, a health insurance contribution of
  €0. It does not compute the contribution for a month with wages, so such a
  month is refused (`EMPLOYER_REPORT_WAGES_UNSUPPORTED`). File that month in
  the e-service and give the reference to the agent. Correcting pension
  insurance details and cancelling a report are also done in the e-service.
- The deadline shown for the separate report is always the 5th of the month,
  although a report due on a Saturday or a holiday may be filed on the next
  banking day.
- The employee register is not encrypted; the data is stored in plain text.
  The only protection is that the file is kept out of git. Keep the home
  directory and its backups for your own use only.
- There is no safe way to enter personal data. `employee show` shows the
  personal identity code and bank account number in plain text, and the
  details cannot be entered without showing them. Write the input to a file
  yourself, do not type the personal identity code into the conversation, and
  delete the file after the person has been added (`employee-pii-entry`).
- The tax card withholding percentage is required even for a person who is
  only reported travel allowances. It does not affect the travel allowance
  report.

## Year-end financial statements

Once all the year's months are closed, the year-end run (`close-year`)
calculates depreciation and accruals. Because the year's months are already
closed, the run records them, and the corrections you give it, in the first
open period of the next fiscal year and attributes them to the closing fiscal
year. So they show in the closing year's income statement and balance sheet,
and the next year starts from balances that include them. The run then goes
through four approvals (adjusting entries, balance sheet breakdowns, notes
and statements) and seals the fiscal year. The seal cannot be undone. The
steps are in [Prepare the financial statements](../tasks/year-end.md).

- Depreciation is always calculated at the asset class's maximum rate. A
  sealed year stores the assets and their book values, and the next year-end
  run depreciates them from there. If the company depreciates less in some
  year, or follows another plan, mark the asset as removed for that year,
  give its depreciation as a correction and give the asset again with its
  book value the next year.
- The run does not record the sale or scrapping of an asset. Record it
  yourself and mark the asset as removed.
- Corrections are given when the run starts. The run records them before the
  adjusting entries are approved, and only the run's own entries are
  attributed to the closing year. A voucher you record yourself in a period
  of the next year shows in the next year's result.
- The official financial statements document with the written part of the
  notes, the signatures, the filing with the trade register and the tax
  return are done outside aggountant.

## Company home and setup

- There is no backup or export command. A backup is an ordinary copy of the
  directory:
  [Back up and restore your books](../tasks/back-up-and-restore.md)
  (`home-backup-coverage`).
- Importing several years of history is not ready. The setup starts from
  opening balances, and the old system's history is kept as an archive:
  [Move your books from another program](../start/move-from-another-program.md)
  (`historical-import`).
- [Upgrading the home](../tasks/upgrade.md) does not fix or replace a file it
  left as it was. If aggountant brings no VAT roles or settlement roles for
  the home's chart of accounts, the upgrade cannot add them either
  (`unavailable`). Do not copy files by hand; ask for help in the project's
  [issue tracker](https://github.com/aggountant/aggountant/issues).
- A company's own chart of accounts cannot be imported. The books use the
  default chart, and the old system's accounts are mapped onto it in the
  opening voucher (`sie4-chart-import`).
- A test-environment variable can mark a return as filed.
  `AGGOUNTANT_BROKER_SCRIPT` is meant only for testing the program. If it is
  set, `file-vat` can mark a period as filed even though nothing was sent.
  Such output carries the field `test_seam`. Make sure the variable is not
  set when you file a real return.

## See also

- [Is aggountant for you?](../start/is-it-for-you.md)
- [When something goes wrong](../troubleshooting.md)
