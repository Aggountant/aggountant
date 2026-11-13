# Record a sales invoice, payroll or another manual entry

Not everything is recorded from the bank statement or through the receipt
route. Sales invoices, payroll vouchers, opening balances and corrections are
prepared by the agent as a voucher draft, which you approve before it is
sealed.

## Before you start

- You have the source of the entry: a sales invoice, a payslip or summary from
  your payroll service, the old system's balance list, or the number of the
  voucher being corrected.
- The month the entry goes into is still open.

## How to do it

1. **Ask the agent and give it the source.**

   > Record sales invoice 2026-0042 to Asiakas Oy, €2,480.00 including VAT
   > 25.5 %. The invoice is in the folder myyntilaskut.

   For payroll:

   > Record March's payroll from the payroll service's summary.

2. **The agent prepares the draft.** It archives the source file, puts
   together the voucher's postings and validates the draft: debits and credits
   balance, the accounts are in the chart of accounts and the attachment is
   stored. Validation stores nothing. If a posting on a VAT account has no VAT
   code on any line of the voucher, validation warns with
   `VAT_POSTING_UNTAGGED` but does not stop the sealing.

3. **Check and approve.** The agent shows the date, the description, every
   posting's account and amount, and the attachment. Check that the amounts
   match the source and that the VAT on sales is on its own account. Unlike
   the receipt route, the program does not calculate the VAT lines here, so
   check them with particular care. Also ask whether the voucher carries a
   VAT code. The code goes on the line of the tax base, for a sale the sales
   account line (for example `sales-domestic-standard`), and the return
   calculates the tax from it. The VAT return is calculated from coded
   postings, not from account numbers, so uncoded VAT would be missing from
   the return ([How the VAT treatment is decided](../concepts/vat-treatment.md)).
   The program notices the gap at the latest in the VAT return preview, but by
   then the month is closed and the return has to be filed by hand in OmaVero
   with corrected figures.

4. **The agent seals the voucher.**

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant artifact attach --file sales-invoice-2026-0042.pdf
aggountant voucher draft < draft.json
aggountant voucher validate < draft.json
aggountant voucher seal < seal.json
```
</details>

## Done when

- The voucher is sealed, and the agent tells you its number.
- The sales invoice shows in the monthly report under revenue and trade
  receivables. When the customer pays, the statement row is recorded against
  trade receivables.

## Limitations

- The program does not calculate sales VAT from the draft or from the bank
  statement. The agent puts the VAT postings together itself.
- Cash wages are not calculated, paid or reported with aggountant. The payroll
  voucher is recorded from your payroll service's figures.
- There is no separate correction command. A correction is an ordinary manual
  entry: [Correct a wrong voucher](correct-voucher.md).

More detail: [What is ready and what is not](../reference/status.md).

## See also

- [Correct a wrong voucher](correct-voucher.md)
- [Sealing and approval](../concepts/sealing-and-approval.md)
- [Set up your company's books](../start/set-up-company.md) (opening balances)
