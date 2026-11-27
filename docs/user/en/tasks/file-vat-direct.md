# Send the VAT return directly to the Tax Administration

The agent sends a closed month's VAT return to the Tax Administration through
the Vero API, using your company's own credentials, once you have approved
the return. A return that has been sent cannot be withdrawn, but it can be
[replaced](#correct-a-return-that-has-been-filed).

## Before you start

- The month is [closed](close-month.md).
- [Direct filing is set up](set-up-vero-api.md), meaning the readiness check
  shows `ready`.
- The month has not yet been reported to the Tax Administration in any way.
- Your company's [Tax Administration payment details](#store-the-tax-administration-payment-details-once)
  are stored.

## Store the Tax Administration payment details once

The payment proposal of a VAT return carries the Tax Administration's bank
account number and the reference number your company pays VAT with. With a
wrong reference the payment goes to another tax, so the program takes the
reference from the company's stored payment details and never makes one up.
Without a reference, sending a VAT return does not start
(`TAX_PAYMENT_REFERENCE_MISSING`).

1. **Look up the payment details in OmaVero:** Maksutilanne (payment status) >
   Verojen maksaminen ja maksutiedot (paying taxes and payment details). A
   limited company or other corporate entity has a general tax reference
   number that pays almost every tax, VAT and prepayments included. A sole
   trader, or an entity that does not use the general reference, pays VAT
   with the reference for self-assessed taxes.
2. **Give them to the agent,** for example:

   > Store the Tax Administration payment details: account
   > FI21 1234 5600 0007 85, general tax reference 7654 3216.

   The agent stores them and shows them to you. Check them against OmaVero.

The payment details are stored in the home, in
`masterdata/tax-payments.json`; they are not secret, and the file belongs in
version control. The program checks the check digits of the reference and the
account number. It does not accept the general reference together with
tax-specific references, because the Tax Administration advises paying every
tax the same way; otherwise payments can go to other taxes than intended. A
payment arrangement reference can be stored, but it is never used in a VAT
payment proposal.

The Tax Administration's Danske Bank account (starting `FI56`) is replaced by
an OP account on 1 December 2026 and closed at the end of May 2027. If the
stored account is that one, the program warns (`TAX_PAYEE_IBAN_RETIRING`):
replace it with the Nordea account or the new OP account from OmaVero.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant tax-payments set --dry-run < payment-details.json
aggountant tax-payments set < payment-details.json
aggountant tax-payments show
```

The input is the whole set of payment details and replaces what is stored:
the fields are `payee_iban`, `general_reference`, `self_assessed_reference`,
`prepayment_reference` and `payment_arrangement_reference`, all optional,
plus `schema_version` 1. A reference is given without spaces.
`tax-payments show` tells in `vat_reference` which reference VAT is paid
with.
</details>

## How to do it

1. **Ask the agent.**

   > File March's VAT return directly with the Tax Administration.

2. **The agent previews the figures** from the closed month's books, the same
   way as for [a return filed in OmaVero](file-vat-omavero.md).

3. **The agent prepares the submission.** It gives the program the month and
   the tax on sales and on purchases. The program takes the account number
   and the reference from the stored payment details. It calculates the figures from
   the books once more and refuses if they differ or if the VAT accounts do
   not reconcile with the return (`VAT_ACCOUNTS_UNRECONCILED`). The program
   then checks readiness, asks the Tax Administration for the month's status
   and waits for your approval. Nothing has been sent yet.

4. **Check and approve.** The agent shows the company, the month, the due
   date, the numbered fields, the net amount and the amount to pay. Check that
   the fields are the same as in the preview and that the month is right. The
   approval covers exactly the content the agent showed.

   > I approve the return.

   If you reject, the run ends and nothing is sent. A new round starts from
   the preview.

5. **The program sends the return.** When the Tax Administration has received
   it, the program stores the acknowledgement (an identifier and a timestamp)
   and marks the month as filed.

   A return usually shows in OmaVero only about half a day later. The agent
   can ask the Tax Administration for its status without sending anything.
   **Do not send the return again because it does not show yet.** What to do
   instead is under
   [The return does not show in OmaVero](../troubleshooting.md#the-return-does-not-show-in-omavero).

6. **Pay the tax yourself**, if there is any to pay. aggountant does not pay
   it. The run is complete once the return is filed, and `run show` gives the
   amount, the account, the reference and the due date.

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

`file-vat start` takes the account number and the reference from the stored
payment details. The input field `payee_iban` is optional, and if it differs
from the stored account the command refuses (`TAX_PAYEE_IBAN_MISMATCH`). If
the agent acknowledges untagged postings on the
VAT accounts, the input also has the field
`vat_reconciliation_acknowledgement`, with the reason (`reason`) and the
voucher numbers the preview lists (`vouchers`).

Before the approval, `file-vat start` reads the period from the Tax
Administration. If that read fails or the period is not open for filing, the
command fails with an error that gives the reason and names the run. Nothing
is sent, and `run show` shows the run as `not_filed`, with the code on the
rejected `reconcile` effect (`error_code`). The same happens when the figures
do not match the books or the VAT accounts do not reconcile. A `not_filed`
run is finished and does not stop a new one. The program does not retry. Once
the cause is fixed, the agent starts a new run.

The run waits at the approval `ApproveFiling`; rejecting is
`--answer reject`. Once the return is filed, the run is `completed`. When there
is tax to pay, the `submit-payment` effect is `skipped` and its `payment` field
has `state` `left_to_user`, `amount`, `payee_iban`, `reference` and
`due_date`: the user
pays it, and nothing says it has been paid. A run that an earlier version left
waiting at `ApprovePayment` is completed with
`file-vat user-pays --run-id fv-…`, which sends nothing.

The acknowledgement is in the output of `approve-filing` and `run show`, on
the `submit-filing` effect: `outcome.receipt.unique_identifier` and
`outcome.receipt.accepted_timestamp`. If `outcome.state` is
`outcome_unknown`, `outcome.next_command` names the command that settles it.
</details>

## Done when

- The agent tells you the identifier and timestamp of the Tax
  Administration's acknowledgement.
- The month's state shows `vat_filed true`.

## Correct a return that has been filed

A replacement return gives all the figures of the period again, corrected,
and the Tax Administration uses it from then on.

1. **Correct the books first.** The filed month is locked, so the agent
   records the [correcting voucher](correct-voucher.md) in the next open
   month. Tell the agent that the voucher corrects the VAT return of the
   filed month. The agent marks that on the voucher when it is sealed
   (`corrects_vat_period`), and the program then counts the voucher's tax in
   the corrected month's figures and not in the return of the month the
   voucher is recorded in. The mark cannot be added after sealing.

   According to the Tax Administration's instructions, a financially minor
   error may also be corrected in the current period's return. In that case
   the agent leaves the mark out and no replacement return is made. The
   choice is yours.

2. **Ask for the replacement return.**

   > Make a replacement return for October's VAT return.

   The agent previews the month's new figures and prepares the replacement.
   Tell it the reason for the correction: a calculation error or an error in
   filling in the return, a change in legal practice, guidance received in a
   tax audit, or an error in interpreting the law.

3. **Check and approve.** The agent shows the whole return and which figures
   change compared with the return the Tax Administration holds now. The
   approval covers this content.

4. **The program sends the replacement return** and stores its
   acknowledgement next to the original's. Both are kept, and the latest is
   in force.

5. **Check the tax account in OmaVero.** A replacement return changes the
   period's tax by the difference. The program neither calculates nor pays
   the difference, so pay what is due yourself or wait for the refund.

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
that change.

`file-vat status` and `file-vat preview` show every filing of the period
with its acknowledgement in the field `filings`.
</details>

If you have already made the replacement return in OmaVero, ask the agent to
record it with its receipt (`file-vat complete-manual --replacement`).

## Limitations

- aggountant does not pay the tax and does not follow whether it has been
  paid; the tax account in OmaVero shows that.
- Direct filing also works for quarterly periods, but in real use it has
  been tried with monthly periods.
- A replacement return has not yet been sent to the Tax Administration's
  production service.

The limitations are described in full on
[What is ready and what is not](../reference/status.md#vat-return).

## If something goes wrong

- `VERO_NOT_READY`: readiness is not in order. The blockers and their fixes
  are on [Vero.fi readiness blockers](../reference/vero-fi-blockers.md).
- `VAT_FIGURES_MISMATCH`: the figures given do not match the books. The
  agent uses the preview's figures.
- `VAT_ACCOUNTS_UNRECONCILED`: a VAT account has postings without a VAT
  code, and tax would be missing from the return. Nothing was sent. If the
  tax belongs in the return,
  [file the return in OmaVero with corrected figures](file-vat-omavero.md#if-something-goes-wrong).
  If the postings really belong outside the return, tell the agent the
  reason. That is the case when, for example, a refund from the Tax
  Administration is recorded on account 2939 or the posting is the VAT
  balance of an opening voucher. The agent attaches the reason and the
  voucher numbers the preview lists to the filing run, and the reason is
  stored in the run.
- `VAT_RECONCILIATION_ACK_MISMATCH`: the vouchers given with the reason are
  not exactly the same as in the preview, so the program refuses.
- `PERIOD_TRANSITION_CONFLICT`: the month has already been filed, and the
  program does not file it again.
- `VERO_VAT_RETURN_NOT_PROCESSED`: the Tax Administration has not yet
  processed the return in force, so a replacement return cannot be sent. Try
  again later.
- If the connection dropped or the service returned an error during sending,
  [the outcome of the filing is uncertain](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain).
  The uncertain outcome of a replacement return is resolved the same way,
  and another replacement is never sent on a guess.
- `VAT_FILING_UNRESOLVED`: an earlier filing of the same month may have got
  through, so nothing is sent. The agent asks the Tax Administration
  (`file-vat reconcile`). If the return is there, the earlier run ends as
  filed. If the answer is "not found", the agent asks again at the same time
  of day on the next bank day or later. The second "not found" closes the
  earlier run, and the month can be filed with a new run. This can take a
  few days, and the reason is under
  [Uncertain outcome](../concepts/filings.md#uncertain-outcome). If the
  earlier run was interrupted while sending, the agent first asks with
  `file-vat status`, as
  [A VAT filing was interrupted while sending](../troubleshooting.md#a-vat-filing-was-interrupted-while-sending)
  describes.

## See also

- [Filings and uncertain outcomes](../concepts/filings.md)
- [File the VAT return in OmaVero](file-vat-omavero.md)
- [Set up direct VAT filing](set-up-vero-api.md)
