---
created: 2026-11-19
updated: 2026-11-19
type: task
status: untriaged
priority: normal
provenance: other
provenance_detail: taskfleet technical-decision brief
source_ref: taskfleet:01m7t1vrfab4fw9pz3qf99zwf0/decision:eservice-knowledge
originating_run: 01m7t1vrfab4fw9pz3qf99zwf0
originating_run_kind: technical-decision
preparation: reviewing
lane: skills-contract
lane_seq: 1
---

# Decide how e-service operating knowledge accumulates and reaches agents

## Description

## Description

aggountant users run their books through their own AI agent, and many tasks
need that agent to operate the authorities' and banks' web services in the
user's browser: OmaVero (download letters, fill forms), the Incomes Register
e-service (order report 630), the bank's web app, pension insurers. This
operating knowledge is general, not company-specific, yet today each company's
agent rediscovers it and keeps it in its own private notes, so it is lost to
the product.

Decide where this knowledge lives in aggountant, how it accumulates from
users' agents into the product, and how it is offered to agents, under these
constraints: public and synthetic (no credentials, personal or company data,
session tokens or identifying URLs); every item carries provenance, the
service/page it applies to and a last-verified date so stale guidance is
recognisable; it never becomes authority to act (a filing goes through
aggountant's own approval and API paths); and general service operation,
company practice and machine configuration stay apart.

The proposed decision is ADR-10, `docs/site/adr-10-eservice-knowledge.md`
(Status: Proposed, decider Jari Mustonen).

## Acceptance Criteria

- [ ] Jari accepts, amends or rejects ADR-10.
- [ ] If accepted, its implementation slices are filed and laned.
