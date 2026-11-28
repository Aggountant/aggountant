---
created: 2026-11-24
updated: 2026-11-28
type: feature
status: open
priority: normal
provenance: other
provenance_detail: technical-decision slice of ADR-11
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/slice:expense-claim-legacy-migration
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
lane: expense-claim
lane_seq: 80
blocked_by: ['@expense-claim-approve-seal', '@expense-claim-payment-matching']
---

# Attach an already sealed voucher to an expense claim identity

## Description

## Goal

A voucher sealed for an earlier expense claim (for example a payable voucher
whose attachment is a monthly summary) can be attached, as it is, to a claim
identity, so the product's claim status, verification and corrections see it
without resealing anything. Follows ADR-11
(`docs/site/adr-11-expense-claim.md`, section 10; product owner 2026-11-26,
answer 15a: attaching a sealed voucher to a claim identity stays in the
product).

## Out of scope

- Importing one home's own claim files (line files, an open month) into
  drafts. Moving a home's own file format is done on that home's side, not
  by a product command (product owner 2026-11-26, answer 15b).
- Claim history in general when a company moves in: the phased `migrate`
  skill (`migrate-skill-phased`) archives old claims with their vouchers and
  carries forward what is still open.

## Scope

- `claim attach-sealed` (name indicative): records a link row of kind
  `legacy` (the claim id in the format the home used, version 1, document
  hash = the voucher's existing `attachment_ref`, the voucher number). No
  new document and no new voucher: a second structured document for the
  same booking would be a second voucher for one transaction.
- Optionally in the same command, the settlement link from an existing
  payment voucher and the existing `file-wage` report found by its
  `source_reference`, so the derived status (paid, reported) is right.
- `claim show` renders an attached version's sealed attachment as it is.
- The guide says a summary edited after approval is not the voucher.

## Acceptance criteria

- [ ] Synthetic home (Esimerkki Oy): two sealed claim vouchers attach with correct status (paid, reported).
- [ ] `voucher verify` and `claim verify` pass after attaching; nothing is sealed by attaching.
- [ ] Attaching the same voucher twice is idempotent; a hash that does not match the voucher refuses.
- [ ] User guide (fi/en) describes attaching.

## Likely files

`crates/process/src/expense_claim.rs`, `crates/ledger/src/expense_claim.rs`,
`crates/cli/src/claim.rs`, `docs/user/{fi,en}/`.

## Dependencies

Needs `expense-claim-approve-seal` (link tables) and
`expense-claim-payment-matching` (settlement link).

## Decision

ADR-11, `docs/site/adr-11-expense-claim.md` (Accepted 2026-11-26); decision issue `expense-claim-model`. The ADR states the principle; the details above are this issue's own. Dependencies above are prose; the conductor sets `blocked_by` after triage.

## Comments

### 2026-11-26T02:54:02Z · @worker

Narrowed on 2026-11-26 to the product owner's answer 15a: attaching an already sealed voucher to a claim identity stays in the product. The former `claim import --legacy` (reading one home's line files into a draft) is out of product scope by answer 15b: a home's own file format is moved on that home's side, and claim history in general goes through the phased `migrate` skill (`migrate-skill-phased`). The dependency on `expense-claim-draft` went with it.

### 2026-11-27T18:08:17Z · @spinoff-01m8f29vs6esbc93pqv250b1sj

When this lands, update `skills/migrate/kululaskut.md`: its "Odottaa ohjelmaa" section says attaching an old aggountant voucher to a claim identity is not in the program yet. Replace that with when the migrate skill uses this command.

### 2026-11-28T06:23:57Z · @worker

ADR-12 (`docs/site/adr-12-open-items.md`, Proposed 2026-11-28): attaching an earlier sealed claim voucher writes the generic rows: an open-item row (`kind = expense_claim`, `kind_key` = the claim id the home used), origin rows naming the existing voucher's payable postings, and, when the payment is already booked, settlement rows naming the existing payment voucher's posting. Nothing is sealed. Checks: the posting exists and is on the payable role account or an account the user names (older homes keep other payables on `2740`), and it is not already the origin of another live item.
