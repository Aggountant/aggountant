---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: untriaged
priority: normal
provenance: other
provenance_detail: found while rebuilding the month skill against the built binary
source_ref: taskfleet:01m7yfttyc2h090xfx2xtre7aa/gap:camt-import-preview-and-balances
originating_run: 01m7yfttyc2h090xfx2xtre7aa
originating_run_kind: spinoff
---

# Bank statement import has no preview, and parse hides the statement balances

## Description

## What goes wrong for the user

A bank statement import seals a voucher for every line at once, and it is the
largest batch of permanent entries in a month. Unlike a receipt
(`capture-receipt record --dry-run` and its preparation hash), the import has
no preview: the agent cannot have the program check the assignments without
sealing, and nothing binds what the user approved to what is sealed.

## Observed (synthetic home, sample statement, binary built 2026-11-21)

- `import camt053 seal --help` has no `--dry-run`. A complete assignment list
  seals immediately (`created_count: 2`). The only refusal seen before sealing
  is a missing assignment (`MISSING_ASSIGNMENT`).
- `import camt053 parse --json` returns `account_iban`, `currency` and
  `transactions`. It does not return the statement's period or its opening and
  closing balances, so the figure the month is reconciled against has to be
  read from the XML by the agent.
- A March statement imported into a home whose work was on July without any
  notice; the guide's `reference/status.md` already notes that an empty
  statement imports as a success.

## Impact

The approval is whatever list the agent chose to show, and an assignment that
would be refused (unknown account, amounts that do not sum) is found only by
running the sealing command. The bank reconciliation, the one check that the
import read the file correctly, depends on a number the program has but does
not show.

## Direction

`import camt053 seal --dry-run` that returns the vouchers it would seal and a
hash to approve, as `capture-receipt record` does; and the statement's period
and balances in the `parse` output. `report statements` could then compare the
bank account with the imported closing balance.

When this lands, `skills/capture-receipt/SKILL.md` drops "Tuonnissa ei ole
esikatselua eikä hyväksynnän tiivistettä…" and "Loppusaldo luetaan tiliotteesta
itsestään, sillä `parse` ei näytä sitä."
