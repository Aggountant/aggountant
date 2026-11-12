---
created: 2026-11-12
updated: 2026-11-12
type: task
reporter: jari
status: in-progress
priority: normal
related: ['@user-docs-design']
preparation: ready
lane: user-guide
lane_seq: 1
collision: [docs/user]
---

# Rewrite the Finnish user guide to the new structure (first draft)

## Description

Rewrite the Finnish user guide as a first draft following
`issues/user-docs-design/design.md`: the page tree under `docs/user/fi/`
(start, tasks, concepts, troubleshooting, reference), the five page-type
templates, the front page ordered by the reader's calendar, one home page per
fact, and the practice material under `docs/user/practice/` for the
practice-month tutorial. Use the three sample pages in the design issue as the
starting point for their pages and the migration map to carry every fact of
the current chapters to its new home; then remove the old chapter files.

The seven design decisions are taken as recommended for this draft (Jari
12.11.2026: go with the recommendations; he returns to the list later, so
keep it open in the design issue). In particular: identical English file
paths in both languages, commands folded in `<details>` under the steps,
Markdown kept mdBook-compatible, a short page for bookkeepers.

Change `docs/user/AGENTS.md` to the new rules (paths, templates, one home per
fact, fi/en parity). The English side is the next issue; until it lands,
`docs/user/en/` stays as it is.

## Acceptance Criteria

- [ ] `docs/user/fi/` follows the design's tree; old chapter files removed with every fact carried (migration checked in `result.md`).
- [ ] Practice material is synthetic and the tutorial's commands run against the current binary in a throwaway home.
- [ ] `docs/user/AGENTS.md` describes the new structure.
