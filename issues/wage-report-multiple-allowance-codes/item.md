---
created: 2026-11-24
updated: 2026-11-26
type: improvement
status: open
priority: high
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:wage-report-multiple-allowance-codes
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
lane: wage-reporting
lane_seq: 17
---

# file-wage: one 331 daily allowance carrying several per diem types

## Description

## Goal

A month with full and partial per diems is reported as one daily-allowance
amount (331) carrying every per diem type it contains, as the Incomes
Register's guidance describes, instead of two 331 transactions. Follows
ADR-11 (`docs/site/adr-11-expense-claim.md`, section 6); this issue owns the
format change.

## Problem

`TravelLine::DailyAllowance` (`crates/domain/src/wage_report.rs`) holds one
`allowance_code`, and `crates/broker/src/tulorekisteri_xml.rs` writes one
`AllowanceCode` per transaction. The Incomes Register's earnings payment
report guidance says the per diem types included in one daily-allowance
amount are reported together and that several values may be given at once;
the vendored schema `crates/broker/schemas/tulorekisteri/2025/WageReportsToIRTypes.xsd`
declares `AllowanceCode` with `maxOccurs="unbounded"`.

## Expected

- `DailyAllowance` carries a non-empty, duplicate-free set of codes 1–4; the
  record writes one 331 transaction with each code.
- Existing input and process state with a single `allowance_code` are still
  read (as a set of one), so open and finished runs keep working.
- The report identity and approval binding are unchanged in meaning.

## Acceptance criteria

- [ ] XML for a 331 with codes 2 and 3 validates against the vendored XSD.
- [ ] Old `wage.json` states and old stdin input deserialize.
- [ ] Guide (fi/en) and `skills/tulorekisteri/SKILL.md` updated.

## Likely files

`crates/domain/src/wage_report.rs`, `crates/broker/src/tulorekisteri_xml.rs`,
`crates/process/src/file_wage.rs`, `crates/cli/src/wage.rs`,
`skills/tulorekisteri/SKILL.md`, `docs/user/{fi,en}/tasks/report-travel-allowances.md`.

## Dependencies

None; `wage-report-from-claim` uses it.

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Accepted 2026-11-26); decision issue `expense-claim-model`. The ADR states the principle; the details above are this issue's own. Dependencies above are prose; the conductor sets `blocked_by` after triage.
