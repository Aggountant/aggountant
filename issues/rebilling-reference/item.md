---
created: 2026-11-11
updated: 2026-11-24
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

## Comments

### 2026-11-24T20:00:41Z · @worker

ADR-11 (`docs/site/adr-11-expense-claim.md`, Proposed, section 10) extends this issue: a trip and a receipt line of an expense claim may carry the opaque rebilling reference, and the trip's reference applies to its scheduled allowances (mileage, per diem), which the product owner decided are rebillable too (2026-11-24). The program computes the cost base per reference (a receipt net of VAT deducted, gross when nothing was deducted; the allowance amount paid); invoicing, price and sales VAT stay in the billing system. The reference reaches the sealed postings as an opaque code (an ADR-4 dimension is the natural carrier), and the claim's monthly allowance voucher splits its postings by reference. Purchase VAT is unaffected, as this issue already says. The claim slices (`expense-claim-draft`, `expense-claim-approve-seal`) consume whatever field this issue defines.
