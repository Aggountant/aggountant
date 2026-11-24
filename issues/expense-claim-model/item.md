---
created: 2026-11-24
updated: 2026-11-24
type: task
status: untriaged
priority: normal
provenance: other
provenance_detail: taskfleet technical-decision brief
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/decision:expense-claim-model
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
---

# Decide the expense claim (kululasku) model: storage, computation, approval and processing

## Description

## Description

Decide how aggountant models, stores, computes and processes an expense
claim (kululasku): travel and out-of-pocket reimbursements of an employee,
owner or board member of a Finnish osakeyhtiö, and how it connects to
vouchers, bank import, Tulorekisteri reporting and customer rebilling.

Today a claim is assembled by hand around `capture-receipt`, `voucher seal`
and `file-wage`; a production home that keeps claims as files has shown the
failure modes (the approved summary edited after sealing, unsealed line
files, calculation and states in agent instructions).

The proposed decision is ADR-11, `docs/site/adr-11-expense-claim.md`
(Status: Proposed, decider Jari Mustonen), with audience companions and the
review record (`review.md`, `assessment.md`) beside this issue.

## Acceptance Criteria

- [ ] Jari accepts, amends or rejects ADR-11 and answers its open questions.
- [ ] If accepted, its implementation slices (listed in the ADR's last section) are triaged and laned.
