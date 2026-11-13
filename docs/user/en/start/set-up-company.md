# Set up your company's books

On this page you set up your company's real books: the company home, the
company's basic details, the opening balances and a description of where your
receipts and bank statements come from. The agent goes through the setup with
you using the `aggountant-init` skill. You do the setup once, and an
unfinished setup can be continued later.

## Before you start

- aggountant and its skills are installed
  ([Install aggountant and its skills](install.md)).
- You have done the practice month or otherwise know how working with the
  agent goes ([Practise your first month](practice-month.md)).
- You have the company's official name, Business ID (Y-tunnus) and fiscal year
  at hand, preferably from the trade register extract (kaupparekisteriote).
- If the company is moving from another bookkeeping program: the old system's
  balance list (balance sheet) for the last day the old system covers.

## How to do it

1. **Ask the agent to begin.**

   > Let's set up Esimerkki Oy's books in aggountant. The home goes in
   > ~/books/esimerkki-oy.

2. **The agent sets up the home.** It creates the directory, the voucher
   store, the default chart of accounts (`fi-general-2026`, 98 accounts) and a
   `.gitignore` file. If the home already holds books, the program refuses
   (`ALREADY_INITIALIZED`) and leaves them alone. Setting up the home does not
   ask for the company name, Business ID or fiscal year; those are collected
   in the next step. The agent recommends making the home a git repository and
   setting it as the default home, so that commands find it without a path.
   What is stored in the home, and why git is not a backup, is explained in
   [The company home: what is stored where](../concepts/company-home.md).

3. **Give the basic details.** The agent asks for the company's official
   name, Business ID, fiscal year, the date the aggountant books start from,
   and whether to continue the old system's chart of accounts. It saves your
   answers with their sources in the company knowledge store. Give a trade
   register extract or a report from the old system as the source, not an
   answer from memory: this information guides the agent's work later. If the
   fiscal year is not the calendar year, the agent also records it in the
   books (`fiscal-year define`) before the first voucher. It defines the
   fiscal year the first voucher falls in; later years follow on their own.

4. **Check and approve the opening balances.** If the company comes from
   another system, the agent records the old system's balances as one
   ordinary voucher and attaches the balance list to it. Before you approve,
   check:
   - The voucher is dated **the old system's last day**. If it is dated the
     first day of the new books, the whole earlier result shows up as the
     first month's result.

     When the company moves in the middle of a fiscal year, the voucher falls
     in the same fiscal year as the new transactions. It holds the year's
     income and expense accounts as well as the balance-sheet accounts, so
     the result for the year so far carries on correctly.

     When the company moves at the turn of a fiscal year (for example, the old
     system covers 2025 and aggountant starts on 1 January 2026), the voucher
     is dated the last day of the previous fiscal year, 31 December 2025. It
     then holds only balance-sheet accounts: the previous year's result has
     been transferred to equity. aggountant uses such a voucher as the
     opening balances of the new fiscal year and does not require the old
     system's year to be closed. Reports refuse (`PERIOD_NOT_SEALED`) if the
     previous fiscal year holds anything else: income or expense accounts,
     an earlier date, or a closed period.

     If the company moves to aggountant at the start of a fiscal year that
     does not begin on 1 January, the opening balances are dated the first
     day of that fiscal year, because the books have no fiscal year before
     it. At the start of a fiscal year the balances contain only
     balance-sheet accounts, so this does not move any result.
   - Every balance matches the balance list, and the voucher balances.

   A new company has no opening balances. Its first transaction is usually the
   share capital payment, as in the practice month.

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

7. **The agent makes a report for the first month.** Check it before you
   record the month's transactions (below, "Done when").

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
aggountant voucher validate < opening-balances.json
aggountant voucher seal < opening-balances-seal.json
aggountant report statements --period 2026-03 --company "Esimerkki Oy" \
  --format html --output esimerkki-2026-03.html \
  --history-start 2026-03-01 --limitation "Opening balances from the previous system"
```

The default home is set in the configuration file whose location
`config path` prints. The file is JSON: `{"home": "/path/to/home"}`.
</details>

## Done when

- `aggountant config show` shows your company's home.
- In the first month's report, before any new transactions, the month's
  result is zero, the balance sheet matches the old system's report and the
  bank account balance matches the bank statement.
- There is an `AGENTS.md` at the root of the home that names the sources of
  your material.

## Limitations

- Importing several years of history from an old system is not ready. The
  setup starts from opening balances.
- `init` does not repair or update an old home, and there is no separate
  upgrade command.

More detail: [What is ready and what is not](../reference/status.md).

## If something goes wrong

- **`ALREADY_INITIALIZED`.** The home already exists. Nothing was changed.
- **Vouchers seem to have disappeared, or `MASTERDATA_NOT_INITIALIZED`.** The
  command hit the wrong home:
  [When something goes wrong](../troubleshooting.md#vouchers-seem-to-have-disappeared).

## See also

- [The company home: what is stored where](../concepts/company-home.md)
- [Do a month's bookkeeping from start to finish](../tasks/monthly-routine.md)
- [Set up direct VAT filing](../tasks/set-up-vero-api.md)
