# Result: travel-report-statutory-deadline

`file-wage preview` now shows the statutory deadline of a tax-exempt travel
report (income types 311 and 331): the 5th of the month after the payment
month, or the next business day when the 5th is a Saturday, Sunday or other
holiday. Before, it showed five days after the payment date.

## Sources

- Laki tulotietojärjestelmästä (53/2018) 12 § 11 mom.: tax-exempt
  reimbursements of expenses are reported "kuukausittain viimeistään
  maksukuukautta seuraavan kalenterikuukauden viidentenä päivänä".
- Verohallinto, "Reporting reimbursements of expenses to the Incomes
  Register": one monthly report may combine a month's payments and carries
  the latest payment date.
- Verohallinto, "Wages: reporting data to the Incomes Register" §2.1: a
  deadline on a Saturday, Sunday or other holiday may be met on the following
  business day under laki säädettyjen määräaikain laskemisesta (150/1930) 5 §.
  That section's weekday holidays are the Finnish bank holidays already in
  `crates/domain/src/bank_days.rs`.

## What changed

- `crates/domain/src/wage_report.rs`: `travel_report_deadline(payment_date)`
  and `report_deadline(income_type, payment_date)` (311/331 only; other
  income types are refused, not guessed). `WageReport::deadline` uses it.
  The expense-claim status slice can call `report_deadline`.
- `palkkatieto_deadline` (pay data, five days) is unchanged and tested.
- `file-wage preview --help` input text, the guide (fi/en task page and
  status page), `skills/tulorekisteri/SKILL.md`, CHANGELOG.

## Acceptance criteria

- [x] 311/331 deadline = 5th of the following month, tested for month end,
      December → January, weekend and public holidays per the source.
- [x] `palkkatieto_deadline` unchanged.
- [x] Guide (fi/en) and `skills/tulorekisteri/SKILL.md` updated.

Companies without travel reports see no change: no other command's output
moved (only the three deadline assertions of `file-wage preview` changed in
the existing suite).

Review: `review.md`, assessment: `assessment.md` (no findings).
