---
created: 2026-11-11
updated: 2026-11-11
type: feature
reporter: jari
status: open
priority: normal
preparation: ready
lane: capture
lane_seq: 4
collision: [crates/model, skills/capture-receipt]
---

# Optional customer rebilling reference on expenses

## Description

An expense that will be invoiced on to a customer (for example travel for a
customer project) has two independent lifecycles: purchase VAT and
reimbursement on one side, sales VAT, billing and payment on the other. There
is no way to mark an expense as rebillable.

Allow an optional opaque customer / project / rebilling reference on an
expense voucher. Purchase VAT stays independent of it; billing stays in the
billing system.

## Acceptance Criteria

- [ ] An expense can carry an optional rebilling reference that appears in reports.
- [ ] Purchase VAT is unaffected by the reference.
