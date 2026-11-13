# Prepare the financial statements

For now the financial statements (tilinpäätös) are prepared outside
aggountant, with an accounting firm or a bookkeeper, or by yourself.
aggountant gives you the material for them: the monthly reports and the
sealed vouchers with their attachments. On this page you gather the material,
hand it over and record the year-end entries.

## Before you start

- Every month of the year is recorded, closed and its VAT filed.
- You know who prepares the financial statements and in what form they want
  the material.

## How to do it

1. **Check the state of the year.**

   > Show the state of the months of 2026 and check the voucher chain.

   The agent shows each month's state and checks the chain. Every month must
   be `sealed` and `vat_filed true`.

2. **Ask for the report for the fiscal year's last month.**

   > Make a monthly report for December in HTML.

   The report has the balance sheet at the end of the fiscal year's last
   month and the income statement since the start of the fiscal year.

3. **Gather the material to hand over.** Ask the agent to put the monthly
   reports, the list of vouchers and the archived attachments into a folder
   you can give to whoever prepares the financial statements. Also tell them
   about the opening balances and anything else they need to know, for
   example purchases that will be depreciated.

4. **Record the year-end entries with the year-end run.** Depreciation,
   accruals and corrections belong to the year that has ended, whose months
   are already closed. The year-end run (`close-year`) records them in the
   first open period of the next year and attributes them to the year that
   has ended, so they show in the right year's result. Recorded as ordinary
   vouchers in a period of the next year, they would show in the next year's
   result. Agree with whoever prepares the financial statements whether they
   give you the assets to depreciate, the accruals and the corrections for
   the run.

   > Run the year-end close for 2026: one machine of 2,000.00 euros in
   > machinery and equipment, no accruals and no corrections.

   The agent shows the calculated depreciation. Once you approve the
   adjusting entries, ask for the December monthly report again: its income
   statement and balance sheet include the adjustments. Approve the
   statements only after you have looked at it. The last approval seals the
   fiscal year, and it cannot be undone.

Drawing up and signing the official financial statements, filing them with
the trade register and the company's tax return are also done outside
aggountant.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant period show --period 2026-12
aggountant fiscal-year show --year 2026
aggountant voucher verify
aggountant voucher list --posting-from 2026-01-01 --posting-to 2026-12-31
aggountant report statements --period 2026-12 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-12.html
aggountant close-year start < close-year-2026.json
aggountant close-year seal-correction --run-id cy-… --correction <id>
aggountant close-year answer --run-id cy-… --gate ApproveAdjustments --answer approve
aggountant close-year answer --run-id cy-… --gate ApproveStatements --answer approve
```
</details>

## Done when

- Whoever prepares the financial statements has the year's reports, vouchers
  and attachments.
- The year's depreciation, accruals and corrections are recorded with the
  year-end run and the fiscal year is `sealed`, or you have agreed otherwise
  with whoever prepares the financial statements.

## Limitations

The year-end run's approval steps do not show the statements you approve, so
look at the report for the fiscal year's last month before you approve the
statements. The run attributes only its own entries to the year that has
ended. Sealing the fiscal year cannot be undone.
More detail: [What is ready and what is not](../reference/status.md#year-end-financial-statements).

## See also

- [For bookkeepers](../concepts/for-bookkeepers.md)
- [Periods, closing and the fiscal year](../concepts/periods.md)
- [See how the month went](monthly-report.md)
