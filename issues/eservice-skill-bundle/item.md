---
created: 2026-11-21
updated: 2026-11-21
type: feature
status: open
priority: high
preparation: ready
lane: skills-contract
lane_seq: 1
collision: [crates/cli/src/skill.rs, skills/vero-fi/SKILL.md]
---

# ADR-10 slice 1: multi-file bundled skills and the e-services skill

## Description

ADR-10 (`docs/site/adr-10-eservice-knowledge.md`, accepted 2026-11-21),
slice 1: multi-file bundled skills and the seeded `e-services` skill.

- Generalise the bundled-skill installer (`crates/cli/src/skill.rs`) to a
  per-skill file list, replacing the `aggountant-init/SOUL.md` special case.
- Add `skills/e-services/`: a short `SKILL.md` and reference files for the
  first services (OmaVero, the Incomes Register e-service, the browser's
  requirements), seeded from the synthetic material in ADR-10, each entry in
  the ADR's entry format.
- Installing `vero-fi` also installs `e-services`. `vero-fi` links its portal
  steps to the references and replaces its stale "ADR-9" citation (that
  number now belongs to the date-time ADR) with ADR-10's restatement.
- Packaging test: every entry has the header lines with well-formed dates;
  it never fails on age.
- The user guide's list of bundled skills (fi, en).

The ADR's rules that the code cannot show move into `skills/e-services/SKILL.md`
and the nearest `AGENTS.md`. The ADR page is removed only when slice 2 has
also landed (`docs/AGENTS.md`).
