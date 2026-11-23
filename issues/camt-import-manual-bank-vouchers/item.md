---
created: 2026-11-22
updated: 2026-11-23
type: bug
status: fixed
priority: high
provenance: conductor
source_ref: stint-2026-11-22:camt-legacy-keys
lane: bank-import
collision: [crates/import, crates/cli/src/bank_import.rs, crates/ledger]
closed: 2026-11-23
---

# camt import does not recognise bank vouchers sealed through voucher seal

## Description

## Problem

`import camt053 seal` now recognises a bank transaction that an earlier
`import camt053 seal` booked (event key `bank-import:<bank>:<account>:<ref>`,
including vouchers backfilled from their retained statement XML). It does not
recognise a bank transaction that was booked through the generic
`voucher seal` path, for example with a natural key such as `bank:<ref>` and
the statement attached by hand. A company that has booked its bank leg that
way and later imports an overlapping camt statement gets a second voucher for
the same transaction: the import sees an unknown event key and seals it.

Synthetic reproduction: seal a voucher for a bank entry through
`voucher seal` with natural key `bank:REF1`, then run
`import camt053 seal` on a statement containing entry `REF1` with a matching
assignment. Two vouchers book the same money.

## Expected

Either the import detects a voucher that already books the same bank entry
(same account, amount and date, or a recorded bank reference) and stops before
writing with a visible conflict, or there is a supported way to bind existing
vouchers to their bank event keys before switching to camt import. The
bank-import guide states which.

## Resolution

### 2026-11-23T19:13:42Z · @issuectl

Import stops with BANK_ENTRY_POSSIBLY_BOOKED on a voucher sealed outside the import that may book a new entry; existing_vouchers binds an entry to such a voucher, not_booked_by rules one out. See result.md.
