---
created: 2026-11-11
updated: 2026-11-11
type: feature
reporter: jari
status: open
priority: normal
preparation: ready
lane: onboarding
lane_seq: 1
collision: [skills/aggountant-init]
---

# Onboarding readiness checklist and one tailored source request

## Description

`init` creates a technically valid home, but starting real bookkeeping needs
more: fiscal year, cutover date from the previous system, opening balances,
bank material, open items, company costs paid personally and durable
business-use facts. Without a checklist these surface one at a time and the
user is asked for material in many rounds.

Extend the init skill with an explicit readiness checklist (identity, fiscal
year, cutover, source coverage, mapping, reconciliations, first close) and
have it produce one request for the initial source package tailored to the
company. Reuse existing config, company-home guidance and report commands; no
workflow engine.

## Acceptance Criteria

- [ ] The init skill shows what is ready and what is missing.
- [ ] It produces one tailored source request; an example uses the fictional Esimerkki Oy.
- [ ] The user guide chapter on installation reflects it.
