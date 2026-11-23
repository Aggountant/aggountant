# Import the bank statement

A month's bookkeeping usually starts from the bank statement. The agent reads
the statement and proposes an entry for every transaction, and once you
approve the proposal, the whole statement is sealed into vouchers at once.

## Before you start

- You have the month's statement as an electronic ISO 20022 file (camt.053 or
  camt.052). Holvi's statement has been tried in practice, and the "Electronic
  account statement" from Holvi's self-service works as it is (it is in
  camt.052 format). Other banks' camt files may work, but they have not been
  tried.
- The month's receipts and invoices are where the home's `AGENTS.md` says.

## How to do it

1. **Ask the agent.**

   > Import March's statement tiliote-2026-03.xml and propose the entries.

2. **The agent reads the statement.** Reading does not record anything yet.
   The program lists the transactions the bank has confirmed and gives each a
   permanent identifier. It leaves out unconfirmed transactions
   (reservations), which are recorded from the statement on which they have
   gone through. The program also shows the statement's period, its opening
   and closing balance, and whether the transactions it read take the opening
   balance to the closing balance. If they do not, the file was not read
   correctly and the agent does not import it.

3. **The agent prepares a proposal.** The agent proposes the accounts and the
   VAT treatment, because the program does not decide them. For every
   transaction it looks for a receipt or invoice and checks how similar
   transactions were recorded before. When a row pays a purchase that has a
   receipt or invoice, the receipt is recorded as a voucher of its own against
   trade payables and the statement row against trade payables 2700 without
   VAT, as
   [When the purchase is also on the bank statement](record-receipts.md#when-the-purchase-is-also-on-the-bank-statement)
   describes. For other rows, such as bank service charges, the agent proposes
   the counter-account and, for purchases, the VAT treatment. Whatever it
   cannot work out, it asks you in one go.

4. **Check and approve.** The agent previews the import. The program checks
   the proposal the way sealing does (accounts, amounts, VAT split, period
   state) and shows every voucher it would seal, without recording anything.
   The preview also says which transactions are new and which are already
   recorded. Ask to see the list row by row if the agent does not show it,
   and look at each row's date, counterparty, amount, account and VAT
   treatment. Check in particular:
   - The VAT rate was chosen by the invoice date and not the payment date.
   - No purchase ends up recorded twice. A row that pays a receipt or invoice
     belongs on 2700 Ostovelat (trade payables), or on 2740 when the company
     pays you back for a purchase you paid with your own money. It does not
     belong on an expense account or VAT, because the expense and the VAT are
     already on the receipt's voucher.

   > I approve, record it.

   Your approval covers exactly the vouchers you saw. The preview returns an
   identifier that the approval is bound to, and sealing succeeds only if it
   produces exactly the previewed vouchers.

5. **The agent seals the whole statement at once.** Either the whole statement
   is sealed or nothing from it is recorded. The original statement file is
   archived and attached to every new voucher created from its transactions.

The same statement can be imported again, and overlapping statements can be
imported one after the other, for example two yearly statements that both
contain the transactions around the turn of the year. The program recognises
a transaction that is already recorded by the bank's transaction reference and
does not record it a second time. It only notes that the transaction was in
this file too. The agent gives an already recorded transaction the same
assignment as the first time.

### When bank transactions were recorded by hand before

If the company has recorded bank transactions as ordinary entries (for
example before it started importing statements), the program cannot tell
from the bank's reference that such an entry already records the
transaction. Before it seals a new voucher for a transaction, it therefore
looks for a voucher that may already record it: one whose identifier or
description contains the bank's transaction reference, or one that records
the same amount on the same side of the same bank account within 7 days of
the bank's date. If it finds one, it stops the whole statement and names the
transaction and the voucher (`BANK_ENTRY_POSSIBLY_BOOKED`). Nothing is
recorded twice.

The agent shows you the voucher and asks which case it is:

- **The voucher records this transaction.** The agent links the transaction
  to the voucher instead of creating a new one. The program checks that the
  voucher's bank row has the same account, amount, side and date (within 7
  days); the voucher itself is not changed. The preview shows the transaction
  as linked to that voucher, and later statements that contain the same
  transaction are linked to it in the same way. One voucher can be linked to
  only one bank transaction. If one voucher records several bank
  transactions, for example a month's card payments, only one of them can be
  linked to it, and the others keep stopping the import.
