---
created: 2026-11-28
updated: 2026-11-28
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-12
source_ref: taskfleet:01m8gcdm7et7akjt1egj1dcmdm/slice:close-open-item-balance-check
originating_run: 01m8gcdm7et7akjt1egj1dcmdm
originating_run_kind: technical-decision
---

# Month close warns when a role account differs from its identified open items

## Description

## Goal

At the month close aggountant warns when the balance of a role account that
holds identified open items (claims payable now; later receivables and
payables) differs from the outstanding total of those items, so an unlinked
payment or an item without identity is found before the close. Follows
ADR-12 (`docs/site/adr-12-open-items.md`, sections 5 and 6).

## Scope

- `close-period` computes, per role account with identified items, the
  account balance and the outstanding total of its items, and warns with the
  difference and the postings on the account that no item or link names.
- A warning only, never a blocker.
- Only accounts with identified items are checked; on a shared account
  (claims on `2740` before the next chart edition) the warning says that
  other payables on the account explain part of the difference.

## Acceptance criteria

- [ ] A synthetic claim whose payment was booked as an ordinary posting on the
      claims payable account produces the warning naming that posting.
- [ ] A home whose items all reconcile shows no warning.
- [ ] Companies without the feature see no questions, output changes or
      slowdown: on a synthetic home without items, `close-period` and
      `close-year` previews and content hashes are byte-identical
      (regression test).
- [ ] User guide (fi/en) month-close page lists the check.

## Likely files

`crates/process/src/close_period.rs`, `crates/ledger/src/open_item.rs`,
`docs/user/{fi,en}/tasks/close-month.md`.

## Dependencies

Needs `expense-claim-payment-matching`; for invoice accounts,
`invoice-open-items`.

## Decision

ADR-12, `docs/site/adr-12-open-items.md` (Proposed); decision issue
`open-items-model`.
