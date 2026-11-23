# Set up your company's books

When you start your company's real books in aggountant, the agent sets up
the company home with you, saves the company's basic details and records the
opening balances. Together you also write down where your receipts and bank
statements come from and what is left waiting. The agent uses the
`aggountant-init` skill for this. You do the setup once, and an unfinished
setup can be continued later.

## Before you start

- You have [installed aggountant and its skills](install.md).
- You have done the [practice month](practice-month.md) or otherwise know
  how working with the agent goes.
- You have the company's official name, Business ID (Y-tunnus) and fiscal year
  at hand, preferably from the trade register extract (kaupparekisteriote).
- If the company is moving from another bookkeeping program, you have the
  old system's list of accounts and its balance list (balance sheet) for the
  last day the old system covers.
  [Move your books from another program](move-from-another-program.md) says
  what else to export from the old program and how.

## How to do it

1. **Ask the agent to begin.**

   > Let's set up Esimerkki Oy's books in aggountant. The home goes in
   > ~/books/esimerkki-oy.

2. **The agent sets up the home.** It creates the directory, the voucher
   store, the default chart of accounts (`fi-general-2026`, 98 accounts) and a
   `.gitignore` file. The agent recommends making the home a git repository
   and setting it as the default home, so that commands find it without a
   path. [The company home: what is stored where](../concepts/company-home.md)
   explains what is stored in the home and why git is not a backup.

3. **Give the basic details.** The agent asks for the company's official
   name, Business ID, fiscal year and the date the aggountant books start
   from. It saves your answers with their sources in the company knowledge
   store and relies on them in its later work, so take the answers from the
   trade register extract or a report from the old system rather than from
   memory. If the fiscal year is not the calendar year, the agent also
   records in the books, before the first voucher, the fiscal year that the
   first voucher falls in. Later fiscal years follow on their own.

4. **Check and approve the opening balances.** A new company has no opening
   balances, and its first transaction is usually the share capital payment,
   as in the practice month. For a company that comes from another system,
   the agent records the old system's balances as one ordinary voucher and
   attaches the balance list to it. The books use aggountant's default chart
   of accounts, so the agent maps the old system's accounts onto it and saves
   the mapping in the home. Several old accounts may merge into one. The
   agent tells you which distinctions are lost that way, and the mapping file
   shows where each balance came from. Before you approve, check:
   - Every balance matches the balance list, and the voucher balances.
   - The voucher is dated the old system's last day. If it is dated the
     first day of the new books, the whole earlier result shows up as the
     first month's result.

   What the voucher holds depends on when the company moves:
   - When the company moves in the middle of a fiscal year, the voucher falls
     in the same fiscal year as the new transactions. It holds the year's
     income and expense accounts as well as the balance-sheet accounts, so
     the result for the year so far carries on correctly.
   - When the company moves at the turn of a fiscal year, the voucher is
     dated the last day of the previous fiscal year. If the old system covers
     2025 and aggountant starts on 1 January 2026, that day is 31 December
     2025. The voucher holds only balance-sheet accounts, because the
     previous year's result has been transferred to equity. aggountant uses
     the voucher as the opening balances of the new fiscal year and does not
     require the old system's year to be closed.
   - If the new fiscal year does not begin on 1 January, the opening
     balances are, as an exception, dated the first day of that fiscal year,
     because the books have no fiscal year before it. At the start of a
     fiscal year the balances contain only balance-sheet accounts, so this
     does not move any result.

5. **Tell the agent about open matters** that the balance list does not show.
   The agent asks whether company bills have been paid from your own account,
   whether the company owes anything on its tax account or has a payment
   arrangement, and whether there are open sales or purchase invoices.

6. **Describe where the material comes from.** Together with you, the agent
   writes an `AGENTS.md` file at the root of the home, where you say in your
   own words:
   - where receipts and invoices arrive (email, cloud folder, local folder,
     the person responsible)
   - where drafts of expense and travel claims are kept
   - where the mileage log (ajopäiväkirja) is kept
   - where processed original material is archived, and when it may be moved
     away from its source
   - which folders of the home belong in git and which do not.

   Without this file the agent has to ask every time or guess. **Do not put
   passwords, credentials or keys in the file.**

7. **The agent writes a `TODO.md` file at the root of the home**, where it
   gathers what is left waiting: what is expected from others (for example
   the receipt for a registration fee the founder paid from their own
   account), which months are open and what closing them still needs, and
   deadlines (for example the VAT return and the first fiscal year's
   financial statements). Check that the list matches your own picture. The
   agent reads the file at the start of every session and updates it when an
   item is settled, as
   [The home's TODO.md](../concepts/company-home.md#the-homes-todomd)
   describes.

8. **The agent makes a report for the first month.** Check the figures
   listed under [Done when](#done-when) in it before you record the month's
   transactions.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant --home ~/books/esimerkki-oy init
aggountant config path
aggountant config show
aggountant chart list
aggountant knowledge company add < fact.json
aggountant fiscal-year define --start 2026-07-01 --end 2027-06-30
aggountant voucher draft < opening-balances.json
aggountant voucher validate < opening-balances-seal.json
aggountant voucher seal < opening-balances-seal.json
aggountant report statements --period 2026-03 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-03.html \
  --history-start 2026-03-01 --limitation "Opening balances from the previous system"
```

The default home is set in the configuration file whose location
`config path` prints. The file is JSON: `{"home": "/path/to/home"}`.

`fiscal-year define` is run only if the fiscal year is not the calendar
year.
</details>

## Done when

- `aggountant config show` shows your company's home.
- In the first month's report, before any new transactions, the month's
  result is zero, the balance sheet matches the old system's report and the
  bank account balance matches the bank statement.
- There is an `AGENTS.md` at the root of the home that names the sources of
  your material, and a `TODO.md` that says what is waiting.

## Limitations

- Several years of history cannot yet be imported from an old system, so the
  setup starts from opening balances.
- The old system's chart of accounts cannot be imported, so the books use
  the default chart.
- `init` does not repair or update an old home.
  [Upgrading aggountant](../tasks/upgrade.md) does that.

All the limitations of the setup are on the page
[What is ready and what is not](../reference/status.md#company-home-and-setup).

## If something goes wrong

- `ALREADY_INITIALIZED`: the home already holds books. The program refused
  to set up the home and changed nothing.
- `PERIOD_NOT_SEALED` in a report for the new fiscal year: the previous
  fiscal year holds something other than the opening balances from the turn
  of the year, that is, income or expense accounts, an earlier date or a
  closed period.
- If vouchers seem to have disappeared or a command stops with
  `HOME_NOT_INITIALIZED`, the command hit the wrong home. The
  troubleshooting section
  [Vouchers seem to have disappeared](../troubleshooting.md#vouchers-seem-to-have-disappeared)
  shows how to find the right one.

## See also

- [The company home: what is stored where](../concepts/company-home.md)
- [Do a month's bookkeeping from start to finish](../tasks/monthly-routine.md)
- [Set up direct VAT filing](../tasks/set-up-vero-api.md)
