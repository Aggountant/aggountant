---
created: 2026-11-10
updated: 2026-11-10
type: improvement
status: open
priority: normal
lane: bank-import
lane_seq: 40
collision: [crates/import]
---

# camt import: expose transaction codes, reversals, charges and original amounts

## Problem

The camt parser (`crates/import/src/camt053.rs`) exposes only amount,
direction, dates, counterparty and remittance information. The skill that
assigns accounts and VAT would judge better with source data the parser
currently discards:

- `BkTxCd` (domain, family and sub-family codes, plus proprietary code): the
  ISO 20022 transaction type (salary, bulk transfer, card, fee…);
- `RvslInd` and `RtrInf`: reversal flag and return reason, which separate a
  bounced payment or correction from an ordinary entry and keep the link to the
  original;
- `Chrgs`: bank charges included in the entry amount, usually booked to a
  separate account;
- `AmtDtls/InstdAmt`: the original currency amount of a foreign-currency
  payment.

## Expected

Optional passthrough fields on `BankTransaction` for these elements, with a
decided shape for reversal semantics, covered by golden tests on synthetic
files.

## Acceptance criteria

- [ ] Each element above is parsed into a typed optional field when present.
- [ ] Golden tests cover a card entry, a reversal with return reason, an entry with charges and a foreign-currency entry.
- [ ] Files without these elements parse exactly as before.
