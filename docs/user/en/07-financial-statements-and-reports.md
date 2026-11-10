# 7. Financial statements, reports and fiscal-year status

## The monthly report

The monthly report contains the month's income statement, the result
accumulated since the start of the fiscal year, and the balance sheet at the end
of the month:

```sh
aggountant report statements --period 2026-03 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-03.html
```

- `--format html` produces a report you open in a browser and can print to PDF
  with the browser's print function. `--format native-json` produces the same
  content in machine-readable form.
- The program never overwrites an existing file (`OUTPUT_EXISTS`). Give a new
  report a new name.
- The report is computed from sealed vouchers. It can also be produced for a
  month that has not been closed yet. It then shows what has been booked so
  far.
- The report always says "completeness unverified". The program cannot know
  whether all the material has been booked. You and the agent judge that
  against the sources. With `--history-start` you can state the date from which
  the bookkeeping history is in aggountant, and with `--limitation` you can add
  a note about a limitation to the report, for example "Opening balances from
  the previous system".

The report is also a good check before closing the period: the bank account
balance must match the statement's closing balance, and the balance sheet must
balance.

## Period and fiscal-year status

```sh
aggountant period show --period 2026-03
aggountant fiscal-year show --year 2026
```

A period's state is `open`, `closing` or `sealed` (chapter 4), and the output
shows whether the period's VAT return has been filed. A fiscal year's state is
`open` or `sealed`. A sealed fiscal year also carries a checksum of its
financial statements.

You can check the integrity of the voucher chain at any time:

```sh
aggountant voucher verify
```

You can look up a single voucher by its number (`aggountant voucher show 12`).
Vouchers can be searched by posting date or by attachment (`aggountant voucher
list --posting-from 2026-03-01 --posting-to 2026-03-31`).

## The year-end close (tilinpäätös)

The tilinpäätös is the Finnish statutory year-end close: the income statement,
balance sheet and notes drawn up for the fiscal year. The program has a
year-end close run (`close-year`). Once every month of the year has been
closed, it computes depreciation and accruals, builds the income statement and
the balance sheet, goes through four approvals (adjusting entries, balance
sheet specifications, notes and statements) and finally seals the fiscal year.

**Do not make a real year-end close with this run yet.** In this version it has
two shortcomings that make the result wrong or the approval blind:

- **Year-end depreciation and accruals are missing from the closing year's
  statements.** The run books them into the first open period of the following
  year, because the year's months are already closed. They are not yet
  attributed back to the closing year's financial statements. So the income
  statement and the balance sheet do not include the year's depreciation.
- **The approval steps do not show the statements.** The command's output only
  shows the status of each step. You would be approving the income statement
  and the balance sheet without seeing them.

Sealing a fiscal year is irreversible. For now, make the year-end close with an
accounting firm or by hand. The monthly reports and the sealed vouchers give
you the material for it. Drawing up and signing the official financial
statements document, filing them with the Trade Register, and the company's
income tax return are also tasks outside aggountant.
