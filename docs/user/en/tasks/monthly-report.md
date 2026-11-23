# See how the month went

The monthly report has the month's income statement, the result accumulated
since the start of the fiscal year, and the balance sheet at the end of the
month. Use it to check the month before you close it.

## Before you start

- The month's transactions are recorded. You can also make the report in the
  middle of the month, and it then shows what has been recorded so far.
- If the month belongs to a new fiscal year, the previous fiscal year is
  sealed with the year-end close. Before that the program does not make the
  report ([`PERIOD_NOT_SEALED`](#if-something-goes-wrong)).

## How to do it

1. **Ask the agent.**

   > Make a monthly report for March in HTML.

2. **The agent makes the report** from the sealed vouchers and tells you which
   file it was saved to. Open the file in a browser. The browser's print
   function turns it into a PDF. The report is in Finnish, like the chart of
   accounts.

3. **Check the report.**
   - The balance of Pankkitili (bank account) equals the statement's closing
     balance. If it differs, a transaction is missing or recorded twice.
   - Ostovelat (trade payables) holds only the sum of invoices not yet paid.
     If an amount already paid stays there, the same purchase was probably
     recorded as an expense both from the receipt and from the statement row.
     See
     [When the purchase is also on the bank statement](record-receipts.md#when-the-purchase-is-also-on-the-bank-statement).
   - The balance sheet totals are the same on both sides.
   - The month's result looks reasonable. If an expense or income item is
     unexpectedly large, ask the agent to show its vouchers.

At the top of the report it says "Kattavuus: varmistamaton" (completeness:
unverified), because the program cannot know whether all the material has
been recorded. You and the agent judge that against the sources.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant report statements --period 2026-03 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-03.html
```

`--format native-json` produces the same content in machine-readable form.
`--history-start` says from which date the bookkeeping history is in
aggountant, and `--limitation` adds a note to the report, for example
"Opening balances from the previous system".

The program does not write over an existing file, so a new report gets a new
name. Without `--output` the program prints the report and does not save it,
as native JSON unless `--format html` is given. The agent uses that for a
quick look at the month.
</details>

## Done when

- The program replies `wrote html report for 2026-03 to …`, and the bank
  account balance matches the statement.

## If something goes wrong

- `OUTPUT_EXISTS`: a file with the same name already exists. Give a new name.
- `PERIOD_NOT_SEALED` for a month of the new fiscal year: the previous fiscal
  year is not sealed yet. The balance sheet starts from the previous fiscal
  year's closing balances, and they are final only once the year-end close
  has sealed that year
  ([Prepare the financial statements](year-end.md)). In a calendar-year
  company you meet this every January until the year-end close is done.
  Meanwhile you record, close and file VAT for the new year's months as usual.
  The month close then shows only this year's movements and says so. The
  error does not apply to the first year you keep in aggountant, whether the
  company is new or the books were opened with
  [opening balances](../start/set-up-company.md).
- If the bank account balance does not match, see
  [The bank balance does not match the statement](../troubleshooting.md#the-bank-balance-does-not-match-the-statement).

## See also

- [Close the month](close-month.md)
- [Prepare the financial statements](year-end.md)
