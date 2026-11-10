---
created: 2026-11-10
updated: 2026-11-10
type: feature
status: open
priority: high
lane: wage-reporting
lane_seq: 20
collision: [crates/cli/src/wage.rs, crates/cli/src/main.rs]
---

# Build a Tulorekisteri travel report from an approved expense claim

## Problem

A Tulorekisteri travel-reimbursement report for an approved monthly expense
claim has to be assembled by hand: mileage kilometres and amount (income type
311), full and partial per diem counts and amounts (331), pay period equal to
the claim month, payment date from the bank. Receipt-based items
(accommodation, parking) must be left out by judgement. All of these facts
already exist in the expense-claim items and the sealed payable voucher.

## Expected

A command (for example `file-wage from-claim --claimant <id> --month YYYY-MM
--payment-date …`) that builds the `file-wage` report JSON from the claim:
311 lines with total kilometres, 331 lines with allowance codes, receipt
reimbursements excluded and listed as excluded, pay period defaulting to the
claim month. The output is the normal preview input; the user still approves
the preview. Whether the payee is a part-owner comes from explicit master data,
never inferred.

Optional: record the claim payment (debit payable, credit bank) in the same
flow, or check it against the bank statement, so the payment is not imported
twice later.

## Acceptance criteria

- [ ] Claim → report JSON is deterministic and covered by tests (mileage, full and partial per diem, receipt items excluded).
- [ ] No personal data in the generated JSON (`employee_id` only).
- [ ] Preview and approval flow unchanged.
