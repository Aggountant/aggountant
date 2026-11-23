# Prepare the financial statements

Once every month of the fiscal year is closed, you gather the material for
whoever prepares the financial statements (tilinpäätös) and record the
depreciation, accruals and corrections with the year-end run. The material is
the monthly reports and the sealed vouchers with their attachments. For now
the official financial statements document is drawn up outside aggountant,
with an accounting firm or a bookkeeper, or by yourself.

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
   you can give to the preparer. Tell the preparer the opening balances and
   anything else they need to know, for example purchases that will be
   depreciated.

4. **Agree with the preparer what goes into the year-end run.**
   Depreciation, accruals and corrections belong to the year that has ended,
   whose months are already closed. The year-end run (`close-year`) records
   them in the first open period of the next year and attributes them to the
   year that has ended, so they show in the right year's result. Agree
   whether the preparer gives you the assets to depreciate, the accruals and
   the corrections for the run.

   In the first financial statements you do with aggountant, you give every
   asset and its book value at the start of the year, taken from the
   previous financial statements. When a year is sealed, aggountant stores
   the assets and their book values at the end of the year, and the next
   year's run continues from them. From then on you give only new
   acquisitions and say which assets were sold or taken out of use.

5. **Check the depreciation calculation before the run.**

   > Show the depreciation calculation for 2026: one machine of 2,000.00
   > euros in machinery and equipment.

   The agent shows the assets, the method, the rate and the depreciation.
   Nothing is recorded yet.

6. **Start the run and approve its steps.**

   > Run the year-end close for 2026: one machine of 2,000.00 euros in
   > machinery and equipment, no accruals and no corrections.

   The run asks for the approvals in turn, and before each one the agent
   shows you what you are approving:

   1. the adjusting entries: the calculated depreciation, the accruals and
      the corrections
   2. the balance sheet breakdowns (tase-erittelyt)
   3. the notes (liitetiedot): the depreciation methods and rates
   4. the financial statements: the income statement and the balance sheet

   An approval covers exactly the content the agent showed you. The last
   approval seals the fiscal year, and it cannot be undone. The run then
   writes the approved statements, balance sheet breakdowns and notes to
   `archive/<year>/tilinpaatos.json` in the company home.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant period show --period 2026-12
aggountant fiscal-year show --year 2026
aggountant voucher verify
aggountant voucher list --posting-from 2026-01-01 --posting-to 2026-12-31
aggountant report statements --period 2026-12 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-12.html
aggountant close-year start --dry-run < close-year-2026.json
aggountant close-year start < close-year-2026.json
aggountant close-year seal-correction --run-id cy-… --correction <id>
aggountant close-year show --run-id cy-…
aggountant close-year answer --run-id cy-… --gate ApproveAdjustments --answer approve
aggountant close-year show --run-id cy-…
aggountant close-year answer --run-id cy-… --gate ApproveTaseErittelyt --answer approve
aggountant close-year show --run-id cy-…
aggountant close-year answer --run-id cy-… --gate ApproveLiitetiedot --answer approve
aggountant close-year show --run-id cy-…
aggountant close-year answer --run-id cy-… --gate ApproveStatements --answer approve
```
</details>

## Done when

- The preparer has the year's reports, vouchers and attachments.
- The year's depreciation, accruals and corrections are recorded with the
  year-end run and the fiscal year is `sealed`, or you have agreed otherwise
  with the preparer.

## Limitations

- The run attributes only its own entries to the year that has ended. A
  voucher you record yourself in a period of the next year shows in the next
  year's result.
- Drawing up and signing the official financial statements, filing them with
  the trade register and the company's tax return are done outside
  aggountant.

More detail: [What is ready and what is not](../reference/status.md#year-end-financial-statements).

## If something goes wrong

- If the run is interrupted, the agent can show the open step again later.
- If the content has changed since the step opened, the agent cannot show it
  and you cannot approve it. Reject the step and start the run again.
- If you give an asset carried from the previous year a figure other than
  the sealed year's, the run refuses.
- If writing `tilinpaatos.json` fails, the agent tells you and writes the
  file again by continuing the run. The books are correct either way.
- `PERIOD_NOT_SEALED`: the monthly report for the next fiscal year's months
  is refused until this year is sealed, because its balance sheet starts
  from this year's closing balances. Recording, closing months and filing
  VAT in the new year do not wait. More on the page
  [See how the month went](monthly-report.md#if-something-goes-wrong).
- If an earlier, unfinished year-end run already recorded some entries,
  include the same assets, accruals and corrections with the same figures in
  the new run. Otherwise the run refuses to start or to seal the year and
  names the missing entries. What you approve has to cover every year-end
  entry in the books, including one the earlier run recorded in the new year
  but did not yet attribute to the year being closed, which would otherwise
  stay in the new year's result. If such an entry was wrong, include it all
  the same and reverse it with a correction in the new run. Once one run has
  sealed the year, any other open run for it can only be rejected.

## See also

- [For bookkeepers](../concepts/for-bookkeepers.md)
- [Periods, closing and the fiscal year](../concepts/periods.md)
- [See how the month went](monthly-report.md)
