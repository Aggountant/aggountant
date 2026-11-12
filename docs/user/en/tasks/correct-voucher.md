# Correct a wrong voucher

A sealed voucher is never edited or deleted. A mistake is corrected with a new
correcting voucher that refers to the original, and both stay in the books.
This is exactly what the Accounting Act requires.

## Before you start

- You know which voucher is wrong. If you do not know its number, ask the
  agent to find the voucher by its date, amount or attachment.
- The correction is recorded in an open month. If the wrong voucher's month
  is already closed, the correction goes into the next open month.

## How to do it

1. **Ask the agent.**

   > Voucher 12 was recorded as office expenses, but it is server rental.
   > Correct it.

2. **The agent prepares a correcting voucher** as a manual entry and refers to
   the voucher being corrected in its description. The shape of the
   correction depends on the mistake:
   - **wrong account or VAT treatment**: a voucher that moves the amount to
     the right account
   - **a transaction recorded twice, or one that does not belong to the
     company**: a reversing entry that cancels the original
   - **wrong amount**: an entry for the difference.

3. **Check and approve.** Check that the correction refers to the right
   voucher, that afterwards the amount is on the right account, and that the
   VAT postings are corrected at the same time. If the VAT treatment changes,
   the correction affects the VAT return of the month in which the correction
   is recorded.

4. **The agent seals the correction.**

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant voucher list --posting-from 2026-03-01 --posting-to 2026-03-31
aggountant voucher show 12
aggountant voucher draft < correction.json
aggountant voucher validate < correction.json
aggountant voucher seal < correction-seal.json
```

Without `--json`, `voucher show` shows only the voucher's number and hash, so
ask the agent to show you the voucher's postings.
</details>

## Done when

- The correcting voucher is sealed, and its description says which voucher it
  corrects.
- In the monthly report the amount is on the right account.

## Limitations

- There is no separate correction command. A correction is an ordinary manual
  entry.
- If the mistake concerns a VAT month that has already been filed and you want
  to fix it with a replacement return, do that in OmaVero. aggountant does not
  make replacement returns.

## See also

- [Sealing and approval](../concepts/sealing-and-approval.md)
- [Record a sales invoice, payroll or another manual entry](record-other-entries.md)
- [Periods, closing and the fiscal year](../concepts/periods.md)
