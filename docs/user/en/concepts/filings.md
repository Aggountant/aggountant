# Filings and uncertain outcomes

aggountant sends the VAT return to the Tax Administration and the travel
allowance report to Tulorekisteri. If the connection drops while sending, the
program does not send the same return again on its own. The agent first finds
out whether the first one got through.

## Why a return is different from a voucher

You correct a wrongly sealed voucher with a new voucher in your own books. A
return that has been sent you cannot correct yourself, because it has legal
effect and stays in the authority's system. A second submission for the same
period would not withdraw the first; at the Tax Administration it would be a
replacement return. That is why every submission needs your approval, and the
approval covers exactly the content that is sent. For the VAT return, the
program calculates the figures from the books once more before sending.

Returns are sent with your company's own credentials. Filing on behalf of
another company is not supported.

## Received is not processed

When the Tax Administration receives a VAT return, it gives an
acknowledgement, which is an identifier and a timestamp, and aggountant marks
the period as filed. At that point the Tax Administration has received the
return but may not have processed it yet. OmaVero has shown a return sent
through the API only about half a day later, and according to the Tax
Administration's support a delay of about 12 hours is normal. The time comes
from experience and has not been promised.

## Uncertain outcome

Sometimes the program does not know whether a return got through: the
connection dropped during sending, the service returned an error, the service
reported success but sent no readable receipt, or the response was
contradictory. The return may have got through even though no clear response
came, so the program does not treat a success response without a receipt as a
rejection.

In that case the program does not send again, because the authority would
have two returns if the first submission had got through after all. The run
stays in a state that needs attention, and the agent asks the authority what
it has:

- If the authority has exactly the same return, the run ends without a new
  submission.
- If the return differs, or the authority shows something else for the
  period, the run keeps waiting. Compare the return in OmaVero.
- If the authority says it has no return for the period, that is not yet
  proof, because a return can take days to show. The answer is recorded in
  the run, and the agent asks again later.

A VAT filing run is closed as not filed when the Tax Administration has
answered "not found" twice, at least one bank day apart. The second query
counts from the same time of day on the next bank day at the earliest. Bank
days are Monday to Friday, except Finnish bank holidays such as Good Friday,
Easter Monday, Ascension Day, Midsummer Eve and the Christmas days. A query
that fails, or an answer the program cannot read, does not count. Once the run
is closed, a new filing run may send the period's return. Until then,
aggountant refuses to start or approve a new filing for the same period
(`VAT_FILING_UNRESOLVED`).

What you do yourself is described under
[The outcome of a filing is uncertain](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain).

If an uncertain API submission was left for the same period and you file by
hand, recording that return also requires evidence that the authority's
status has been checked. Otherwise the return made by hand could quietly be a
duplicate.

## What is special about Tulorekisteri

A Tulorekisteri report contains a personal identity code. The report's
checksum is derived partly from it, so the checksum is treated as sensitive:
it is not stored in git, in notes or in a conversation log. The program does
not show Tulorekisteri's error texts, because they too may contain a personal
identity code. You see the exact errors in Tulorekisteri's e-service.

The payment identifier or the payment date is not changed to get around a
rejection or an uncertainty, because a new identifier would be a new report
to Tulorekisteri.

## What aggountant does not do

aggountant does not pay the tax. You pay VAT yourself with the OmaVero
reference. It does not cancel an employer's separate report either. That is
done in Tulorekisteri's e-service.

A replacement VAT return, the replacement and the cancellation of a travel
allowance report, and the replacement of an employer's separate report are
made with aggountant, each with its own preview and approval.

## What this means for you

- Approve a return only after you have looked at its fields.
- Do not send a return again because it does not show in OmaVero yet.
- When the outcome is uncertain, wait and let the agent ask for the status. Do
  not file by hand before you know whether the first submission got through.

## See also

- [Send the VAT return directly to the Tax Administration](../tasks/file-vat-direct.md)
- [Report travel allowances to Tulorekisteri](../tasks/report-travel-allowances.md)
- [Sealing and approval](sealing-and-approval.md)
