---
created: 2026-11-11
updated: 2026-11-11
type: feature
reporter: jari
status: open
priority: normal
preparation: ready
lane: onboarding
lane_seq: 2
collision: [crates/ledger, skills/aggountant-init]
---

# Validate the opening-balance cutover from a previous system

## Description

When books move mid-year from another system, opening balances include the
current year's result, and the old accounts map onto aggountant's chart,
often many to one. An account can be left out, mapped twice or have its sign
flipped without anything noticing.

Define one machine-checkable mapping in which every source account appears
exactly once, preserving the source breakdown and debit/credit direction.
After import, check the first month's result, the year-to-date result, the
balance sheet and the bank opening balance against the previous system's
control totals. Bulk multi-year history stays with `historical-import`.

## Acceptance Criteria

- [ ] A mapping missing or duplicating a source account is rejected with the account named.
- [ ] Post-import checks report each control total and any difference.
- [ ] Tests use synthetic charts and amounts.
