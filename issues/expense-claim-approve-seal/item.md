---
created: 2026-11-24
updated: 2026-11-24
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:expense-claim-approve-seal
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
---

# Approve an expense claim and seal its document and vouchers in one step

## Description

## Goal

One content-bound command approves a claim and seals it into the books: the
user sees the preview, says "hyväksyn", and the claim becomes vouchers plus a
frozen claim document that no later step can change. Decided in ADR-11
(`docs/site/adr-11-expense-claim.md`, sections 2, 6 and 8).

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
- A batch seal in `ledger`, one transaction: the document to artifacts, one
  voucher per receipt line (receipt date, the Domain VAT expansion and
  treatments `capture-receipt` uses, reverse charge included; card slip,
  no receipt, entertainment and foreign VAT carry no deduction), one voucher
  per calendar month in which a trip ends for that month's scheduled
  allowances (last day of month), each with the document hash as
  `attachment_ref` and a natural key naming claim, version and part; link
  tables outside the hash chain (version, vouchers, receipt hashes).
- Chart role `expense-claim-payable` in a new role file (masterdata by
  addition), `fi-general-2026` mapped to `2740`.
- Refusals: receipt already sealed by `capture-receipt` or another claim,
  closed period, sealed VAT period for a receipt, any open question.
  Refused seals keep the ledger's rejection record.
- `claim verify`: links against vouchers, documents exist and hash right.
- The draft is removed after the seal.

## Acceptance criteria

- [ ] A synthetic claim with two receipts in different months and trips in two months seals four vouchers atomically; a crash mid-way leaves nothing.
- [ ] Re-running approve with the same content is idempotent; a changed draft refuses.
- [ ] `claim verify` detects a missing or altered document.
- [ ] Self-approval is visible in preview, document and `claim show`.
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

ADR-11, `docs/site/adr-11-expense-claim.md` (Proposed 2026-11-24); decision issue `expense-claim-model`. Dependencies above are prose; the conductor sets `blocked_by` after triage.
