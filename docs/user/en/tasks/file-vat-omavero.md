# File the VAT return in OmaVero

When a month is closed, its VAT return is made with figures calculated from
the books. On this page you enter the figures yourself in OmaVero, the Tax
Administration's online service, and record the return in aggountant as
filed. Do it this way if direct filing is not set up or you prefer to file
yourself.

## Before you start

- The month is closed, meaning its state is `sealed`
  ([Close the month](close-month.md)).
- You can log in to OmaVero on behalf of the company.

## How to do it

1. **Ask the agent to show the figures.**

   > Show the figures for March's VAT return.

2. **The agent previews the return.** The preview only reads the books: it
   starts nothing, reads no credentials and makes no network connection. The
   result has the return's numbered fields in euros, the tax on sales and on
   purchases, the net amount and the amount to pay. For example, a month with
   just one €124.00 purchase receipt gives:

   | Field | Amount |
   |---|---|
   | 307 Kohdekauden vähennettävä vero (deductible tax for the period) | €25.20 |
   | 308 Maksettava tai palautukseen oikeuttava vero (tax payable or refundable) | −€25.20 |
   | To pay | €0.00 |

3. **Enter the fields in OmaVero** and send the return. You can also prepare
   an Ilmoitin.fi file yourself (Ilmoitin.fi is the Tax Administration's
   service for uploading return files). aggountant does not make one.

4. **Pay the tax**, if there is any to pay, with the OmaVero reference in your
   bank or in OmaVero. aggountant does not pay anything.

5. **Save the confirmation.** Save the return's confirmation from OmaVero,
   for example as a PDF, and give it to the agent.

   > Here is the confirmation of March's VAT return from OmaVero. Record the
   > return as filed.

6. **Check and approve.** The agent archives the confirmation and marks the
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

The channel is `omavero` or `ilmoitin`. The command sends and pays nothing.
If an uncertain direct submission was left for the same month, the command
also requires `--remote-status-evidence-ref` with evidence of the Tax
Administration's status. For a quarterly period, the period is given as
`2026-Q1`.
</details>

## Done when

The month's state shows `vat_filed true`:

```
period "2026-03" — "sealed" (…, vat_filed true)
```

## Limitations

- The preview's guidance text always tells you to enter the figures by hand,
  even when direct filing is set up. The text is out of date.
- aggountant does not make corrected or replacement returns. If the filed
  figures need correcting, make a replacement return in OmaVero.

More detail: [What is ready and what is not](../reference/status.md#vat-return).

## If something goes wrong

- **`PERIOD_NOT_SEALED`.** The month has not been closed, or the close is in
  progress: [Close the month](close-month.md).
- **The command asks for evidence of the Tax Administration's status.** A
  direct submission with an uncertain outcome was left for the same month.
  Find out first whether it got through, so that the return is not filed
  twice:
  [When something goes wrong](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain).

## See also

- [Send the VAT return directly to the Tax Administration](file-vat-direct.md)
- [Filings and uncertain outcomes](../concepts/filings.md)
- [Periods, closing and the fiscal year](../concepts/periods.md)