- **The voucher is something else**, for example another payment of the same
  amount. The agent marks that this voucher does not record the transaction,
  and the transaction gets a new voucher.

A company that moves to statement import links its earlier hand-recorded
bank transactions this way the first time it imports a statement that
contains them.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant import camt053 parse --file tiliote-2026-03.xml
aggountant voucher list --posting-from 2026-03-01 --posting-to 2026-03-31
aggountant import camt053 seal --file tiliote-2026-03.xml --dry-run < assignments.json
aggountant import camt053 seal --file tiliote-2026-03.xml \
  --approved-preparation-hash sha256:v1:… < assignments.json
aggountant import camt053 observations
```

The same command also reads camt.052 files, and it can be run under the name
`import camt052` as well.

`import camt053 observations` shows which transactions each statement file
contained.
</details>

## Done when

- The program replies `sealed N new vouchers from "FI…", M transactions
  already in the books`, and every confirmed transaction on the statement is
  recorded. If a transaction's receipt is still missing, the agent has named
  it as open in the home's `TODO.md`.
- The bank account balance in the [monthly report](monthly-report.md) is the
  same as the statement's closing balance.

## Limitations

- Every confirmed transaction needs an entry, including the payment row of a
  receipt that is already recorded, which clears the receipt's payable. If
  even one entry is missing, the whole statement is rejected.
- A purchase invoice cannot be attached to a voucher recorded from the bank
  statement, because its attachment is the statement. That is why the invoice
  is recorded as a voucher of its own, and the statement row records only the
  payment.
- VAT is split out from the statement only for purchases. A sale that
  includes VAT is [recorded as a manual entry](record-other-entries.md).

All the limitations of statement import are on the page
[What is ready and what is not](../reference/status.md#bank-statement).

## If something goes wrong

- `MISSING_ASSIGNMENT`: a transaction was left without an entry, so the
  whole statement was rejected. Nothing was sealed, and the agent completes
  the proposal.
- `ACCOUNT_NOT_FOUND`: an entry refers to an account that is not in the
  chart of accounts. Nothing was sealed, and the agent picks an account from
  the chart.
- `SOURCE_FACTS_CHANGED` means the statement has a transaction that is
  already recorded and the bank's facts about it have changed. Nothing was
  sealed, and the error names each such transaction and its voucher. Find out
  what changed before anything is recorded for the transaction.
- `POSTING_PROJECTION_CHANGED` means a transaction that is already recorded
  was given a different assignment than before. Nothing was sealed, and the
  error names the transactions and their vouchers. The agent gives the same
  assignment as on the voucher. If the earlier entry was wrong, it is
  corrected with a new voucher.
- `BANK_ENTRY_POSSIBLY_BOOKED` means a voucher recorded outside statement
  import may already record a new transaction on the statement. Nothing was
  sealed. The agent asks you whether the voucher records the transaction, as
  [When bank transactions were recorded by hand before](#when-bank-transactions-were-recorded-by-hand-before)
  describes.
- `BANK_VOUCHER_MISMATCH` means a transaction was linked to a voucher that
  does not record it: its bank row has a different account, amount, side or
  date, or the voucher is already linked to another transaction. Nothing was
  sealed. Check the voucher number.
- `IMPORT_PREPARATION_CHANGED` means the statement, the proposal or the chart
  of accounts changed after the preview. Nothing was sealed. The agent
  previews again and asks you to approve the new list.
- If the bank account balance does not match the statement, a transaction is
  missing or recorded twice. The troubleshooting section
  [The bank balance does not match the statement](../troubleshooting.md#the-bank-balance-does-not-match-the-statement)
  explains how to find the cause.
- Other codes are explained on [Error codes](../reference/error-codes.md).

## See also

- [Record receipts and purchase invoices](record-receipts.md)
- [How the VAT treatment is decided](../concepts/vat-treatment.md)
- [Do a month's bookkeeping from start to finish](monthly-routine.md)
