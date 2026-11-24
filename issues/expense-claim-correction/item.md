---
created: 2026-11-24
updated: 2026-11-24
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:expense-claim-correction
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
---

# Correct or cancel an approved expense claim with Incomes Register follow-up

## Description

## Goal

An approved claim can be corrected or cancelled without editing anything
sealed, and the Incomes Register side follows the Tax Administration's
correction rules. Decided in ADR-11 (`docs/site/adr-11-expense-claim.md`,
sections 7 and 9).

## Scope

- `claim correct` opens a draft copied from version n; approval seals version
  n+1 (supersedes n), correction vouchers for the difference in the first open
  period, and links. `claim cancel` is a correction to empty content.
- Overlap and receipt checks exclude the claim's own predecessor.
- Incomes Register (VH/7200/05.00.00/2025): a reporting error with the right
  amount paid → replacement report on the same `source_reference`, prepared
  for its own approval; an underpaid amount → a new report for the date the
  difference is paid with reference `<claim_id>-p<n>`; an amount actually
  overpaid → the books get the correction and a receivable from the
  claimant, and the claim shows "Incomes Register correction needed outside
  aggountant" until `file-wage correction complete-manual` records the
  e-service evidence (the excess is taxable pay / unjust enrichment,
  guidance example 23, which `file-wage` cannot report).
- A correction touching a filed VAT period carries `corrects_vat_period`
  under the existing rule.

## Acceptance criteria

- [ ] Synthetic corrections: kilometres reduced before payment, increased after payment and report, reduced after payment and report (manual register state), cancellation before payment.
- [ ] No sealed voucher or filed report is modified.
- [ ] Every new state has its closing command and appears in `claim list --open`.

## Likely files

`crates/process/src/expense_claim.rs`, `crates/ledger/src/expense_claim.rs`,
`crates/process/src/file_wage_correction.rs`, `crates/cli/src/claim.rs`,
`crates/cli/src/wage.rs`, `docs/user/{fi,en}/`.

## Dependencies

Needs `expense-claim-approve-seal`, `expense-claim-payment-matching` and
`wage-report-from-claim`.

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Proposed 2026-11-24); decision issue `expense-claim-model`. Dependencies above are prose; the conductor sets `blocked_by` after triage.
