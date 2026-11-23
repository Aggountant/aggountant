# How the VAT treatment is decided

The agent proposes a VAT treatment for every purchase. When the receipt does
not show how the purchase is taxed, it asks you, even if the answer seems
obvious. The figures in the VAT return are calculated from the treatments
stored in the vouchers.

## The VAT treatment is always a named choice

A purchase is recorded with one of these treatments:

- In a domestic purchase the receipt's amount includes VAT, and the deductible
  tax is calculated from it.
- Under the reverse charge the buyer accounts for the tax, for example when a
  service is bought from another EU country. There are three kinds: EU
  services, EU goods and services from outside the EU.
- A VAT-exempt purchase is recorded when the exemption has been confirmed from
  the receipt or the company's practices.

If a decisive detail is missing, the agent marks the receipt as needing
clarification and asks you what has to be found out. It does not guess,
because a purchase guessed to be exempt is an error nobody notices before the
VAT return.

[The receipt route does not handle](../reference/status.md#receipts-and-entries)
partial deduction rights or imports. The agent tells you if a purchase is one
of those.

## The rate is chosen from the receipt's date

The VAT rate depends on the product and on when the purchase was made. That
is why the rate is chosen from the date of the receipt or invoice, not the
payment date or the bank's booking date. The date matters because rates have
changed in recent years: the general rate has been 25.5% since 1 September
2024, and the reduced 14% rate fell to 13.5% on 1 January 2026.

The agent gives the program the treatment, the rate category and the date,
and the program picks the percentage from its VAT rate table. On the receipt
route and for purchases on the bank statement the agent does not write VAT
lines by hand.

The zero-rate categories of sales do not fit a purchase. The program refuses
them and names the treatment that fits (`VAT_TREATMENT_CLASS_MISMATCH`). The
one exception is the exempt intra-Community acquisition of goods, described in
the next section.

## Purchases from another EU country

For a reverse-charge purchase the rate category is the Finnish rate that
applies to the goods or service. That it came from another EU country is given
separately. So a taxable intra-Community acquisition of goods uses the Finnish
rate category (for example `domestic-standard`) even when the seller's invoice
shows no VAT. The invoice shows none because the buyer accounts for it, so its
absence does not make the purchase exempt.

A service bought from another EU country whose supply the Finnish VAT Act
exempts, for example an insurance or financial service, is recorded as a
VAT-exempt purchase (`{"type":"no-vat"}`) and not as a reverse charge. It gives
rise to no Finnish VAT and no deductible tax, and it does not appear in the VAT
return. According to the instructions of Verohallinto (the Finnish Tax
Administration),
[Instructions for completing VAT returns](https://www.vero.fi/en/businesses-and-corporations/taxes-and-charges/vat/when-to-file-and-pay/instructions-for-completing-the-vat-return/),
services under the general rule whose supply is exempt are not reported under
"Palveluostot muista EU-maista" (purchases of services from other EU member
states). Section 9 of the detailed guidance
[Value-added taxation of cross-border supply and acquisition of services](https://www.vero.fi/en/detailed-guidance/guidance/48679/value-added-taxation-of-cross-border-supply-and-acquisition-of-services/)
says that box holds the services on which the buyer is liable for Finnish VAT.

Goods follow a different rule. An exempt intra-Community acquisition of goods
is reported in the VAT return, but no tax is paid or deducted on it. An
acquisition is exempt, for example, when importing the goods into Finland
would be exempt or the goods are placed in a VAT warehouse. It is recorded as
a reverse charge with the zero-rate category:

```json
{"type":"reverse-charge","rate_class":"zero-rated","scope":"eu-goods"}
```

The amount is the price without VAT, and no VAT account is posted. In the
return the value shows only under "Tavaraostot muista EU-maista" (purchases of
goods from other EU member states), the tax on goods purchases is zero and the
deductible tax does not change (chapters 6.6 and 12 of the detailed guidance
[Arvonlisäverotus EU-tavarakaupassa](https://www.vero.fi/syventavat-vero-ohjeet/ohje-hakusivu/48691/arvonlisaverotus-eu-tavarakaupassa4/),
in Finnish). This is the only purchase treatment that accepts the zero-rate
category. It cannot carry a partial deduction, because there is no tax to
deduct.

The exempt treatment is used only when the exemption has been confirmed,
because the program cannot tell it from a taxable acquisition given
`zero-rated` by mistake.

## Treatment is decided at sealing

The VAT treatment is stored in the voucher when the voucher is sealed, and the
VAT return is calculated from these stored codes. The program does not infer
the treatment afterwards from the account number.

So a VAT posting without a VAT code does not show in the return, even if it is
on a VAT account. This can happen especially in
[manual entries](../tasks/record-other-entries.md), such as sales invoices,
where the agent puts the VAT postings together itself. The program then
notices the disagreement: the VAT return preview compares the VAT accounts'
movement with the return's figures, and direct filing refuses if they do not
agree.

A wrong treatment is corrected with a correcting voucher, and the correction
affects the return of the period in which it is recorded.

## VAT on sales

The bank statement import and the receipt route split out VAT only for
purchases. A sale that includes VAT is recorded as a manual entry, for example
from the sales invoice, and the customer's payment is recorded from the bank
statement against trade receivables.

## What this means for you

- When you approve a purchase, check the treatment and the invoice date as
  well as the amount.
- When the agent asks about VAT, find the answer from the receipt or the
  seller.
- If a purchase is unusual (foreign, partly private, an import), ask your
  bookkeeper.

## See also

- [Record receipts and purchase invoices](../tasks/record-receipts.md)
- [File the VAT return in OmaVero](../tasks/file-vat-omavero.md)
- [Sealing and approval](sealing-and-approval.md)
