---
created: 2026-11-24
updated: 2026-11-27
type: task
status: done
priority: normal
provenance: other
provenance_detail: taskfleet technical-decision brief
source_ref: taskfleet:01m87gzzt5thzm0qb8xrqabanq/decision:expense-claim-model
originating_run: 01m87gzzt5thzm0qb8xrqabanq
originating_run_kind: technical-decision
closed: 2026-11-27
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

The decision is ADR-11, `docs/site/adr-11-expense-claim.md`
(Status: Accepted 2026-11-26, decider Jari Mustonen), with audience companions and the
review record (`review.md`, `assessment.md`) beside this issue.

## Acceptance Criteria

- [x] Jari accepts, amends or rejects ADR-11 and answers its open questions.
- [x] If accepted, its implementation slices (listed in the ADR's last section) are triaged and laned.

## Comments

### 2026-11-26T02:54:02Z · @worker

ADR-11 rewritten on 2026-11-26 (status still Proposed): the product owner's answers to the rethink's questions 10-16 are folded in as decisions, the ADR's own open questions 1-9 stay open, and field lists, commands, the state table, refusal cases, basis codes and the ADR-5 change-log inheritance (review F7) moved into the implementation issues, which now own their details. Run `01m8aw1tra9jdfqq7gvdwa56w8`.

## Decisions

### 2026-11-26T03:58:54Z · @worker

ADR-11 accepted by the product owner (Jari Mustonen) on 2026-11-26; Status is now Accepted (2026-11-26). His answers of the same day to the ADR's open questions 1-9 are folded in as decisions (year-boundary travel day, meals, rounding and leisure in section 2; claim-payable account, retention and 'awaiting approval by X' in section 5; offset date in section 6; manual travel advances in new section 11) and the open-questions section is removed. The owning implementation issues carry the details. Run 01m8azxjdt7ce6e5r7vkgn6f2p.
