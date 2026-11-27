# Move your books from another program

When a company moves to aggountant from another bookkeeping program,
aggountant starts from the opening balances on the cutover date. The old
system's history is exported from the old program and the bank into an
archive, which you keep for as long as the law requires. Collect the material
before you give up the old program, because afterwards you may no longer be
able to get it. The agent uses the `migrate` skill for the move.

The move does not have to happen all at once. The books move first, and the
VAT returns, the Incomes Register (Tulorekisteri) reports and travel and
expense claims can follow later, each on its own date. This page describes
moving the books; the other areas are under
[Other areas later](#other-areas-later). The agent asks only about the areas
your company has.

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
   last number in the voucher list.

   Read the list through. You know whether the company has had other bank
   accounts or an earlier program that the agent cannot see from the old
   program.

3. **The agent exports the material.** It uses the program's own export or
   API first. What is only available in the browser the agent can fetch from
   your signed-in browser, if its environment can control a browser.
   Otherwise you download the files yourself and hand them to the agent.
   aggountant itself has no tools for operating the old program or the bank.
   The agent only reads the old system: it exports and downloads, but saves,
   approves, changes or deletes nothing.

4. **The agent checks the files and assembles the archive.** It opens every
   file and reads from its content that the company and the period are
   right. In the books it checks that every voucher's debits and credits
   agree and that the yearly totals match the financial statements. The agent
   keeps the original exports unchanged and records every file's SHA-256
   checksum in a `SHA256SUMS` file, which later shows that a file is as it
   was.

5. **Decide where the archive is kept.** Keep it out of git, because it is
   real financial data and large, and nothing can be removed from git
   history. Keep it [within your backups](../tasks/back-up-and-restore.md).
   The agent copies the archive to a second place, checks both copies
   against the checksums and writes in the home's `AGENTS.md` where the
   archive is. Then it can find an old voucher when you ask about one.

6. **Go through what is missing.** The agent shows the items on the list it
   could not get and the differences it found when checking. Ask the
   accounting firm or the program's administrator for what is missing. Keep
   the old program's subscription active until every item is saved or you
   have accepted that it stays missing.

7. **Set up the books.** The balance list and the list of accounts become
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
   the balance list and the list of open invoices so that the same invoice is
   not booked twice.

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
- The company facts record the date from which the books are in aggountant.

## Other areas later

Each area has its own cutover date, and the agent records in the company
facts what was moved and from which date. The next area can therefore start
months later in another session:

> Let's now move Esimerkki Oy's Incomes Register reports to aggountant from
> 1 March 2026.

- **VAT returns** need the books, because the return is calculated from
  aggountant's vouchers. The first period aggountant files lies wholly in its
  books, so set the books' cutover date at the turn of a VAT period. Agree
  who files the old system's last period: its due date usually falls after
  the cutover.
- **Incomes Register reports** (tax-free travel allowances and the
  employer's separate report) move at the turn of a month. The agent imports
  the months already reported from the Incomes Register's report 630
  ([When an earlier payroll service filed reports](../tasks/file-employer-report.md#when-an-earlier-payroll-service-filed-reports)),
  and you agree with the previous reporter who reports the payments of the
  cutover month. aggountant does not report cash wages.
- **Travel and expense claims** apply only to a company that pays them. Old
  claims stay in the archive with their vouchers. What carries over is the
  claims approved but unpaid on the cutover date, unreported reimbursements
  and each person's [regular workplace](../tasks/record-regular-workplace.md).
  The program has no expense claim commands yet, so part of this area waits
  for them: [Receipts and entries](../reference/status.md#receipts-and-entries).

## Notes for specific programs

The agent's skill has observations on Briox's and Holvi's exports from 2026.
For your part, this follows from them:

### Briox

- The export of the archive and attached documents
  ([Arkiston vienti/lataaminen](https://help.briox.fi/hc/fi/articles/360029163831-Arkiston-vienti-lataaminen),
  in Finnish) and the voucher CSV export may not be visible with your access
  rights. Ask the accounting firm or the administrator for them then.
  Without the archive export, attachments are available only one at a time
  in the browser, and the display version of an image attachment may not be
  the file originally uploaded.
- We found no SIE4 export.

### Holvi

- Which reports are available depends on the pricing plan
  ([Holvin kirjanpitoraportit ja viennit](https://support.holvi.com/hc/fi/articles/115000984169-Holvin-kirjanpitoraportit-ja-viennit),
  in Finnish).
- Holvi's electronic statement (camt.052) can be imported into aggountant as
  it is.
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
