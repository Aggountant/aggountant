# See how the month went

The monthly report has the month's income statement, the result accumulated
since the start of the fiscal year, and the balance sheet at the end of the
month. The report tells you how the company is doing, and it is the best
check before closing the month.

## Before you start

- The month's transactions are recorded. You can also make the report in the
  middle of the month, when it shows what has been recorded so far.

## How to do it

1. **Ask the agent.**

   > Make a monthly report for March in HTML.

2. **The agent makes the report** from the sealed vouchers and tells you which
   file it was saved to. Open the file in a browser. The browser's print
   function turns it into a PDF. The report is in Finnish, like the chart of
   accounts.

3. **Check in the report:**
   - **Pankkitili** (bank account) equals the statement's closing balance. If
     it differs, a transaction is missing or recorded twice.
   - **The balance sheet totals** are the same on both sides.
   - **The month's result** looks reasonable. If an expense or income item is
     unexpectedly large, ask the agent to show its vouchers.

At the top of the report it says "Kattavuus: varmistamaton" (completeness:
unverified). The program cannot know whether all the material has been
recorded. You and the agent judge that against the sources.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant report statements --period 2026-03 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-03.html
```

`--format native-json` produces the same content in machine-readable form.
`--history-start` says from which date the bookkeeping history is in
aggountant, and `--limitation` adds a note to the report, for example
"Opening balances from the previous system". The program never writes over an
existing file, so a new report gets a new name.
</details>

## Done when

- The program replies `wrote html report for 2026-03 to …`, and the bank
  account balance matches the statement.

## If something goes wrong

- **`OUTPUT_EXISTS`.** A file with the same name already exists. Give a new
  name.
- **The bank account balance does not match.**
  [When something goes wrong](../troubleshooting.md#the-bank-balance-does-not-match-the-statement)

## See also

- [Close the month](close-month.md)
- [Prepare the financial statements](year-end.md)
