# Prepare the financial statements

For now the financial statements (tilinpäätös) are prepared outside
aggountant, with an accounting firm or a bookkeeper, or by yourself.
aggountant gives you the material for them: the monthly reports and the
sealed vouchers with their attachments. On this page you gather the material
and hand it over.

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

4. **Whoever prepares the financial statements** draws up the depreciation,
   accruals, the statements and the notes. These entries belong to the year
   that has ended, whose months are already closed, and aggountant does not
   yet have a way to record them there. If they were recorded in the first
   month of the next year, they would show in the wrong year's result. Agree
   with them on how these entries are taken into account.

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
```
</details>

## Done when

- Whoever prepares the financial statements has the year's reports, vouchers
  and attachments.
- You have agreed with them how the year-end entries are taken into account.

## Limitations

The program has a year-end run (`close-year`), but **do not use it for your
real financial statements**. In it, the year-end depreciation and accruals
are missing from the closing year's statements, and the approval steps do not
show the statements you are approving. Sealing the fiscal year cannot be
undone.
More detail: [What is ready and what is not](../reference/status.md#year-end-financial-statements).

## See also

- [For bookkeepers](../concepts/for-bookkeepers.md)
- [Periods, closing and the fiscal year](../concepts/periods.md)
- [See how the month went](monthly-report.md)
