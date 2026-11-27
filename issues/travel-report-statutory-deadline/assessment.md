# Assessment of the travel-report-statutory-deadline review

Assessed against `4a98975`.

The review produced no findings, so there is nothing to triage.

FIX: 0   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

One observation from the host's own reading, not a review finding: the
employer's separate report deadline (`ReportMonth::deadline` in
`crates/domain/src/payroll.rs`) shows the 5th without the weekend shift, and
the status page says so. It is cautious, not wrong in the harmful direction,
and outside this issue's scope; it is proposed to the conductor rather than
filed.

## Simplicity

- Reused: `bank_days::{is_finnish_bank_day, next_finnish_bank_day}` for the
  shift, because 150/1930 5 § shifts the same weekdays the bank calendar
  already models; no second holiday table. `palkkatieto_deadline` untouched.
- Repetition: the travel deadline rule lives in `travel_report_deadline`;
  `WageReport::deadline` and `report_deadline` both call it.
- Left out on purpose: a taxable income type's deadline (refused by
  `report_deadline` until one is modelled, rather than guessed); the paper
  form's eight-day rule (aggountant files electronically); changing the
  employer report's deadline.
