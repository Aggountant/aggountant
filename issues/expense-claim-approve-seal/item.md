---
created: 2026-11-24
updated: 2026-11-26
type: feature
status: open
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:expense-claim-approve-seal
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
lane: expense-claim
lane_seq: 20
blocked_by: ['@expense-claim-draft']
---

# Approve an expense claim and seal its document and vouchers in one step

## Description

## Goal

One content-bound command approves a claim and seals it into the books: the
user sees the preview, says "hyväksyn", and the claim becomes vouchers plus a
frozen claim document that no later step can change. Follows ADR-11
(`docs/site/adr-11-expense-claim.md`, sections 5 and 8); this issue owns the
document, the approval record and the vouchers.

## Scope

- Claim document schema (canonical JSON through the wire encoder, own
  `record_type` and `schema_version`): content, computation with table hash,
  claimant facts and name (no HETU/IBAN), workplace snapshots, Incomes
  Register lines and left-out lines, proposed postings, recording times,
  approvals.
- `claim preview` (content hash) and `claim approve` (hash + one or more
  approvals: name, optional `employee_id`, role, time, optional evidence
  hash; `self_approved` marker and optional note; claimant's statement).
  A changed draft refuses `CLAIM_PREPARATION_CHANGED`.
- Every approval entry states its kind (product owner 2026-11-26):
  - *own approval*: approver's name, optional `employee_id`, role, time,
    optional evidence;
  - *recorded outside approval*: the outside approver, the time, the system
    and its reference, and separately the operator who recorded it. The
    program accepts it only when its own computation of the draft matches
    the content approved in the outside system (from
    `expense-claim-external-import`); a mismatch refuses with a code naming
    the differing items, and the user either fixes the source and imports
    again or approves the claim as an own approval. No separate approval
    workflow.
- A batch seal in `ledger`, one transaction: the document to artifacts, one
  voucher per receipt line (receipt date, the Domain VAT expansion and
  treatments `capture-receipt` uses, reverse charge included; card slip,
  no receipt, entertainment and foreign VAT carry no deduction), one voucher
  per calendar month in which a trip ends for that month's scheduled
  allowances (last day of month), each with the document hash as
  `attachment_ref` and a natural key naming claim, version and part; link
  tables outside the hash chain (version, vouchers, receipt hashes).
- Chart role `expense-claim-payable` in a new role file (masterdata by
  addition), `fi-general-2026` mapped to `2740`. The product owner decided
  on 2026-11-26 that claim payables get their own account in the next chart
  edition (proposed `2741` Kululaskuvelat, in the same edition as `1740`
  Matkaennakot for travel advances); the role maps to it there. The chart
  edition itself, and how an existing home moves to it, is a later slice,
  not part of this issue.
- The approver (product owner 2026-11-26, ADR-11 section 5): the company
  records who approves its claims, and "awaiting approval by X" exists in
  every company. In a company with several people the claimant submits the
  previewed content and the claim waits for that approver; the wait ends
  when the approver approves or rejects (back to the draft) or the claimant
  withdraws. In a one-person company the step is acknowledged at once and
  the claimant's approval is recorded visibly as self-approval, with no
  separate wait. Settle in this slice where the approver record lives and
  what a draft write after submission does (the binding to previewed
  content suggests it withdraws the submission).
- Retention (product owner 2026-11-26): nothing is deleted. The retention
  period is decided when deletion is built, likely about ten years for the
  data an Incomes Register correction may rely on; the document stays
  outside the hash chain so that this remains possible.
- Closed periods (product owner 2026-11-26): the allowance voucher of a
  trip in a month already closed is booked in the first open period of the
  same fiscal year, as a correction is, and the document shows the trip's
  own dates. A receipt line dated in a closed period is treated as
  `capture-receipt` treats the same receipt. A trip across the fiscal-year
  end is carried by the year close's accrual (siirtovelka), which lists
  trips with no sealed claim; the claim then books against it.
- Refusals: receipt already sealed by `capture-receipt` or another claim,
  sealed VAT period for a receipt, any open question, a recorded outside
  approval that does not match. Refused seals keep the ledger's rejection
  record.
- One-way migration of `fiscal.sqlite` for the link tables (allowed by the
  product owner 2026-11-26): back up the store first and compare the
  migrated copy with the original before the slice is accepted.
- `claim verify`: links against vouchers, documents exist and hash right.
- The draft is removed after the seal.

## Acceptance criteria

- [ ] A synthetic claim with two receipts in different months and trips in two months seals four vouchers atomically; a crash mid-way leaves nothing.
- [ ] Re-running approve with the same content is idempotent; a changed draft refuses.
- [ ] `claim verify` detects a missing or altered document.
- [ ] Self-approval is visible in preview, document and `claim show`.
- [ ] In a synthetic two-person company a submitted claim waits for the recorded approver and leaves the wait by approval, rejection or withdrawal; in a one-person company the step is acknowledged at once as self-approval.
- [ ] A recorded outside approval seals when the computation matches and refuses with the differing items when it does not; the document shows both the outside approver and the recorder.
- [ ] A synthetic trip in a closed month seals its allowance voucher in the first open period with the trip dates in the document.
- [ ] The document is byte-stable across machines (golden vector).
- [ ] User guide (fi/en): approving and what the vouchers are.

## Likely files

`crates/ledger/src/lib.rs`, a new `crates/ledger/src/expense_claim.rs`,
`crates/ledger/src/store/sqlite_fiscal.rs` (migration), `crates/wire/`
(document encoding, goldens), `crates/ledger/data/` (role file),
`crates/process/src/expense_claim.rs`, `crates/process/src/capture_receipt.rs`
(shared VAT preparation, not a copy), `crates/cli/src/claim.rs`, `docs/user/{fi,en}/`.

## Dependencies

Needs `expense-claim-draft`.

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Accepted 2026-11-26); decision issue `expense-claim-model`. The ADR states the principle; the details above are this issue's own. Dependencies above are prose; the conductor sets `blocked_by` after triage.
