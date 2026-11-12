---
created: 2026-11-12
updated: 2026-11-12
type: task
reporter: jari
status: in-progress
priority: normal
preparation: ready
lane: docs-design
lane_seq: 1
---

# Design the user documentation for new users

## Description

The user guide under `docs/user/{fi,en}/` was written chapter by chapter
from the CLI's command groups. It is accurate but reads like a reference.
Design what documentation a new user actually needs, before rewriting it:
who the readers are (a small-company owner doing their own books, a
bookkeeper, someone evaluating aggountant), what they are trying to do on
day one, in the first month and at year end, and what pages, page types and
structure serve that (for example a getting-started path, task guides,
explanations of concepts such as sealing and approval, reference, troubleshooting).
Look at how well-regarded documentation for comparable tools is organised
and say what to borrow.

Deliver a design, not the rewrite: proposed page list and navigation, a
template for each page type, two or three sample pages written in full
(Finnish), how Finnish and English versions stay in step, and what in the
current guide maps where. Whether docs stay as Markdown in the repository
or get a docs site is part of the proposal.

## Acceptance Criteria

- [ ] `design.md` in this issue with readers, page list, page-type templates, migration map from the current guide and a recommendation on format.
- [ ] Two or three sample pages in Finnish beside it.
- [ ] Decisions that need Jari listed as questions with a recommendation each.
