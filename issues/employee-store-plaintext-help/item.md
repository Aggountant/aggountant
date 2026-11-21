---
created: 2026-11-10
updated: 2026-11-21
type: bug
status: fixed
priority: normal
lane: payroll-pii
lane_seq: 1
collision: [crates/ledger/src/store/sqlite_pii.rs, crates/cli/src/main.rs]
related: ['@employee-pii-entry']
closed: 2026-11-21
closed_by: claude-opus-5-5
commits:
- hash: 5e0b48e
  summary: 'cli: fix INPUT help findings from review'
---

# employee help claims an encrypted store; the store is plaintext

## Problem

`aggountant employee --help` describes the employee store as a "PII-isolated,
encrypted store", and `employee add --help` says "PII lands only in the
encrypted store". The store is not encrypted:
`crates/ledger/src/store/sqlite_pii.rs` and `crates/ledger/src/employee.rs`
state that the cipher is a plaintext placeholder. `file-wage` needs a payee's
personal identity code and IBAN in this store.

`employee add --help` also calls the id field `id`, but the input requires
`employee_id` (`unknown field id` otherwise).

## Impact

The help tells a user their personal data is encrypted at rest when it is
plaintext SQLite protected only by `.gitignore`. A user may copy the home or its
backups somewhere they would not put plaintext identity codes. The user guide
already says the store is not yet encrypted.

## Expected

The help states plainly that the store is isolated and gitignored but not
encrypted, until real at-rest encryption lands. The help names `employee_id`.

## Acceptance criteria

- [ ] `employee` help text matches the store's actual protection.
- [ ] `employee add --help` names `employee_id`.

## Resolution

### 2026-11-21T07:19:19Z · @claude-opus-5-5

employee help now states the separate gitignored, unencrypted PII store and names employee_id (cli-stdin-input-help review F2).
