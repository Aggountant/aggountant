# Move your books from another program

When a company moves to aggountant from another bookkeeping program,
aggountant starts from the opening balances on the cutover date. The old
system's history is exported from the old program and the bank into an
archive, which you keep for as long as the law requires. Collect the material
before you give up the old program, because afterwards you may no longer be
able to get it.

## Before you start

- You have decided the cutover date: the last day the old system covers. The
  turn of a fiscal year is simplest. The dating rules are in
  [Set up your company's books](set-up-company.md).
- You can sign in to the old program and the bank with your own credentials,
  and the subscription stays active until the archive has been checked.
- If an accounting firm keeps your books in the old program, you have agreed
  with them who exports the material. Some exports may not be visible with
  your access rights.

## What to collect

Under the Accounting Act (kirjanpitolaki), the financial statements
(tilinpäätös), the management report (toimintakertomus) where one was
prepared, the books (journal and general ledger), the list of accounts and the
list of books and records are kept for at least 10 years from the end of the
fiscal year. Vouchers, business correspondence and other accounting records
are kept for at least six years from the end of the calendar year in which the
fiscal year ended (Accounting Act, chapter 2, section 10). So collect every
fiscal year whose retention period is still running.

| Material | What it is needed for |
|---|---|
| list of accounts (account numbers, names, VAT codes) | mapping the old accounts onto aggountant's chart in the opening balances, and the archive |
| balance list or balance sheet for the cutover date, and the income statement too if you move mid-year | the opening balance voucher |
| journal and general ledger for every fiscal year | the archive, and checking that vouchers and totals agree |
| financial statements, management reports and balance sheet specifications (tase-erittely) | the archive, and comparatives for the next financial statements |
| vouchers and all their attachments | the archive |
| purchase and sales invoices in every state, including open ones | working out which invoices are open on the cutover date, and the archive |
| comments and notes on vouchers and invoices | the archive, because they often say why something was booked the way it was |
| bank statements in electronic form (camt.053 or camt.052) and as PDF | those after the cutover date are imported into aggountant, earlier ones go to the archive |
| VAT returns and other filings already made | the archive, although you can also see them in OmaVero later |

## How to do it

1. **Sign in to the old program and the bank yourself** and complete the
   multi-factor authentication. Do not give the agent your password. Neither
   of you copies cookies, sign-in tokens or links that carry a sign-in into
   files, notes or the conversation. If the session expires, sign in again.

2. **Ask the agent to make an inventory.**

   > We are moving Esimerkki Oy's books to aggountant. The old system covers
   > up to 31 December 2025. Let's first list everything we need to export
   > from the old program and the bank.

   The agent writes a list of the material it expects: fiscal years, reports,
   voucher series and their numbers, invoice counts, statement periods and
   attachments. It takes the figures from the old system, for example the
   last number in the voucher list. The collection is done only when every
   item on the list is saved or marked missing with a reason.

3. **The agent exports the material.** It uses the program's own export or
   API first, because those keep working when the service's interface
   changes. What is only available in the browser the agent can fetch from
   your signed-in browser, if its environment can control a browser.
   aggountant itself has no tools for operating the old program or the bank.
   The agent only reads the old system: it exports and downloads, but saves,
   approves, changes or deletes nothing, not even on a comments page.

4. **The agent checks every file.** It reads the company and the period from
   the file's content, because the date in a file name can be the export
   time. It reads the file format from the content too, because a file named
   XLS can be XLSX. The agent opens every file, because an interrupted
   download leaves an incomplete file behind. In the books it checks that
   every voucher's debits and credits agree and that the yearly totals match
   the financial statements.

5. **The agent saves the archive.** It keeps the original exports unchanged
   and puts the tables and check reports derived from them in a folder of
   their own, so that you can always go back to the original. For every file
   it records the size, the SHA-256 checksum, where the file came from and
   when it was fetched, and collects the checksums into a `SHA256SUMS` file.
   It marks an item done only when its files match their checksums. Finally
   the agent copies the archive to a second place and checks every file in
   both copies against the checksums.

   Keep the archive out of git, because it is real financial data and large,
   and nothing can be removed from git history. Keep it
   [within your backups](../tasks/back-up-and-restore.md) and say in the
   home's `AGENTS.md` where it is. Then the agent can find an old voucher
   when you ask about one.

6. **Set up the books.** The balance list and the list of accounts become
   [the opening balance voucher](set-up-company.md). Statements after the
   cutover date are
   [imported as every month](../tasks/import-bank-statement.md), and receipts
   and invoices are [recorded as usual](../tasks/record-receipts.md).

   Import the first statement starting from the day after the cutover date.
   Earlier transactions are already in the opening balances, and the program
   requires a booking for every transaction on a statement, so a statement
   that spans the cutover date cannot be imported without booking the same
   transactions twice.

   An invoice dated before the cutover date but paid after it may already be
   in the opening balances as a trade payable. The agent checks this against
   the balance list so that the same invoice is not booked twice.

<details>
<summary>Commands the agent runs</summary>

The agent checks the archive with ordinary tools, not with aggountant:

```sh
cd archive-esimerkki-oy
sha256sum -c SHA256SUMS
```

The setup commands are on the pages
[Set up your company's books](set-up-company.md) and
[Import the bank statement](../tasks/import-bank-statement.md).
</details>

## Done when

- Every item on the inventory is saved or marked missing with a reason.
- `sha256sum -c SHA256SUMS` accepts every file in both copies.
- The opening balance voucher
  [agrees with the balance list](set-up-company.md#done-when).
- The home's `AGENTS.md` says where the old system's archive is.

## Notes for specific programs

These notes are from 2026. Services change, so check the current interface
and the service's own help before relying on them.

### Briox

- The contents of the sections "Arkisto" (archive) and "Liitetyt asiakirjat"
  (attached documents) can be exported with Briox's own export
  ([Arkiston vienti/lataaminen](https://help.briox.fi/hc/fi/articles/360029163831-Arkiston-vienti-lataaminen),
  in Finnish). The menu may not be visible with every access level. If it is
  not, ask the accounting firm or the administrator for the export. Otherwise
  you can get voucher attachments only one at a time from the voucher's
  attachment viewer in the browser, and for image attachments the viewer
  shows a display version that may not be the file originally uploaded.
- Reports are available as PDF and Excel: journal, general ledger, list of
  accounts, balance list, balance sheet, income statement, balance sheet
  specification, opening balances and deleted vouchers. The Excel file can be
  old XLS or XLSX.
- When you switch fiscal year, reload the page and check the year in the
  report header before exporting, because the old header can stay on screen.
  The balance sheet specification can keep the previous year's end date, so
  check its date separately.
- The journal can contain deleted rows. Count the active entries from the
  general ledger and keep the journal separately.
- The list of accounts holds the account names and VAT codes, and it is the
  basis for mapping the accounts in the opening balances. We found no SIE4
  export.
- The voucher CSV export can be blocked by access rights. The bank
  transaction list has its own CSV and XLSX export, which works separately.
- Lists default to short time windows; the processing history, for example,
  shows the last week. Widen the range and check the hit count. Look at
  purchase invoices in every state with no date filter.

### Holvi

- The "Raportit" (reports) section gives the bank statement as PDF and Excel,
  the electronic statement in camt.052 format, and the journal and general
  ledger. Which reports are available depends on the pricing plan
  ([Holvin kirjanpitoraportit ja viennit](https://support.holvi.com/hc/fi/articles/115000984169-Holvin-kirjanpitoraportit-ja-viennit),
  in Finnish). camt.052 can be imported into aggountant as it is.
- Bank transactions and Holvi's bookkeeping documents (purchase and sales
  invoices with their attachments) are different sets, so inventory both. The
  documents' attachments and comments are per document in the browser. Look
  at the comments on every document, including those whose list row shows no
  note.
- The Excel export can be XLSX even when the name says XLS.
- Yearly statements can overlap at the boundary date. Count transactions by
  unique bank reference, not by row count.
- The session expires after a short idle time. Sign in again when it does,
  because reloading the page does not replace signing in.

## Limitations

- The old system's vouchers and chart of accounts cannot be imported into
  aggountant, not as a SIE4 file either:
  [Company home and setup](../reference/status.md#company-home-and-setup).
- Of bank statements, only Holvi's has been tried:
  [Bank statement](../reference/status.md#bank-statement).

## If something goes wrong

- If the service refuses an export or another function, the agent notes it
  as missing in the inventory and does not try a workaround. Ask the
  program's administrator or the accounting firm for the right or the export.
- If a file does not open or is shorter than it should be, the download was
  interrupted. The agent fetches it again and updates its checksum.
- If the bank balance does not agree after the first month, see
  [The bank balance does not match the statement](../troubleshooting.md#the-bank-balance-does-not-match-the-statement).

## See also

- [Set up your company's books](set-up-company.md)
- [The company home: what is stored where](../concepts/company-home.md)
- [What is ready and what is not](../reference/status.md)
