# Close the month

When a month's transactions have been recorded, the month is closed. After the
close nothing more can be recorded in the month, and the VAT return is made
from its figures. A close cannot be undone, so do it only when the month is
finished.

## Before you start

- Every transaction on the bank statement is recorded or named as open
  ([Import the bank statement](import-bank-statement.md)).
- Receipts and invoices are recorded and attached to vouchers
  ([Record receipts and purchase invoices](record-receipts.md)).
- The bank account balance in the monthly report equals the statement's
  closing balance ([See how the month went](monthly-report.md)).
- There are no open obligations. An open obligation blocks the close
  ([Periods, closing and the fiscal year](../concepts/periods.md#obligations)).

## How to do it

1. **Ask the agent.**

   > Close March.

2. **The agent checks the month.** It goes through the conditions above and
   tells you if something is missing. If there are gaps, fix them first. The
   agent does not start the close until the month is finished or you have
   decided that something named as open will be recorded later.

3. **The agent starts the close.** The month moves to the state `closing`.
   Nothing more can be recorded in it, and the close waits for your approval.

4. **Check and approve.** The agent should tell you which month is being
   closed, how many vouchers it has and whether the bank account balance
   matches. Ask if it does not. Approve when you know nothing more needs to be
   recorded in the month. After approval the month is locked permanently.

   > I approve the close.

   **Do not reject the close.** If you notice a gap at this point, approve the
   close anyway and record the missing item in the next month. In this
   version a rejected close leaves the month permanently in the state
   `closing`, where nothing can be recorded in it and it cannot be closed.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant obligation list
aggountant obligation resolve --id <identifier>
aggountant run start close-period --period 2026-03
aggountant run answer --run-id cp-… --gate ApproveClose --answer approve
aggountant period show --period 2026-03
```

An obligation is marked resolved (`obligation resolve`) only when the matter
has really been dealt with. The close waits at the approval `ApproveClose`.
Rejecting would be `--answer reject`, which should not be used. For a
quarterly period, the period is given as `2026-Q1`.
</details>

## Done when

The agent shows the month's state, and the state is `sealed`:

```
period "2026-03" — "sealed" (…, vat_filed false)
```

`vat_filed false` means the month's VAT return has not been made yet. The next
step is the [VAT return in OmaVero](file-vat-omavero.md) or
[directly to the Tax Administration](file-vat-direct.md).

## Limitations

- A closed month, or one being closed, cannot be reopened.
- You cannot close both months and quarters in the same year. Monthly is the
  recommended and tried option.

More detail: [What is ready and what is not](../reference/status.md#rejected-close).

## If something goes wrong

- **You noticed a mistake after the close.** Record the correction in the next
  open month: [Correct a wrong voucher](correct-voucher.md).
- **An entry is refused with `PERIOD_CLOSING` or `PERIOD_SEALED`.** The
  month's close is in progress or done. Record it in the next month.
- **The computer crashed or the connection dropped during the close.**
  Nothing was lost, and the agent continues the run:
  [When something goes wrong](../troubleshooting.md#a-run-was-interrupted).
- **The close was rejected.**
  [When something goes wrong](../troubleshooting.md#a-month-is-stuck-in-closing).

## See also

- [Periods, closing and the fiscal year](../concepts/periods.md)
- [Do a month's bookkeeping from start to finish](monthly-routine.md)
