---
created: 2026-11-12
updated: 2026-11-12
type: task
reporter: jari
status: open
priority: normal
related: ['@user-docs-design']
preparation: ready
lane: user-guide
lane_seq: 3
collision: [scripts/ci-local.sh]
---

# Gate: user guide language pairs, links and practice material

## Description

Add a check to `scripts/ci-local.sh` (and hosted CI) that keeps the user
guide in step with the product: every `docs/user/fi/` page has an `en/` page
at the same path and vice versa, every local link resolves, and the practice
material still imports and the tutorial's commands run in a throwaway home.
Design decision 7 in `issues/user-docs-design/design.md`.

## Acceptance Criteria

- [ ] A missing language pair, a dead link or a broken practice import fails the gate.
