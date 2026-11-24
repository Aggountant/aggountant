---
created: 2026-11-24
updated: 2026-11-24
type: bug
reporter: jari
status: open
priority: high
lane: wage-reporting
lane_seq: 15
collision: [crates/domain/src/wage_report.rs, skills/tulorekisteri/SKILL.md]
---

# file-wage: show the statutory deadline for tax-exempt travel reports

## Problem

`file-wage` shows the deadline of a tax-exempt travel report (income types 311
and 331) as five calendar days after the payment date
(`WageReport::deadline` in `crates/domain/src/wage_report.rs`). For tax-exempt
travel expense reimbursements the statutory deadline is later: they can be
reported once a month, by the fifth day of the month following the payment
month, and several payments of one month can go in one report carrying the
latest payment date (laki tulotietojärjestelmästä 53/2018, 12 §; Verohallinto,
"Reporting reimbursements of expenses to the Incomes Register":
https://www.vero.fi/en/incomes-register/companies-and-organisations/employers/earnings-payment-data/reporting-reimbursements-of-expenses-to-the-incomes-register/).

The five-day figure is the general deadline for pay data. Showing it for
travel reimbursements is cautious, but it is not the true deadline, and it
pushes users to report each payment separately.

## Expected

- `file-wage preview` / `run show` / the guide show the statutory deadline for
  a travel report: the 5th of the month after the payment month. If the 5th
  falls on a weekend or public holiday, follow what the Incomes Register says
  about that (check the primary source; do not assume).
- Taxable income types, if they are ever added, keep the five-day rule; the
  deadline is a function of the income type.
- The guide and the `tulorekisteri` skill say that one monthly report may
  combine a month's payments, with the latest payment date.

## Acceptance criteria

- [ ] Deadline for a 311/331 report = 5th of the following month, with tests
      (month end, December → January, weekend/holiday per the source).
- [ ] `palkkatieto_deadline` for pay data unchanged.
- [ ] Guide (fi/en) and `skills/tulorekisteri/SKILL.md` updated.

## Decisions

### 2026-11-24T18:08:01Z · @conductor

Jari 24.11.2026: show the correct (statutory) deadline, not the cautious five days.
