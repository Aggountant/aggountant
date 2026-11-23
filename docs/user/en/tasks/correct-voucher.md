# Correct a wrong voucher

A sealed voucher is never edited or deleted. A mistake is corrected with a new
voucher that refers to the original, and both stay in the books, as the
Accounting Act requires.

## Before you start

- You know which voucher is wrong. If you do not know its number, ask the
  agent to find the voucher by its date, amount or attachment.

## How to do it

1. **Ask the agent.**

   > Voucher 12 was recorded as office expenses, but it is server rental.
   > Correct it.

2. **The agent prepares a correcting voucher** as a manual entry and refers to
   the voucher being corrected in its description. The correction is recorded
   in an open month. If the wrong voucher's month is already closed, the
   correction goes into the next open month. The shape of the correction
   depends on the mistake:
   - If the account or the VAT treatment is wrong, the agent moves the amount
     to the right account.
   - If the transaction was recorded twice or does not belong to the company,
     the agent cancels the original with a reversing entry.
   - If the amount is wrong, the agent records the difference.

3. **Check and approve.** Check that the correction refers to the right
   voucher, that afterwards the amount is on the right account, and that the
   VAT postings are corrected at the same time. If the VAT treatment changes,
   the correction affects the VAT return of the month in which the correction
   is recorded.

   Correcting the books does not correct a VAT return that has been filed. If
   the mistake concerns a month whose VAT return has been filed, tell the
   agent before the voucher is sealed and make a replacement return as
   described in
   [Correct a return that has been filed](file-vat-direct.md#correct-a-return-that-has-been-filed).

4. **The agent seals the correction.**

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant voucher list --posting-from 2026-03-01 --posting-to 2026-03-31
aggountant voucher show 12
aggountant voucher draft < correction.json
aggountant voucher validate < correction-seal.json
aggountant voucher seal < correction-seal.json
```

Ask the agent to show you the voucher's postings, because without `--json`
`voucher show` shows only the voucher's number and hash.
</details>

## Done when

- The correcting voucher is sealed, and its description says which voucher it
  corrects.
- In the monthly report the amount is on the right account.

## Limitations

- There is no separate correction command, so the agent records the
  correction as an ordinary manual entry.

## See also

- [Sealing and approval](../concepts/sealing-and-approval.md)
- [Record a sales invoice, payroll or another manual entry](record-other-entries.md)
- [Periods, closing and the fiscal year](../concepts/periods.md)
