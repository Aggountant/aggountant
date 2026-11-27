# Review: travel-report-statutory-deadline

- Skill: `/llm-review`, rigor **routine**: the change alters a deadline the
  program shows; it sends, seals and stores nothing differently.
- Scope: commit `4a98975` against `9c9051b` (`crates/domain/src/wage_report.rs`,
  `crates/domain/src/lib.rs`, `crates/cli/src/input_help.rs`, three test
  assertions, the guide pages fi/en, `skills/tulorekisteri/SKILL.md`,
  `CHANGELOG.md`).
- Finder: `gpt-6-astra`, one call (the diff plus the full `wage_report.rs` and
  `bank_days.rs`). Verifier: none; no candidate survived to verify and the
  surface is not high-risk. One model call in total.
- Risk class: displayed filing deadline (no authority side effect).

## Findings

No findings. The finder inspected the diff, both domain files, the preview
caller and `palkkatieto_deadline`, and confirmed from Verohallinto's guidance
both the monthly aggregation with the latest payment date and the
next-business-day adjustment.

## Dropped candidates

None proposed.

## Local evidence checked

- Primary sources read: laki tulotietojärjestelmästä 53/2018 12 § (11 mom.
  gives the monthly deadline for tax-exempt reimbursements; the law itself has
  no weekend rule), laki säädettyjen määräaikain laskemisesta 150/1930 5 §,
  Verohallinto's detailed guidance "Wages: reporting data to the Incomes
  Register" §2.1 (cites 150/1930 5 § for the shift), and "Reporting
  reimbursements of expenses to the Incomes Register".
- The weekday days that 150/1930 5 § shifts are exactly the bank holidays in
  `bank_days::is_finnish_bank_holiday`; checked list against list.
- Unit tests cover month end, February in a leap year, December → January,
  Saturday, Sunday, Easter Monday, Good Friday followed by Easter, Ascension
  Day and Epiphany after a weekend; the computus years were checked
  independently. `scripts/ci-local.sh` passes.

Residual risk: the employer's separate report (`ReportMonth::deadline`) still
shows the unshifted 5th; out of scope here and proposed as a follow-up.
