# For bookkeepers

If you keep the books with aggountant on the owner's behalf, or receive
material from it for the financial statements, you need to know how the
program meets the requirements of the Finnish Accounting Act
(kirjanpitolaki), what material you can get from it and what it is not used
for.

## Vouchers and the audit trail

- Vouchers are numbered without gaps in one running series, in the order they
  are sealed.
- A hash is calculated from a sealed voucher's content and chained to the
  previous voucher.
  [The chain check](sealing-and-approval.md#what-sealing-is) reveals if a
  voucher has been changed or deleted afterwards.
- A sealed voucher is not edited. A mistake is corrected with a correcting
  voucher that refers to the original, and both stay in the books.
- A voucher has a document date (the date of the receipt or invoice) and a
  separate posting date. For a transaction recorded from the bank statement,
  the posting date is the bank's booking date.
- [A month is closed](periods.md) with an approval, and nothing can be
  recorded in a closed month. Corrections go into the next open period.

## Attachments and retention

- Every source file (receipt, invoice, bank statement) is archived in the
  company home's attachment store under a hash of its content, and the
  voucher refers to it. The same file is recognised if someone tries to
  record it again.
- The original bank statement file is archived once and attached to every
  voucher created from its transactions.
- A receipt or purchase invoice paid from the company's account is recorded
  as a voucher of its own, with its attachment, against trade payables 2700,
  and the statement's payment row is recorded against the same payable.
- The attachment store and the voucher database are not in git, so the owner
  [backs them up separately](company-home.md#git-and-backups).

## VAT

A named [VAT treatment](vat-treatment.md) is stored in the voucher at the
moment of sealing, and the VAT return figures are calculated from these
codes, not from account numbers. The rate comes from a dated VAT rate table
according to the invoice date. A VAT return can be filed only for a closed
period.

## What material you get

From aggountant you can get:

- [the monthly report](../tasks/monthly-report.md) in HTML or JSON. It has
  the month's income statement, the result accumulated since the start of
  the fiscal year and the balance sheet at the end of the month, and every
  account refers to the vouchers its amount is made of.
- the vouchers in machine-readable form, down to individual postings, and
  their attachments
- the state of the periods, and whether each period's VAT has been filed.

Ask the owner or the agent to
[gather the material](../tasks/year-end.md).

## Limits

- The official financial statements document and the written part of the
  notes are done elsewhere. The year-end run records depreciation, accruals
  and corrections in the year that has ended, calculates the income statement
  and balance sheet, and seals the fiscal year. Because the year's months are
  already closed, the run records these vouchers in the first open period of
  the next fiscal year and attributes them to the year that has ended. In the
  voucher list they therefore appear at the start of the next year
  ([Year-end financial statements](../reference/status.md#year-end-financial-statements)).
- Payroll is calculated, paid and reported elsewhere, and a payroll voucher
  is recorded in aggountant. To Tulorekisteri (the Incomes Register),
  aggountant reports only tax-free travel allowances and the employer's
  separate report for months without wages.
- You cannot file on behalf of another company. One home is one company's
  books, and returns are sent with the company's own credentials.

All limitations are on the page
[What is ready and what is not](../reference/status.md).

## See also

- [You, the agent and aggountant](division-of-work.md)
- [Correct a wrong voucher](../tasks/correct-voucher.md)
- [Glossary](../reference/glossary.md)
