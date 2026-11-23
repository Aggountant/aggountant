# File the VAT return in OmaVero

When a month is closed, the agent calculates the figures for its VAT return
from the books. You enter them yourself in OmaVero, the Tax Administration's
online service, and the agent records the return in aggountant as filed. Do it
this way if direct filing is not set up or you prefer to file yourself.

## Before you start

- The month is closed, meaning its state is `sealed`
  ([Close the month](close-month.md)).
- You can log in to OmaVero on behalf of the company.

## How to do it

1. **Ask the agent to show the figures.**

   > Show the figures for March's VAT return.

2. **The agent previews the return** and shows its numbered fields in euros,
   the tax on sales and on purchases, the net amount and the amount to pay.
   For example, a month with just one €124.00 purchase receipt gives:

   | Field | Amount |
   |---|---|
   | 307 Kohdekauden vähennettävä vero (deductible tax for the period) | €25.20 |
   | 308 Maksettava tai palautukseen oikeuttava vero (tax payable or refundable) | −€25.20 |
   | To pay | €0.00 |

   The preview also reconciles the VAT accounts with the return: it compares
   the month's movement on the VAT accounts (for example 2939 and 1763) with
   the return's tax on sales and on purchases. When they agree, the result's
   `vat_reconciliation.status` is `reconciled`. A difference of a few cents on
   vouchers that carry a VAT code is normal, because the program calculates
   the tax line by line.

   The state `unreconciled` means that a VAT account has postings without a
   VAT code and tax is missing from the return. The preview then shows the
   code `VAT_ACCOUNTS_UNRECONCILED`, the voucher numbers and the missing
   amount. Sort this out before you file the figures
   ([If something goes wrong](#if-something-goes-wrong)).

3. **Enter the fields in OmaVero** and send the return. You can also prepare
   an Ilmoitin.fi file yourself, because aggountant does not make one
   (Ilmoitin.fi is the Tax Administration's service for uploading return
   files).

4. **Pay the tax yourself**, if there is any to pay, with the OmaVero
   reference in your bank or in OmaVero. aggountant does not pay anything.

5. **Save the return's confirmation from OmaVero**, for example as a PDF, and
   give it to the agent.

   > Here is the confirmation of March's VAT return from OmaVero. Record the
   > return as filed.

6. **Approve the mark.** The agent archives the confirmation and marks the
   month as filed. The mark is permanent, so approve it only once the return
   has really been sent.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant file-vat preview --period 2026-03
aggountant artifact attach --file omavero-confirmation-2026-03.pdf
aggountant file-vat complete-manual --period 2026-03 --channel omavero \
  --evidence-ref <the confirmation's identifier>
aggountant period show --period 2026-03
```

`file-vat preview` only reads the books: it starts nothing, reads no
credentials and makes no network connection. That is why you can also run it
for a month that has already been filed and use it to check earlier returns.
It does not check whether direct filing is set up, so its guidance names both
routes: direct filing when `aggountant vero-fi status` reports ready, and
otherwise the manual route on this page. For a quarterly period, the period is
given as `2026-Q1`.

The reconciliation leaves out VAT payments, refunds and transfers between VAT
accounts, because they do not belong in a return. They are left out even when
the same voucher books interest or a late fee. The program recognises a
payment or refund by its money side going through a bank account or the tax
account. The chart of accounts marks these accounts as settlement accounts: in
the standard chart they are 1910 and 1920 but not the cash account 1900. If
the same voucher has any other balance-sheet item, such as an advance
received, a fixed asset, a payable or equity, the program does not treat the
posting as a payment and checks it.

`file-vat complete-manual` sends and pays nothing. The channel is `omavero` or
`ilmoitin`. If an uncertain direct submission was left for the same month, the
command also requires `--remote-status-evidence-ref` with evidence of the Tax
Administration's status.
</details>

## Done when

The month's state shows `vat_filed true`:

```
period "2026-03" — "sealed" (…, vat_filed true)
```

## Limitations

aggountant cannot tell late-payment interest from a credit that is booked
without a VAT code in the same voucher as a VAT payment. The credit passes as
part of the payment when it is smaller than the VAT paid and is booked on an
expense account (a sales credit) or an income account (a purchase credit), or
when its counter-entry is a liability debited beside the payment, such as an
advance received. So record credits in their own vouchers and with a VAT code.

The other limitations of the VAT return are in
[What is ready and what is not](../reference/status.md#vat-return).

## If something goes wrong

- `PERIOD_NOT_SEALED`: the month has not been closed, or the close is in
  progress. [Close the month](close-month.md) first.
- `VAT_PERIOD_NOT_SEALED`: you previewed a month that is not closed yet. The
  figures are a draft for checking (`fileable: false`) and cannot be filed
  before the close.
- `VAT_ACCOUNTS_UNRECONCILED`: the listed vouchers have tax on a VAT account
  without a VAT code. In the books the tax is correctly on the VAT account,
  but it is missing from the preview's figures. Ask the agent to show the
  vouchers and go through them against the source.
  - If the tax belongs in this month's return, file the month in OmaVero with
    corrected figures: add the missing tax and its base to the field for the
    source's VAT rate. A sales invoice whose VAT code was forgotten belongs in
    the return, and so does a refunded advance, a returned fixed-asset
    purchase or a private-use correction of input VAT booked without a VAT
    code. A closed month cannot be reopened, and direct filing does not send
    corrected figures. Do not record the same tax again with a VAT code in the
    next month, because it would then be reported twice. If the month has
    already been filed with figures that are too low, make a replacement
    return, described at the end of this list.
  - If the posting does not belong in a return, the figures are right and you
    can file them. This is the case, for example, when a refund from the Tax
    Administration is recorded on account 2939 or the posting is the VAT
    balance of an opening voucher. In direct filing
    [the agent records the reason in the filing run](file-vat-direct.md#if-something-goes-wrong).
- `VAT_SETTLEMENT_ROLES_MISSING`: shown in the preview beside the previous
  code. The listed vouchers were recorded under a chart of accounts whose
  settlement accounts are not marked, so aggountant cannot tell a VAT payment
  or refund from a correction. The books may still be in order. If you use
  aggountant's standard chart, ask the agent to
  [upgrade the home's master data](upgrade.md) and run the preview again.
  With the company's own chart, the agent marks the settlement accounts by
  hand in `masterdata/reference/settlement-account-roles.json`, because there
  is no command for it. If the vouchers are
  VAT payments or refunds, the agent can also acknowledge them in the filing
  run like other postings that do not belong in a return.
- The command asks for evidence of the Tax Administration's status, because a
  direct submission with an uncertain outcome was left for the same month.
  Find out first whether it got through, so that the return is not filed
  twice:
  [The outcome of a filing is uncertain](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain).
- A return that has been filed has an error. Correct it with a replacement
  return
  ([Correct a return that has been filed](file-vat-direct.md#correct-a-return-that-has-been-filed)).
  If you make the replacement return in OmaVero, the agent records it with its
  receipt (`file-vat complete-manual --replacement`).

## See also

- [Send the VAT return directly to the Tax Administration](file-vat-direct.md)
- [Filings and uncertain outcomes](../concepts/filings.md)
- [Periods, closing and the fiscal year](../concepts/periods.md)
