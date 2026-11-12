# For bookkeepers

This page is for a bookkeeper or accounting firm that keeps the books with
aggountant on the owner's behalf, or receives material from it for the
financial statements. It explains how aggountant meets the requirements of
the Finnish Accounting Act (kirjanpitolaki), what material you can get from
it and where its limits are.

## Vouchers and the audit trail

- **Numbering.** Vouchers are numbered without gaps in one running series in
  the order they are sealed.
- **Permanence.** A hash is calculated from a sealed voucher's content and
  chained to the previous voucher. The chain check reveals if a voucher has
  been changed or deleted afterwards
  ([Sealing and approval](sealing-and-approval.md)).
- **Corrections.** A sealed voucher is not edited. A mistake is corrected with
  a correcting voucher that refers to the original, and both stay in the
  books.
- **Dates.** A voucher has a separate document date (the date of the receipt
  or invoice) and posting date. For a transaction recorded from the bank
  statement, the posting date is the bank's booking date, and the VAT rate is
  chosen from the invoice date.
- **Periods.** A month is closed with an approval, and nothing can be recorded
  in a closed month. Corrections go into the next open period
  ([Periods, closing and the fiscal year](periods.md)).

## Attachments and retention

- Every source file (receipt, invoice, bank statement) is archived in the
  home's attachment store under a hash of its content, and the voucher refers
  to it. The same file is recognised if someone tries to record it again.
- The original bank statement file is archived once and attached to every
  voucher created from its transactions.
- The attachment store and the voucher database are not in git; the owner
  backs them up separately
  ([The company home](company-home.md#git-and-backups)).

## VAT

The VAT treatment is stored in the voucher at the moment of sealing as a
named treatment, and the VAT return figures are calculated from these codes,
not from account numbers. The rate comes from a dated VAT rate table. The VAT
return is calculated only from a closed period
([How the VAT treatment is decided](vat-treatment.md)).

## What material you get

- **The monthly report** in HTML or JSON: the month's income statement, the
  result accumulated since the start of the fiscal year, and the balance sheet
  at the end of the month. Every account refers to the vouchers its amount is
  made of ([See how the month went](../tasks/monthly-report.md)).
- **The vouchers** in machine-readable form, down to individual postings, and
  their attachments.
- **The state of the periods**, and whether each period's VAT has been filed.

Ask the owner or the agent to gather the material:
[Prepare the financial statements](../tasks/year-end.md).

## Limits

- **The financial statements are not yet prepared with aggountant.** The
  program has a year-end run, but it cannot be used for real financial
  statements. Depreciation, accruals, the statements and the notes are done
  elsewhere.
- **Payroll** is calculated, paid and reported elsewhere. A payroll voucher is
  recorded in aggountant. To Tulorekisteri, aggountant reports only tax-free
  travel allowances.
- **A purchase recorded from the bank statement has the statement attached**,
  not the purchase invoice.
- **Filing on behalf of another company is not supported.** One home is one
  company's books, and returns are sent with the company's own credentials.

All limitations: [What is ready and what is not](../reference/status.md).

## See also

- [Sealing and approval](sealing-and-approval.md)
- [Prepare the financial statements](../tasks/year-end.md)
- [Glossary](../reference/glossary.md)
