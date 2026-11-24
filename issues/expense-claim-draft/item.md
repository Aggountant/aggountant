---
created: 2026-11-24
updated: 2026-11-24
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:expense-claim-draft
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
---

# Expense claim draft: collect trips and receipt lines through the CLI with open questions

## Description

## Goal

The agent builds an expense claim over days or weeks through the CLI, and
after each write the program answers with what is still missing. The user
can see the claim as Markdown at any time. Decided in ADR-11
(`docs/site/adr-11-expense-claim.md`, sections 1, 2, 4 and 5).

## Scope

- Program-owned draft at `intake/expense-claims/<claim_id>/draft.json`,
  atomic writes, a program-created `.gitignore` in `intake/expense-claims/`.
- Commands (names indicative): `claim trip add|revise|remove`,
  `claim line add|revise|remove` (receipt lines; scheduled allowances are
  computed, the draft may only record a lower paid amount), `claim show
  [--format markdown|json]`, `claim discard --reason`.
- Every write validates typed fields and returns open questions: missing
  statutory fields (EPA 17 §), start-place role, vehicle ownership, free
  meals on a per-diem trip, meal-allowance answers, workplace entry, travel
  day boundary locations abroad.
- `recorded_at` and `source` per trip and line (user statement, imported
  record, confirmed reconstruction); "recorded afterwards" marker after
  60 days; evidence artifact hashes on trips and drives.
- Plausibility checks: kilometres vs route and per diem distance, receipt
  outside every trip, possible duplicate
  receipt, overlapping trips across open drafts and claims in force (a
  correction excluded from its own predecessor once corrections exist).
- Receipts go to `artifacts/` at capture; the draft holds the hash.
- Claim ids are opaque; a generated default from the claimant and first
  content date.

## Acceptance criteria

- [ ] A synthetic monthly claim and a synthetic single-trip claim can be built, shown and discarded.
- [ ] No draft content reaches git, also in a home whose root `.gitignore` predates the directory.
- [ ] Hand-edited draft JSON that fails validation is refused on read, not silently accepted.
- [ ] Open questions are machine-readable codes the skill can branch on.
- [ ] User guide (fi/en) and the claims skill describe collecting a claim; the skill keeps no calculation.

## Likely files

New `crates/process/src/expense_claim.rs` (or a module dir), `crates/cli/src/claim.rs`,
`crates/cli/src/main.rs`, `crates/domain/src/travel/`, `skills/` (a claims skill or an
extension of `capture-receipt`), `docs/user/{fi,en}/`.

## Dependencies

Needs `travel-allowance-rates-and-calculator` and `claimant-workplace-register`.

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Proposed 2026-11-24); decision issue `expense-claim-model`. Dependencies above are prose; the conductor sets `blocked_by` after triage.
