# Sealing and approval

The agent asks for your approval before it records anything, and a voucher,
once recorded, cannot be changed. Both follow from recording being permanent.

## What sealing is

When a voucher is recorded, it is sealed. It gets a permanent running number,
and a hash is calculated from its content and chained to the previous
voucher's hash. This forms the voucher chain, in which every voucher depends
on all the earlier ones.

If an old voucher is changed or deleted afterwards, the chain no longer adds
up. The agent can check the chain at any time. When everything is intact, the
check prints:

```
chain ok — 3 vouchers, head #3
```

The requirement comes from the Finnish Accounting Act (kirjanpitolaki):
entries must be based on dated and numbered vouchers and made in a permanent
way, and a correction must leave the original entry visible. Paper
bookkeeping did this by writing in ink; aggountant does it with the voucher
chain.

## A mistake is corrected with a new voucher

A mistake in a sealed voucher is corrected with a correcting voucher. If
Esimerkki Oy's server rental was recorded as office expenses, a new voucher
moves the amount to the right account and refers to the original. Both stay
in the books, and anyone can see later what happened and when.

Nothing is recorded in a closed month, so the correction of a mistake there
goes into the next open month.

## Why approval comes before sealing

After sealing you can no longer affect the voucher, so you decide before it
what stays in the books. That is why the agent seals only once you have
explicitly approved the voucher.

Other steps that cannot be undone wait for your approval too:

| Step | Why approval is required |
|---|---|
| sealing a voucher | a sealed voucher is permanent and is corrected only with a correcting voucher |
| closing a month | nothing more can be recorded in a closed month |
| sending the VAT return | the return has legal effect and cannot be withdrawn |
| sending a Tulorekisteri report | same as the VAT return |
| switching Vero.fi to production, and the attestations it needs | the program relies on your attestations and cannot check them |

The agent does everything else without asking: it reads the chart of accounts
and earlier vouchers, previews entries, and runs reports and checks. None of
that leaves a permanent trace.

## You approve what you were shown

An approval covers exactly the content the agent showed you. When the agent
previews a receipt entry, the program calculates a hash from the preview, and
when sealing, the agent gives the same hash back. If the receipt details, the
agent's proposal or the chart of accounts have changed in between, the program
refuses:

```
CAPTURE_PREPARATION_CHANGED
```

The agent then shows a new preview and asks for approval again. So you cannot
approve one thing and get another, even if the agent makes a mistake.

The bank statement import works the same way. Its preview shows every voucher
the statement would produce, and if anything changes after the preview, the
program seals nothing (`IMPORT_PREPARATION_CHANGED`).

Filings are bound to the approval in the same way: the approval of the VAT
return covers exactly the content that is sent, and the program calculates the
figures from the books once more before sending.

## What this means for you

- Look at what you approve. A good approval request shows each voucher's
  date, amount, counterparty, account and VAT treatment. A bare "shall I
  record 14 vouchers?" is not enough, so ask to see the rows if the agent does
  not show them.
- Before approval you can change the proposal freely or reject it, because
  nothing has been stored yet.
- Do not fix the voucher database by hand. The chain check reveals the
  change, and the books must then be
  [restored from a backup](../tasks/back-up-and-restore.md).

## See also

- [Correct a wrong voucher](../tasks/correct-voucher.md)
- [Periods, closing and the fiscal year](periods.md)
- [You, the agent and aggountant](division-of-work.md)
