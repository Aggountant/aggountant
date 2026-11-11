---
created: 2026-11-11
updated: 2026-11-11
type: feature
reporter: jari
status: open
priority: normal
preparation: ready
lane: capture
lane_seq: 1
collision: [crates/process, skills/capture-receipt]
---

# Approve a month's prepared vouchers in one hash-bound batch

## Description

Every receipt is approved on its own. A month can hold dozens, and approving
them one by one is heavy.

Produce a monthly manifest of the prepared items (source facts, proposals,
control totals and each preparation's hash) and bind one explicit approval to
that manifest. If any item changed after the manifest was made, nothing is
sealed until the user looks again; define the partial-result behaviour
deterministically.

## Acceptance Criteria

- [ ] One approval seals all items of an unchanged manifest.
- [ ] A changed item after the manifest stops the whole batch with the item named.
- [ ] Each sealed voucher keeps its own preparation hash.
