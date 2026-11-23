# Close the month

Once a month's transactions are recorded, you close the month. After that
nothing more can be recorded in it, and the VAT return is made from its
figures. A close cannot be undone, so do it only when the month is finished.

## Before you start

- Every transaction on the bank statement is recorded or named as open
  ([Import the bank statement](import-bank-statement.md)).
- Receipts and invoices are recorded and attached to vouchers
  ([Record receipts and purchase invoices](record-receipts.md)).
- The bank account balance in the monthly report equals the statement's
  closing balance ([See how the month went](monthly-report.md)).
- There are no open obligations, because an open obligation blocks the close
  ([Periods, closing and the fiscal year](../concepts/periods.md#obligations)).
- The VAT accounts agree with the VAT return. The VAT return preview also
  runs on an open month, as a draft that cannot be filed, and it shows whether
  a posting on a VAT account is missing its VAT code. Before the close such a
  voucher is still corrected in the same month. After the close, the return
  has to be filed in OmaVero with corrected figures.

## How to do it

1. **Ask the agent.**

   > Close March.

2. **The agent checks the month.** It goes through the conditions above and
   tells you what is missing. The close preview shows what the close would
   lock and changes nothing yet. Fix the gaps first: the agent starts the
   close only when the month is finished or you have decided that something
   named as open will be recorded later.

3. **The agent starts the close.** The month moves to the state `closing`.
   Nothing more can be recorded in it, and the close waits for your approval.

4. **Check and approve.** The program puts together what you approve: the
   count and numbers of the month's vouchers, the month's result and the
   result since the start of the fiscal year, the account balances at the end
   of the month (for example the bank account and trade payables) and the
   month's open obligations. The agent shows it and tells you whether the bank
   account matches the statement, whether the VAT accounts agree with the
   return, and what stays open. Ask if it does not. Approve when you know
   nothing more needs to be recorded in the month, because approval locks the
   month permanently.

   > I approve the close.

   If you notice a gap at this point, reject the close. The month returns to
   the state `open`, nothing is removed, and you can record the missing item
   and start the close again.

   > I reject the close, a receipt is still missing from the month.

   If the previous fiscal year has not been closed yet, the balances show only
   this fiscal year's transactions without the opening balances, and the
   content says so. The close does not wait for the year-end close.

   You approve exactly the content you were shown. If the books change after
   that, for example because a voucher recorded in an earlier open month
   changes the closing balances, the approval is refused with
   `CLOSE_CONTENT_CHANGED`. The close is then rejected and started again, and
   you approve what the books say now.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant obligation list
aggountant obligation resolve --id <identifier>
aggountant file-vat preview --period 2026-03
aggountant run preview close-period --period 2026-03
aggountant run start close-period --period 2026-03
aggountant run show --run-id cp-…
aggountant run answer --run-id cp-… --gate ApproveClose --answer approve
aggountant period show --period 2026-03
```

An obligation is marked resolved (`obligation resolve`) only when the matter
has really been dealt with.

Before the close, `file-vat preview` answers with `fileable: false` and the
attention item `VAT_PERIOD_NOT_SEALED`. Its `vat_reconciliation.status` must be
`reconciled`, or the postings listed under `VAT_ACCOUNTS_UNRECONCILED` must be
ones that do not belong in a return.

`run preview` changes nothing. Its `ready` and `blockers` say whether a close
can start.

The close waits at the approval `ApproveClose`. `run start` and `run show`
return the content to approve in `open_gate.content` and the command that
answers it in `next_command`. That command has `'<approve|reject>'` in place
of the answer, and the agent puts in `approve` or `reject` according to your
decision. The placeholder as it stands is refused as an error. `--answer
reject` returns the month to `open`.

For a quarterly period, the period is given as `2026-Q1`.
</details>

## Done when

The agent shows the month's state, and it is `sealed`:

```
period "2026-03" — "sealed" (…, vat_filed false)
```

`vat_filed false` means the month's VAT return has not been made yet. Make it
next, [in OmaVero](file-vat-omavero.md) or
[directly to the Tax Administration](file-vat-direct.md).

## Limitations

- A closed month cannot be reopened.
- You cannot close both months and quarters in the same year. Monthly is the
  recommended and tried option.

More detail: [What is ready and what is not](../reference/status.md#fiscal-year-and-periods).

## If something goes wrong

- If you notice a mistake after the close, record the correction in the next
  open month: [Correct a wrong voucher](correct-voucher.md).
- If an entry is refused with `PERIOD_CLOSING` or `PERIOD_SEALED`, the month's
  close is in progress or done. Record the entry in the next month.
- If the computer crashed or the connection dropped during the close, nothing
  was lost, and the agent continues the run:
  [A run was interrupted](../troubleshooting.md#a-run-was-interrupted).
- If the month stayed in the state `closing`, see
  [A month is stuck in closing](../troubleshooting.md#a-month-is-stuck-in-closing).

## See also

- [Periods, closing and the fiscal year](../concepts/periods.md)
- [Do a month's bookkeeping from start to finish](monthly-routine.md)
