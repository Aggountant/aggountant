# Filings and uncertain outcomes

aggountant sends the VAT return to the Tax Administration and the travel
allowance report to Tulorekisteri. When sending, the program is more careful
than anywhere else: it never sends the same return again automatically, even
if the connection drops halfway. This page explains why and what it means for
you.

## Why a return is different from a voucher

A wrongly sealed voucher is corrected by a new voucher in your own books. A
return that has been sent cannot be corrected by you: it has legal effect and
stays in the authority's system. A second submission for the same period
would be a replacement return at the Tax Administration, not a withdrawal.
That is why every submission needs your approval, and the approval is bound
to exactly the content that is sent. For the VAT return, the program
calculates the figures from the books once more before sending.

Returns are always sent with your company's own credentials. Filing on behalf
of another company is not supported.

## Received is not processed

When the Tax Administration receives a VAT return, it gives an
acknowledgement (an identifier and a timestamp), and aggountant marks the
period as filed. The acknowledgement means the return has been received, not
that it has already been processed. In practice OmaVero has shown a return
sent through the API only about half a day later, and according to the Tax
Administration's support a delay of about 12 hours is normal. This is an
observation from practice, not a promised service time.

## Uncertain outcome

Sometimes the program does not know whether a return got through: the
connection dropped during sending, the service returned an error, or the
response was contradictory. The return may have got through even though no
response came.

In that case the program does not send again. If it did, and the first
submission had got through after all, the authority would have two returns.
Instead the run stays in a state that needs attention, and the agent asks the
authority what it has:

- If the authority has exactly the same return, the run ends without a new
  submission.
- If the return does not show or differs, the run keeps waiting, because the
  authority may not show the latest information yet. It is asked again later.

A return is never sent again blindly. What to do is described in
[When something goes wrong](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain).

The same care applies to a return made by hand. If an uncertain API
submission was left for the same period, recording a return made by hand also
requires evidence that the authority's status has been checked. Otherwise the
return made by hand could quietly be a duplicate.

## What is special about Tulorekisteri

A Tulorekisteri report contains a personal identity code. That is why:

- The report's checksum is derived partly from the personal identity code,
  and it is treated as sensitive: it is not stored in git, in notes or in a
  conversation log.
- The program does not show Tulorekisteri's error texts, because they may
  contain a personal identity code. You see the exact errors in
  Tulorekisteri's e-service.
- The payment identifier or the payment date is not changed to get around a
  rejection or an uncertainty. A new identifier would be a new report to
  Tulorekisteri.

## What aggountant does not do

- It does not pay the tax. You pay VAT yourself with the OmaVero reference.
- It does not make replacement returns or cancellations. They are made in
  OmaVero or in Tulorekisteri's e-service.

## What this means for you

- Approve a return only after you have looked at its fields.
- Do not send a return again because it does not show in OmaVero yet.
- When the outcome is uncertain, wait and let the agent ask for the status. Do
  not file by hand before you know whether the first submission got through.

## See also

- [Send the VAT return directly to the Tax Administration](../tasks/file-vat-direct.md)
- [Report travel allowances to Tulorekisteri](../tasks/report-travel-allowances.md)
- [Sealing and approval](sealing-and-approval.md)
