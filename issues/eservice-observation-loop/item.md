---
created: 2026-11-21
updated: 2026-11-21
type: feature
status: open
priority: normal
related: ['@agent-feedback-contributions']
preparation: pending
lane: skills-contract
lane_seq: 2
blocked_by: ['@eservice-skill-bundle']
---

# ADR-10 slice 2: e-service observations from company knowledge to a public issue and back

## Description

ADR-10 (`docs/site/adr-10-eservice-knowledge.md`, accepted 2026-11-21),
slice 2, after `eservice-skill-bundle`: the accumulation loop.

- Recording and contribution guidance in `skills/e-services/SKILL.md`: a new
  or contradicting observation is first held as a company fact naming the
  entry id, then drafted as a synthetic public "E-service observation" for
  the user to submit; the local fact is retired once a binary ships the
  entry.
- A one-line pointer from `SOUL.md`.
- The GitHub issue form under `.github/ISSUE_TEMPLATE/`, a maintainer note
  in `CONTRIBUTING.md`, and a short user-guide section (fi, en) on what a
  contribution contains and never contains.
- Remove the ADR-10 page once its rules live in the skill and `AGENTS.md`.

Related: `agent-feedback-contributions` (the maintainer's wish that agents
open standard-format issues or pull requests for friction and observations,
through the user). Design it before or with this slice; this slice may need
to wait for that decision.
