# Send the VAT return directly to the Tax Administration

Once direct filing is set up, the agent sends a closed month's VAT return to
the Tax Administration through the Vero API, using your company's own
credentials, after you have approved it. A return that has been sent cannot
be withdrawn.

## Before you start

- The month is closed ([Close the month](close-month.md)).
- Direct filing is set up, meaning the readiness check shows `ready`
  ([Set up direct VAT filing](set-up-vero-api.md)).
- The month has not yet been reported to the Tax Administration in any way.

## How to do it

1. **Ask the agent.**

   > File March's VAT return directly with the Tax Administration.

2. **The agent previews the figures** from the closed month's books, as on
   [File the VAT return in OmaVero](file-vat-omavero.md).

3. **The agent prepares the submission.** It gives the program the month, the
   tax on sales and on purchases, and the Tax Administration's bank account
   number, which is shown in OmaVero. The program calculates the figures from
   the books once more and refuses if they differ. It checks readiness, asks
   the Tax Administration for the month's status and waits for your approval.
   Nothing has been sent yet.

4. **Check and approve.** The agent shows the company, the month, the due
   date, the numbered fields, the net amount and the amount to pay. Check that
   the fields are the same as in the preview and that the month is right. The
   approval covers exactly this content.

   > I approve the return.

   If you reject, the run ends and nothing is sent. A new round starts from
   the preview.

5. **The program sends the return.** When the Tax Administration has received
   it, the program stores the acknowledgement (an identifier and a timestamp)
   and marks the month as filed.

6. **Pay the tax yourself**, if there is any to pay, with the reference shown
   in OmaVero.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant vero-fi status
aggountant file-vat preview --period 2026-03
aggountant file-vat start < return.json
aggountant file-vat approve-filing --run-id fv-… --answer approve
aggountant run show --run-id fv-…
aggountant period show --period 2026-03
aggountant file-vat status --period 2026-03
```

In the filing run's input, the Tax Administration's bank account number is
the field `payee_iban`. The run first waits at the approval `ApproveFiling`,
and for a return with tax to pay finally at `ApprovePayment`. Rejecting is
`--answer reject`.
</details>

## Done when

- The agent tells you the identifier and timestamp of the Tax
  Administration's acknowledgement.
- The month's state shows `vat_filed true`.

A return usually shows in OmaVero only about half a day later. The agent can
ask the Tax Administration for its status without sending anything. **Do not
send it again because it does not show yet:**
[When something goes wrong](../troubleshooting.md#the-return-does-not-show-in-omavero).

## Limitations

- When there is tax to pay, the run finally opens a payment approval.
  aggountant has no payment connection, so this step never completes and the
  run keeps showing a waiting state. The return has still been filed. A
  return that gives a refund ends without a payment step.
- Direct filing has been tried with monthly periods.
- aggountant does not make replacement returns.

More detail: [What is ready and what is not](../reference/status.md#vat-return).

## If something goes wrong

- **`VERO_NOT_READY`.** Readiness is not in order:
  [Vero.fi readiness blockers](../reference/vero-fi-blockers.md).
- **`VAT_FIGURES_MISMATCH`.** The figures given do not match the books. The
  agent uses the preview's figures.
- **`PERIOD_TRANSITION_CONFLICT`.** The month has already been filed. It is
  not filed again.
- **The connection dropped or the service returned an error during
  sending.**
  [When something goes wrong](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain).

## See also

- [Filings and uncertain outcomes](../concepts/filings.md)
- [File the VAT return in OmaVero](file-vat-omavero.md)
- [Set up direct VAT filing](set-up-vero-api.md)
