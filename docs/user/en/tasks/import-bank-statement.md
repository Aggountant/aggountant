# Import the bank statement

A month's bookkeeping usually starts from the bank statement. The agent reads
the statement and proposes an entry for every transaction, and with your
approval the whole statement is sealed into vouchers at once.

## Before you start

- You have the month's statement as an electronic ISO 20022 file (camt.053 or
  camt.052). Holvi's statement has been tried in practice. The "Electronic
  account statement" from Holvi's self-service is in camt.052 format and works
  as it is. Other banks' camt files may work, but they have not been tried.
- The month's receipts and invoices are where the home's `AGENTS.md` says.

## How to do it

1. **Ask the agent.**

   > Import March's statement tiliote-2026-03.xml and propose the entries.

2. **The agent reads the statement.** Reading records nothing. The program
   lists the transactions the bank has confirmed and gives each a permanent
   identifier. Transactions the bank has not yet confirmed (reservations) are
   left out; they are recorded from the statement on which they have gone
   through. The program also shows the statement's period, its opening and
   closing balance, and whether the transactions it read take the opening
   balance to the closing balance. If they do not, the file was not read
   correctly and is not imported.

3. **The agent prepares a proposal.** The program does not decide accounts or
   VAT treatment. For every transaction the agent looks for a receipt or
   invoice and looks at how similar transactions were recorded before. When a
   row pays a purchase that has a receipt or invoice, the receipt is recorded
   as a voucher of its own against trade payables, and the row is recorded
   against trade payables 2700 without VAT
   ([When the purchase is also on the bank statement](record-receipts.md#when-the-purchase-is-also-on-the-bank-statement)).
   For other rows, such as bank service charges, the agent proposes the
   counter-account and, for purchases, the VAT treatment. What it cannot work
   out, it asks you all together.

4. **Check and approve.** The agent previews the import: the program checks
   the proposal as sealing would (accounts, amounts, VAT split, period state)
   and shows every voucher it would seal, without recording anything. Ask to
   see the list row by row if the agent does not show it. Look at each row's date, counterparty, amount, account and VAT
   treatment. Check in particular:
   - The VAT rate was chosen from **the invoice date**, not the payment date.
   - **No purchase ends up recorded twice.** A row that pays a receipt or
     invoice is recorded to 2700 Ostovelat (trade payables), or to 2740 when
     the company pays you back for a purchase you paid with your own money,
     not to an expense account or VAT. The expense and VAT are already on the
     receipt's voucher.

   The preview returns an identifier, as receipt recording does. Your
   approval is bound to it: sealing succeeds only if it produces exactly the
   vouchers that were previewed.

   > I approve, record it.

5. **The agent seals the whole statement at once.** The original statement
   file is archived and attached to every new voucher created from its
   transactions. Either the whole statement is sealed or nothing from it is
   recorded.

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

The same command also reads camt.052 files (also under the name
`import camt052`).
</details>

## Done when

- The program replies `sealed N new vouchers from "FI…", M transactions
  already in the books`, and every confirmed transaction on the statement is
  either recorded or named as open by the agent.
- The bank account balance in the monthly report matches the statement's
  closing balance ([See how the month went](monthly-report.md)).

The same statement can be imported again, and overlapping statements (for
example two yearly statements that both contain the transactions around the
turn of the year) can be imported one after the other. The program recognises
a transaction already recorded by the bank's transaction reference and does
not record it a second time. It only notes that the transaction was also in
this file; `import camt053 observations` shows which transactions each
statement file contained. A transaction already recorded is given the same
assignment as the first time. The preview tells for each transaction whether
it is new or already recorded.

## Limitations

- **Every confirmed transaction needs an entry.** If even one is missing or
  refers to an account that is not in the chart of accounts, the whole
  statement is rejected and nothing is sealed. The payment row of a receipt
  already recorded is recorded too: it clears the receipt's payable.
- A purchase invoice cannot be attached to a voucher recorded from the bank
  statement. The attachment is the statement. That is why the invoice is
  recorded as a voucher of its own, and the statement row records only the
  payment.
- VAT is split out from the statement only for purchases. A sale that
  includes VAT is recorded separately:
  [Record a sales invoice, payroll or another manual entry](record-other-entries.md).

More detail: [What is ready and what is not](../reference/status.md#bank-statement).

## If something goes wrong

- **The whole statement was rejected.** A transaction was left without an
  entry, or an account is missing from the chart of accounts. Nothing was
  sealed, and the agent fixes the proposal:
  [Error codes](../reference/error-codes.md).
- **The import was refused with `SOURCE_FACTS_CHANGED` or
  `POSTING_PROJECTION_CHANGED`.** The statement has a transaction that is
  already recorded, but the bank's facts about it have changed
  (`SOURCE_FACTS_CHANGED`) or it was given a different assignment than before
  (`POSTING_PROJECTION_CHANGED`). Nothing was sealed. The error names each such
  transaction and its voucher. Changed bank facts are investigated before
  anything is recorded for the transaction; a different assignment is fixed by
  giving the same assignment as on the voucher, and if the earlier entry was
  wrong, it is corrected with a new voucher
  ([Error codes](../reference/error-codes.md)).
- **Sealing was refused with `IMPORT_PREPARATION_CHANGED`.** The statement,
  the proposal or the chart of accounts changed after the preview. Nothing
  was sealed; the agent previews again and asks you to approve the new list.
- **The bank account balance does not match the statement.** A transaction is
  missing or recorded twice:
  [When something goes wrong](../troubleshooting.md#the-bank-balance-does-not-match-the-statement).

## See also

- [Record receipts and purchase invoices](record-receipts.md)
- [How the VAT treatment is decided](../concepts/vat-treatment.md)
- [Do a month's bookkeeping from start to finish](monthly-routine.md)
