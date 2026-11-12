# Periods, closing and the fiscal year

The books are closed piece by piece: first a month, finally the whole fiscal
year. Nothing more can be recorded in a closed period. This page explains
what periods are, what states they can be in and why an open matter can block
a close.

## The fiscal year and periods

aggountant treats the fiscal year as the calendar year. The year is closed
period by period, either monthly (`2026-03`) or quarterly (`2026-Q1`), but
you cannot use both in the same year. Monthly is the recommended and tried
option. A fiscal year other than the calendar year is not yet supported.

The VAT return is made for the same period the books are closed by: a month
or a quarter. Direct VAT filing has been tried with monthly periods.

## Period states

The period's state is shown whenever the agent shows it:

```
period "2026-03" — "sealed" (…, vat_filed false)
```

| State | What it means | Can you record in it? |
|---|---|---|
| `open` | the period has not been closed | yes |
| `closing` | the close has started and waits for your approval | no |
| `sealed` | the period is locked permanently | no |

`vat_filed` tells you whether the period's VAT return has been made. VAT
return figures are calculated only from a closed period, because only then
can they no longer change.

A period moves in one direction only. A close cannot be undone, and this
version has no command to make a period open again. If a close is rejected,
the period stays permanently in the state `closing`
([What is ready and what is not](../reference/status.md#rejected-close)).

## Why nothing is recorded in a closed period

Once a month's figures have been reported and its VAT filed, they must stay
the same. If a voucher could be added to a closed month, the filed VAT and the
books would no longer match. That is why a gap or mistake noticed later is
recorded in the next open period, as has always been done in bookkeeping:
[Correct a wrong voucher](../tasks/correct-voucher.md).

## Obligations

An obligation is an open matter with a deadline, for example a missing
mileage log. An open obligation blocks the close of its period
(`OPEN_OBLIGATION`), so that the matter is not forgotten under the close. An
obligation is marked resolved only when the matter has really been dealt
with.

In this version obligations are created only through the consultation route
for unusual receipts, and that route cannot be used yet. In ordinary use the
obligation list is therefore empty.

## Fiscal-year status

A fiscal year's state is `open` or `sealed`. A sealed fiscal year also has a
checksum of its financial statements. The fiscal year is sealed by the
year-end run, which cannot yet be used for real financial statements, so
normally the fiscal year is in the state `open`
([Prepare the financial statements](../tasks/year-end.md)).

## What this means for you

- Close a month only when it is finished: the bank account matches the
  statement and open items have been dealt with.
- If you notice a gap after the close has started, approve the close and
  record the gap in the next month. Do not reject the close.
- Keep the same period split for the whole year. Monthly is recommended.

## See also

- [Close the month](../tasks/close-month.md)
- [Sealing and approval](sealing-and-approval.md)
- [Filings and uncertain outcomes](filings.md)
