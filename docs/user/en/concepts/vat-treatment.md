# How the VAT treatment is decided

The agent proposes a VAT treatment for every purchase, and sometimes it asks
you something that seems obvious. This page explains how the VAT treatment is
decided, why the agent does not guess and where the VAT return figures come
from.

## The VAT treatment is always a named choice

Every purchase gets a named treatment:

- **domestic purchase**: the receipt's amount includes VAT, and the deductible
  tax is calculated from it
- **reverse charge**: the buyer accounts for the tax, for example when buying
  services from another EU country. There are three kinds: EU services, EU
  goods and services from outside the EU.
- **VAT-exempt purchase**, when the exemption has been confirmed from the
  receipt or the company's practices.

If a decisive detail is missing, the agent marks the receipt as needing
clarification and does not guess. You then get a question that tells you what
needs to be found out. Recording unknown VAT as exempt would be silently wrong
bookkeeping that nobody would notice before the return.

The receipt route does not handle partial deduction rights or imports. The
agent tells you if a purchase is one of those.

## The rate is chosen from the receipt's date

The VAT rate depends on the product and on when the purchase was made. That
is why the rate is chosen from the date of the receipt or invoice, not the
payment date or the bank's booking date. Rates have changed in recent years:

- the general rate is 25.5 % from 1 September 2024
- the reduced 14 % rate fell to 13.5 % on 1 January 2026.

The program picks the percentage itself from its VAT rate table once the agent
has given the treatment, the rate category and the date. The agent does not
write VAT lines by hand on the receipt route or for purchases on the bank
statement.

## Treatment is decided at sealing

The VAT treatment is stored in the voucher at the moment of sealing, and the
VAT return is calculated from these stored codes. The program does not infer
the VAT treatment afterwards from the account number. That is why:

- A VAT posting without a VAT code does not show in the return, even if it is
  on a VAT account. This applies especially to manual entries, such as sales
  invoices, where the agent puts the VAT postings together itself
  ([Record a sales invoice, payroll or another manual entry](../tasks/record-other-entries.md)).
  Even then the program does not infer the treatment from the account, but
  it notices the disagreement: the VAT return preview compares the VAT
  accounts' movement with the return's figures, and direct filing refuses if
  they do not agree.
- A wrong treatment is corrected with a correcting voucher, and the correction
  affects the return of the period in which it is recorded.

## VAT on sales

The bank statement import and the receipt route split out VAT only for
purchases. A sale that includes VAT is recorded as a manual entry, for example
from the sales invoice, and the customer's payment is recorded from the bank
statement against trade receivables.

## What this means for you

- When you approve a purchase, look at the treatment and the invoice date,
  not just the amount.
- When the agent asks about VAT, it does not know the answer. Find it out from
  the receipt or the seller.
- If a purchase is unusual (foreign, partly private, an import), ask your
  bookkeeper.

## See also

- [Record receipts and purchase invoices](../tasks/record-receipts.md)
- [File the VAT return in OmaVero](../tasks/file-vat-omavero.md)
- [Sealing and approval](sealing-and-approval.md)
