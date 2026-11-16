---
created: 2026-11-16
updated: 2026-11-16
type: feature
reporter: jari
status: in-progress
priority: high
preparation: ready
lane: wage-reporting
lane_seq: 0
collision: [crates/broker/src/tulorekisteri.rs, crates/cli/src/wage.rs]
---

# File the employer's separate report (työnantajan erillisilmoitus) to the Incomes Register

## Why

A company in the Tax Administration's employer register must send an
employer's separate report (työnantajan erillisilmoitus) to the Incomes
Register for **every month**, also when it paid no wages. The report states
the month's employer health insurance contribution, or carries "no wages
paid" when nothing was paid. It is a different report from the earnings
payment report (palkkatietoilmoitus) that `file-wage` already files for
travel reimbursements, and filing one does not replace the other. Its
deadline is the 5th day of the following month; a missing report leads to
reminders and late-filing fees.

aggountant has the descriptor (`TulorekisteriReport::TyonantajanErillisilmoitus`
in `crates/domain/src/payroll.rs`) and a stubbed `erillisilmoitus` effect in
`run_payroll`, but no production channel: `crates/broker/src/tulorekisteri.rs`
only sends wage reports (`WageReportService.svc`, `SendWageReport`). An
owner-only company that pays only tax-free travel allowances, the common case
for aggountant users, therefore cannot file the report at all.

## What to build

Filing the monthly employer's separate report through the same own-organisation
Incomes Register channel `file-wage` uses, with the same safety model: a
preview of the exact content, an approval that binds those bytes, one
submission, the receipt as the fence against resubmission, and a read-only
status check. An uncertain outcome is resolved by asking the Incomes Register,
never by sending again.

- **No-wages month**: a report with "no wages paid" for the month.
- **Month with only tax-free reimbursements**: the official guidance has to
  decide whether that is "no wages paid" or a report with zero contributions.
  Find the answer in the Incomes Register's own documentation and implement
  that; if the documentation does not settle it, use "no wages paid" and
  record the choice.
- **Month with wages**: contributions computed from the month's filed
  earnings payment reports. If that is too large for this issue, refuse such
  a month clearly and leave it to a follow-up.
- **Which months are missing**: a read that lists the months since the
  employer registration (a company setting) with no accepted separate report,
  with each deadline, so an agent can catch up on late months.
- **Correction**: replacing a sent report follows the Incomes Register's
  replacement rules; a delete is out of scope unless trivial.

## Acceptance criteria

- [ ] Preview shows the exact report for a month; approval binds its hash; submit sends once and stores the receipt.
- [ ] A second submit for the same month with a stored receipt is refused; an unknown outcome is resolved through a read-only status check.
- [ ] "Missing months" lists unreported months with deadlines and is read-only.
- [ ] Tests against the mock/scripted Incomes Register cover no-wages, reimbursements-only, receipt, rejection and unknown outcome.
- [ ] The request XML validates against the official schema for the employer's separate report.
- [ ] The user guide (fi, en) has a task for the monthly report, linked from the travel-allowance task.
