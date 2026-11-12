# Sealing and approval

When you use aggountant, you notice two things straight away. The agent asks
for your approval before every recording, and a voucher, once recorded,
cannot be changed. This page explains why and what that means in daily work.

## What sealing is

When a voucher is recorded, it is **sealed**. It gets a permanent running
number, and a hash is calculated from its content and chained to the previous
voucher's hash. This forms the **voucher chain**, in which every voucher
depends on all the earlier ones.

If someone changed or deleted an old voucher afterwards, the chain would no
longer add up. The agent can check the chain at any time, and the check tells
you whether everything is intact:

```
chain ok — 3 vouchers, head #3
```

Sealing is not aggountant's own invention. The Finnish Accounting Act
(kirjanpitolaki) requires entries to be based on dated and numbered vouchers,
to be made in a permanent way, and a correction to leave the original entry
visible. In paper bookkeeping the same was done by writing in ink. aggountant
makes it technically certain.

## A mistake is corrected with a new voucher

Because a sealed voucher is not edited, a mistake is corrected with a
**correcting voucher**. If Esimerkki Oy's server rental had been recorded as
office expenses, a new voucher moves the amount to the right account and
refers to the original. Both stay in the books, and anyone can see later what
happened and when.

The same goes for whole months. Nothing is recorded in a closed month, so the
correction goes into the next open month.

## Why approval comes before sealing

Because sealing is permanent, the only moment to influence what stays in the
books is before sealing. That is why the agent seals nothing without your
explicit approval. The approval is the moment you decide what stays in the
books permanently.

The same principle applies to other steps that cannot be undone:

| Step | Why approval is required |
|---|---|
| sealing a voucher | a sealed voucher is permanent and is corrected only with a correcting voucher |
| closing a month | nothing more can be recorded in a closed month |
| sending the VAT return | the return has legal effect and cannot be withdrawn |
| sending a Tulorekisteri report | same as the VAT return |
| switching Vero.fi to production, and the attestations it needs | the program relies on your attestations and cannot check them |

Everything else is the agent's own work, which it does without asking:
reading the chart of accounts and earlier vouchers, previewing entries,
reports and checks. They leave no permanent trace.

## You approve what you were shown

An approval is not a general permission. It is bound to exactly the content
the agent showed you. When the agent previews a receipt entry, the program
calculates a hash from the preview. When sealing, the agent gives the same
hash back. If the receipt details, the agent's proposal or the chart of
accounts have changed in between, the program refuses:

```
CAPTURE_PREPARATION_CHANGED
```

The agent must then show a new preview and ask for approval again. So you
cannot approve one thing and get another, even if the agent makes a mistake.

For filings the binding is the same: the approval of the VAT return covers
exactly the content that is sent, and the program calculates the figures from
the books once more before sending.

The bank statement import has no separate preview hash in this version. Your
approval is that you look at the agent's list of entries row by row before it
seals the statement.

## What this means for you

- **Look at what you approve.** A good approval request shows each voucher's
  date, amount, counterparty, account and VAT treatment. A bare "shall I
  record 14 vouchers?" is not enough. Ask to see the rows if the agent does
  not show them.
- **When you refuse or correct, nothing has been stored yet.** Before
  approval you can change the proposal freely.
- **A mistake in a sealed voucher is not a disaster.** It is corrected with a
  new voucher, as always in bookkeeping.
- **Do not try to fix the voucher database by hand.** The chain check reveals
  the change, and the books must then be restored from a backup
  ([Back up and restore your books](../tasks/back-up-and-restore.md)).

## See also

- [Correct a wrong voucher](../tasks/correct-voucher.md)
- [Periods, closing and the fiscal year](periods.md)
- [You, the agent and aggountant](division-of-work.md)
