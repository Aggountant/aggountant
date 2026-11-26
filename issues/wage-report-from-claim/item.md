---
created: 2026-11-10
updated: 2026-11-26
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

## Comments

### 2026-11-24T20:00:41Z · @worker

ADR-11 (`docs/site/adr-11-expense-claim.md`, Proposed, section 9) confirms this issue and sharpens it:
- The input is built only from sealed claim documents (slice `expense-claim-approve-seal`), not from line files; the payment date comes from the settlement link written by the bank import or `claim settle` (slice `expense-claim-payment-matching`), so `--payment-date` is no longer typed by hand.
- Meal allowance goes as 331 type 1 (not 303); lodging, the night travel allowance and receipt lines are listed as left out with the reason.
- One 331 carries every per diem type of the amount; that format change is its own slice `wage-report-multiple-allowance-codes`.
- Monthly combination: `from-claim` accepts several settlements of one claimant in one payment month (law 53/2018, 12 §). `source_reference` = the claim id for one claim, `expense-claims-<employee_id>-<YYYY-MM>` for several, `<claim_id>-p<n>` for a supplementary payment. A settlement belongs to exactly one report: the `file-wage` process state lists the settlements it reports and `from-claim` refuses one already reported.
- The "optional: record the payment" part moves to `expense-claim-payment-matching`.
Depends on `expense-claim-approve-seal` and `expense-claim-payment-matching`.

### 2026-11-26T02:54:02Z · @worker

ADR-11 was rewritten on 2026-11-26 to the product owner's decisions and now states principles only. The earlier comment's section number refers to the first text; the details it lists stand as this issue's own (the Incomes Register is now ADR-11 section 6). The `source_reference` patterns and the left-out lines are this issue's proposals, not ADR decisions.
