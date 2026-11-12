---
created: 2026-11-12
updated: 2026-11-12
type: improvement
reporter: jari
status: done
priority: normal
preparation: ready
lane: docs-readme
lane_seq: 1
collision: [README.md]
closed: 2026-11-12
closed_by: spinoff
---

# README: say plainly what aggountant does and how it is used

## Description

The README opens too technically. A newcomer should understand within the
first screen what aggountant does for them and how they use it: you keep a
Finnish company's books by talking to an AI agent; the agent prepares
vouchers from bank statements and receipts; you approve; aggountant seals the
books and files VAT and Tulorekisteri reports. Implementation vocabulary
(`--json`, strict validation, skills as operating manuals, sealed hash chains)
belongs further down or in the guide.

Rewrite the README so it is short and to the point: one plain sentence of
what it is, a concrete picture of a month with aggountant, honest status,
how to get started, and links to the user guide in Finnish and English.
Keep it English. Keep shipshape's marker regions (badges, install, licence)
valid.

## Acceptance Criteria

- [ ] The first screen says what aggountant does and how it is used, without implementation jargon.
- [ ] Every command and claim in the README is true of the current binary.
- [ ] `shipshape audit` shows no new README gap.
