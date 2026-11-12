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
   through.

3. **The agent prepares a proposal.** The program does not decide accounts or
   VAT treatment. For every transaction the agent looks for a receipt or
   invoice, looks at how similar transactions were recorded before, and
   proposes the counter-account and, for purchases, the VAT treatment and the
   invoice date. What it cannot work out, it asks you all together.

4. **Check and approve.** Ask to see the list row by row if the agent does not
   show it. Look at each row's date, counterparty, amount, account and VAT
   treatment. Check in particular:
   - The VAT rate was chosen from **the invoice date**, not the payment date.
   - **No transaction ends up recorded twice.** If a row pays a receipt that
     is already recorded as a voucher of its own, ask the agent which account
     the row is recorded to. See "Limitations" below.

   The bank statement import has no separate preview hash as receipt
   recording does. Your approval is that you look at the list before it is
   sealed.

   > I approve, record it.

5. **The agent seals the whole statement at once.** The original statement
   file is archived and attached to every voucher created from its
   transactions.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant import camt053 parse --file tiliote-2026-03.xml
aggountant voucher list --posting-from 2026-03-01 --posting-to 2026-03-31
aggountant import camt053 seal --file tiliote-2026-03.xml < assignments.json
```

The same command also reads camt.052 files (also under the name
`import camt052`).
</details>

## Done when

- The program replies `sealed N vouchers from "FI…"`, and every confirmed
  transaction on the statement is either recorded or named as open by the
  agent.
- The bank account balance in the monthly report matches the statement's
  closing balance ([See how the month went](monthly-report.md)).

The same statement can be imported again. Transactions already sealed are
recognised and not recorded a second time, even though the program prints the
same `sealed` line.

## Limitations

- **Every confirmed transaction needs an entry.** If even one is missing or
  refers to an account that is not in the chart of accounts, the whole
  statement is rejected and nothing is sealed. This also applies to a row that
  pays a receipt already recorded, and there is a **risk of double booking**:
  if the receipt was recorded directly against the bank account and the row is
  also recorded as an expense, the same purchase is in the books twice.
  Comparing the bank account balance with the statement reveals this before
  the close.
- A purchase invoice cannot be attached to a voucher recorded from the bank
  statement. The attachment is the statement.
- VAT is split out from the statement only for purchases. A sale that
  includes VAT is recorded separately:
  [Record a sales invoice, payroll or another manual entry](record-other-entries.md).

More detail: [What is ready and what is not](../reference/status.md#bank-statement).

## If something goes wrong

- **The whole statement was rejected.** A transaction was left without an
  entry, or an account is missing from the chart of accounts. Nothing was
  sealed, and the agent fixes the proposal:
  [Error codes](../reference/error-codes.md).
- **The bank account balance does not match the statement.** A transaction is
  missing or recorded twice:
  [When something goes wrong](../troubleshooting.md#the-bank-balance-does-not-match-the-statement).

## See also

- [Record receipts and purchase invoices](record-receipts.md)
- [How the VAT treatment is decided](../concepts/vat-treatment.md)
- [Do a month's bookkeeping from start to finish](monthly-routine.md)
