# Periods, closing and the fiscal year

The books are closed piece by piece: first a month, finally the whole fiscal
year. Nothing more can be recorded in a closed period. This page explains
what periods are, what states they can be in and why an open matter can block
a close.

## The fiscal year and periods

The fiscal year is the calendar year unless you define another one. A
company whose fiscal year runs, for example, from 1 July to 30 June defines
it once when the books are set up, and later years follow it. A first fiscal
year, or the year in which the fiscal year changes, can be shorter or longer
than 12 months, at most 18 months (Accounting Act 1:4). A fiscal year is
named by the calendar year it ends in: 1.7.2026–30.6.2027 is fiscal year
2027. Two fiscal years cannot end in the same calendar year; if you change
your fiscal year, use one longer transition year instead.

The year is closed period by period. A calendar fiscal year is closed either
monthly (`2026-03`) or quarterly (`2026-Q1`), but you cannot use both in the
same year; monthly is the recommended and tried option. A fiscal year that is
not the calendar year is always closed monthly.

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
| `closing` | the close has started and waits for you to approve or reject it | no |
| `sealed` | the period is locked permanently | no |

`vat_filed` tells you whether the period's VAT return has been made. VAT
return figures are calculated only from a closed period, because only then
can they no longer change.

A period moves in the order `open` → `closing` → `sealed`. While a close
waits for your approval you can still reject it: a rejection means the month
is not ready yet, and it returns the period to `open`. You can then record
what is missing and start a new close. Entries made during the close are kept;
nothing is removed. An approved close cannot be undone, and a sealed period is
not reopened.

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

A fiscal year's state is `open` or `sealed`. `aggountant fiscal-year show`
also shows the year's first and last day and its length in months. A sealed fiscal year also has a
checksum of its financial statements. The fiscal year is sealed by the
year-end run; until then the fiscal year is in the state `open`
([Prepare the financial statements](../tasks/year-end.md)).

## What this means for you

- Close a month only when it is finished: the bank account matches the
  statement and open items have been dealt with.
- If you notice a gap after the close has started, reject the close, record
  the gap and start the close again.
- Keep the same period split for the whole year. Monthly is recommended.

## See also

- [Close the month](../tasks/close-month.md)
- [Sealing and approval](sealing-and-approval.md)
- [Filings and uncertain outcomes](filings.md)
