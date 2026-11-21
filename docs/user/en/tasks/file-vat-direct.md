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
   the books once more and refuses if they differ. It also refuses if the VAT
   accounts do not reconcile with the return (`VAT_ACCOUNTS_UNRECONCILED`). It
   checks readiness, asks
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
`--answer reject`. The acknowledgement is in the output of `approve-filing`
and `run show`, on the `submit-filing` effect: `outcome.receipt.unique_identifier`
and `outcome.receipt.accepted_timestamp`. If `outcome.state` is
`outcome_unknown`, `outcome.next_command` names the command that settles it.

Before the approval, `file-vat start` reads the period from the Tax
Administration. If that read fails or the period is not open for filing, the
command fails with the reason, nothing is sent, and the error names the run,
which `run show` shows as `needs_attention` with the `reconcile` effect
rejected. Nothing retries it; once the cause is fixed, start a new run.
</details>

## Done when

- The agent tells you the identifier and timestamp of the Tax
  Administration's acknowledgement.
- The month's state shows `vat_filed true`.

A return usually shows in OmaVero only about half a day later. The agent can
ask the Tax Administration for its status without sending anything. **Do not
send it again because it does not show yet:**
[When something goes wrong](../troubleshooting.md#the-return-does-not-show-in-omavero).

## Correct a return that has been filed

A filed return cannot be withdrawn, but it can be replaced. A replacement
return gives all the figures of the period again, corrected, and the Tax
Administration uses it from then on.

1. **Correct the books first.** The filed month is locked, so the correcting
   voucher is recorded in the next open month
   ([Correct a sealed voucher](correct-voucher.md)). Tell the agent that the
   voucher corrects the VAT return of the filed month. The agent marks that
   on the voucher when it is sealed (`corrects_vat_period`), and the
   voucher's tax is then counted in the corrected month's figures and not in
   the return of the month the voucher is recorded in. The mark cannot be
   added after sealing.

   According to the Tax Administration's instructions, a financially minor
   error may also be corrected in the current period's return. In that case
   the mark is left out and no replacement return is made. The choice is
   yours.

2. **Ask for the replacement return.**

   > Make a replacement return for October's VAT return.

   The agent previews the month's new figures and prepares the replacement.
   Tell it the reason for the correction: a calculation error or an error in
   filling in the return, a change in legal practice, guidance received in a
   tax audit, or an error in interpreting the law.

3. **Check and approve.** The agent shows the whole return and which figures
   change compared with the return the Tax Administration holds now. The
   approval covers exactly this content, as with the first return.

4. **The program sends the replacement return** and stores its
   acknowledgement next to the original's. Both are kept, and the latest is
   in force.

5. **Check the tax account in OmaVero.** A replacement return changes the
   period's tax by the difference. The program neither pays nor calculates
   the difference: pay what is due yourself, or wait for the refund.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant voucher seal < correction-seal.json
aggountant file-vat preview --period 2026-10
aggountant file-vat replace < replacement.json
aggountant file-vat approve-filing --run-id fv-… --answer approve
aggountant file-vat status --period 2026-10
```

`file-vat replace` takes the same input as `file-vat start` plus the field
`replacement_reasons`. `replacement.changes` in the answer lists the fields
that change. `file-vat status` and `file-vat preview` show every filing of
the period with its acknowledgement in the field `filings`.
</details>

A replacement return cannot be sent before the Tax Administration has
processed the return in force (`VERO_VAT_RETURN_NOT_PROCESSED`); try again
later. If the outcome of the submission stays uncertain, it is resolved as
for a first return: another replacement is never sent on a guess
([When something goes wrong](../troubleshooting.md)).

If you have already made the replacement return in OmaVero, ask the agent to
record it with its receipt (`file-vat complete-manual --replacement`).

## Limitations

- When there is tax to pay, the run finally opens a payment approval.
  aggountant has no payment connection, so this step never completes and the
  run keeps showing a waiting state. The return has still been filed. A
  return that gives a refund ends without a payment step.
- Direct filing has been tried with monthly periods.
- A replacement return neither pays nor calculates the difference to pay.

More detail: [What is ready and what is not](../reference/status.md#vat-return).

## If something goes wrong

- **`VERO_NOT_READY`.** Readiness is not in order:
  [Vero.fi readiness blockers](../reference/vero-fi-blockers.md).
- **`VAT_FIGURES_MISMATCH`.** The figures given do not match the books. The
  agent uses the preview's figures.
- **`VAT_ACCOUNTS_UNRECONCILED`.** A VAT account has postings without a VAT
  code, and tax would be missing from the return. Nothing was sent. If the
  tax belongs in the return, file the return in OmaVero with corrected
  figures ([File the VAT return in OmaVero](file-vat-omavero.md#if-something-goes-wrong)).
  If the postings really belong outside the return, for example a refund from
  the Tax Administration recorded on account 2939 or the VAT balance of an
  opening voucher, give the reason: the agent
  adds the field `vat_reconciliation_acknowledgement` to the filing run's
  input, with the reason (`reason`) and the voucher numbers the preview lists
  (`vouchers`). The reason is stored in the run. If the vouchers are not
  exactly the same, the program refuses with
  `VAT_RECONCILIATION_ACK_MISMATCH`.
- **`PERIOD_TRANSITION_CONFLICT`.** The month has already been filed. It is
  not filed again.
- **The connection dropped or the service returned an error during
  sending.**
  [When something goes wrong](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain).
- **`VAT_FILING_UNRESOLVED`.** An earlier filing of the same month may have
  got through, so nothing is sent. The agent asks the Tax Administration
  (`file-vat reconcile`). If the return is there, the earlier run ends as
  filed. If the answer is "not found", the agent asks again at the same time
  of day on the next bank day or later; the second "not found" closes the
  earlier run, and the month can
  be filed with a new run. This can take a few days
  ([Uncertain outcome](../concepts/filings.md#uncertain-outcome)). If the
  earlier run was interrupted while sending, the agent first asks with
  `file-vat status`
  ([A VAT filing was interrupted while sending](../troubleshooting.md#a-vat-filing-was-interrupted-while-sending)).

## See also

- [Filings and uncertain outcomes](../concepts/filings.md)
- [File the VAT return in OmaVero](file-vat-omavero.md)
- [Set up direct VAT filing](set-up-vero-api.md)
