---
created: 2026-11-10
updated: 2026-11-10
type: feature
status: open
priority: high
lane: vat-filing
lane_seq: 12
collision: [crates/process/src/file_vat.rs]
---

# Typed Tax Administration payment references and payee account

## Problem

A Finnish company pays the Tax Administration with several different
references: the permanent reference for self-assessed taxes (VAT, employer
contributions), a separate reference for a payment arrangement, and another for
prepayment tax. Using the wrong one allocates the payment to the wrong debt.
aggountant has no place for these references except free-text company
knowledge, so an agent has to find the right one from old bank statements.

`file-vat start` also takes `payee_iban` by hand on every run, and the payment
proposal carries no reference at all.

## Expected

Typed company master data for the authority payment references (self-assessed
taxes, payment arrangement, prepayment tax) and the Tax Administration's payee
account. `file-vat` payment proposals, and the preview's payment instruction,
include the payee IBAN and the self-assessed-tax reference automatically, and
never the arrangement reference. These are non-secret company data and may live
in git.

## Acceptance criteria

- [ ] Master-data schema and CLI to set and show the references.
- [ ] The VAT payment proposal includes the correct reference and IBAN; `payee_iban` input becomes optional when master data has it.
- [ ] A clear error when the self-assessed-tax reference is missing while a payment proposal is built.
