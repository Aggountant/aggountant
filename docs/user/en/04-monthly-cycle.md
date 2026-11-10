# 4. The monthly cycle: bank, receipts and period close

A month's bookkeeping follows the same order every month:

1. import the bank statement
2. book receipts and invoices and attach them to their vouchers
3. clear up open matters and obligations
4. close the period
5. file the VAT return (chapter 5).

You can ask the agent to handle a whole month at once, for example "let's do
March". The agent uses the `capture-receipt` skill, does whatever the sources
are enough for, and brings you its questions and the vouchers to approve as one
batch.

## The bank statement

aggountant reads the bank's electronic statement in ISO 20022 format (camt.053
or camt.052). Support was built for Holvi's statements. Other banks' camt files
may work, but they have not been tested.

Importing takes two steps. First the agent reads the file:

```sh
aggountant import camt053 parse --file statement-2026-03.xml
```

Reading books nothing. It lists the transactions and gives each one a permanent
identifier. The program does not decide accounts or VAT treatment. Those are
the agent's proposal, which you approve. For each transaction the agent puts
together a proposal: the bank account, the counter-account or accounts, and for
purchases, where needed, the VAT treatment and the invoice date. Once you
approve, the whole statement is sealed in one go:

```sh
aggountant import camt053 seal --file statement-2026-03.xml < assignments.json
```

Worth knowing:

- **Every transaction on the statement needs an assignment.** If even one is
  missing, or points to an account that is not in the chart of accounts, the
  whole batch is rejected and nothing is sealed.
- **The original statement file is archived** and attached to every voucher
  created from its transactions.
- **You can import the same statement again.** Transactions already sealed are
  recognised and not booked a second time.
- **This command has no separate preview fingerprint**, unlike receipt booking.
  Your approval consists of looking at the agent's assignment list before it
  runs `seal`. Ask to see the list line by line.
- **The import splits out VAT only for purchases.** Sales that include VAT are
  not booked from the bank statement. The agent prepares them as a separate
  voucher.
- **Each transaction is booked once.** If a statement line is the payment of a
  receipt that has already been booked, it is not booked again. The agent
  checks earlier vouchers before making its proposal.

## Receipts and invoices

An ordinary purchase receipt is booked like this:

1. **The source file is archived.** `aggountant artifact attach --file receipt.pdf`
   stores the file and returns its identifier. The same file is recognised
   later, so a duplicate of the same receipt is caught
   (`aggountant voucher list --attachment-ref <identifier>`).
2. **The agent previews the entry.** It records the details read from the
   receipt (amount, receipt date, description) and its own proposal (expense
   account, payment account, VAT treatment), and runs
   `aggountant capture-receipt record --dry-run`. The program builds the
   entry's VAT lines from the rate table according to the receipt date. The
   preview stores nothing.
3. **You approve.** The agent shows the source details, the proposal and the
   resulting entry lines. For example, a software licence of €124.00 (including
   25.5 % VAT) is booked as €98.80 to account 7690 Atk-laite- ja
   ohjelmistokulut (IT hardware and software expenses), €25.20 to account 1763
   Arvonlisäverosaaminen (VAT receivable), and €124.00 to the credit of account
   1910 Pankkitili (bank account).
4. **The voucher is sealed.** The agent runs the same command again and passes
   the preview's fingerprint (`--approved-preparation-hash`). If anything has
   changed in between, the program refuses (`CAPTURE_PREPARATION_CHANGED`) and
   the preview is done again.

Trying to book the same receipt again does not create a second voucher. The
program returns the voucher that is already sealed. The receipt date matters,
because the VAT rate is chosen by it. The general rate is 25.5 % from
1 September 2024, and the reduced 14 % rate fell to 13.5 % on 1 January 2026.

The VAT treatment is always a named choice: a domestic purchase, reverse charge
(EU services, EU goods, services from outside the EU), or a VAT-exempt purchase
once the exemption has been confirmed. If the deciding information is missing,
the agent marks the receipt as open rather than guessing. You then get a
question that says what needs to be found out. This route does not handle
partial deduction rights or imports.

**Special entries** that the receipt route does not support (for example sales
invoices, opening balances and corrections) are prepared by the agent as a
voucher draft and checked before sealing (`voucher draft`, `voucher validate`,
`voucher seal`). The same rule applies: the agent shows the lines and you
approve before sealing.

### Unusual receipts and consultation

`capture-receipt start` is the route for receipts whose handling would need an
outside consultation. Only the details you approved would leave the machine,
never the original files. This version of the program has no connection to an
outside consultation service, however, so the route cannot be used yet. For an
unusual receipt the agent asks you or your accountant.

## Missing receipts

When the statement shows a payment with no receipt, the agent looks for the
receipt in the places described in the home's `AGENTS.md`. If it cannot find
one, the agent marks the transaction as open and tells you what is missing.

The program has a `hunt-receipts` command for searching receipts automatically
from email and online-shop accounts. In this version it is not connected to any
real email or shop account, so for now the search is manual work for you and
the agent.

## Obligations

An obligation is an open matter with a due date, such as a missing mileage
log. An open obligation blocks the close of its period, so the matter is not
forgotten.

```sh
aggountant obligation list
aggountant obligation resolve --id <identifier>
```

In this version obligations are created only by the consultation route for
unusual receipts, so in normal use the list is usually empty. Mark an
obligation resolved only once the matter has actually been dealt with.

## Closing the period

When all of the month's transactions have been booked, the period is closed.
After the close nothing more can be booked into the period, and the VAT return
is made from the closed period's figures.

**Before you start the close**, check with the agent that:

- every transaction on the statement has been booked or marked as open
- the receipts are attached
- the bank account balance matches the statement's closing balance (from the
  monthly report, chapter 7)
- there are no open obligations.

Start the close like this:

```sh
aggountant run start close-period --period 2026-03
```

The period moves to the `closing` state, and the close run waits for your
approval (`ApproveClose`). In this state no new vouchers can be sealed into the
period. When you approve, the period is sealed:

```sh
aggountant run answer --run-id cp-… --gate ApproveClose --answer approve
```

You can see the period's state at any time:

```sh
aggountant period show --period 2026-03
```

The state is `open`, `closing` (close in progress) or `sealed` (closed). The
output also shows whether the period's VAT return has been filed
(`vat_filed`).

**Important: do not start the close too early, and do not reject it.** This
version has no command to reopen a period. If you start the close and then
notice something missing:

- **If you approve the close**, the period is closed, and the missing or
  corrected transaction is booked into the next open period. This is the normal
  way to correct a closed period in bookkeeping.
- **If you reject the close** (`--answer reject`), the period stays in `closing`
  for good. Nothing can then be booked into it and it cannot be closed again.
  This is a known shortcoming.

Nothing can be booked into a closed period (`PERIOD_SEALED`). Corrections are
booked into the next open period.
