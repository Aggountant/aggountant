# Record a sales invoice, payroll or another manual entry

Sales invoices, payroll vouchers, opening balances and corrections do not come
from the bank statement or through the receipt route. The agent prepares a
voucher draft for each, and you approve it before it is sealed.

## Before you start

- You have the source of the entry: a sales invoice, a payslip or summary from
  your payroll service, the old system's balance list, or the number of the
  voucher being corrected.
- The month the entry goes into is still open.

## How to do it

1. **Ask the agent and give it the source.**

   > Record sales invoice 2026-0042 to Asiakas Oy, €2,480.00 including 25.5%
   > VAT. The invoice is in the folder myyntilaskut.

   For payroll you could ask:

   > Record March's payroll from the payroll service's summary.

2. **The agent prepares the draft.** It archives the source file, puts
   together the voucher's postings and validates the draft without storing
   anything: debits and credits balance, the accounts are in the chart of
   accounts and the attachment is stored. If a posting on a VAT account has no
   VAT code on any line of the voucher, validation warns with
   `VAT_POSTING_UNTAGGED` but does not stop the sealing.

3. **Check and approve.** The agent shows the date, the description, every
   posting's account and amount, and the attachment. Check that the amounts
   match the source and that the VAT on sales is on its own account. Unlike
   the receipt route, the program does not calculate the VAT lines here, so
   check them carefully.

   Also ask whether the voucher carries a VAT code. The VAT return is
   calculated from coded postings and not from account numbers, so
   [uncoded VAT is left out of the return](../concepts/vat-treatment.md). The
   code goes on the line of the tax base, which for a sale is the sales
   account line (for example `sales-domestic-standard`).

   An uncoded posting shows once more in the
   [VAT return preview before the month is closed](close-month.md#before-you-start).
   If it is found only after the close, the return has to be filed by hand in
   OmaVero with corrected figures.

4. **The agent seals the voucher.** When the customer later pays the invoice,
   the agent records the statement row against trade receivables.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant artifact attach --file sales-invoice-2026-0042.pdf
aggountant voucher vat-codes --document-date 2026-03-15
aggountant voucher draft < draft.json
aggountant voucher validate < seal.json
aggountant voucher seal < seal.json
```

`voucher vat-codes --document-date` lists the VAT codes valid on the voucher's
date.

With `voucher validate` the agent validates the same file it seals. Validation
then also refuses a voucher dated in a closed month, and tells whether the
file's `natural_key` has already sealed a voucher. If the content is the same,
validation warns and sealing again returns the existing voucher. If the
content is different, validation refuses the file.
</details>

## Done when

- The agent tells you the number of the sealed voucher.
- The sales invoice shows in the monthly report under revenue and trade
  receivables.

## Limitations

- The program
  [does not calculate sales VAT](../reference/status.md#bank-statement) from
  the draft or from the bank statement.
- You
  [cannot calculate, pay or report cash wages](../reference/status.md#payroll)
  with aggountant, so the agent records the payroll voucher from your payroll
  service's figures.
- There is no separate command for a correction. It is an ordinary manual
  entry, and [Correct a wrong voucher](correct-voucher.md) tells how to make
  one.

## See also

- [Sealing and approval](../concepts/sealing-and-approval.md)
- [Set up your company's books](../start/set-up-company.md), where the opening
  balances are recorded
- [What is ready and what is not](../reference/status.md)
