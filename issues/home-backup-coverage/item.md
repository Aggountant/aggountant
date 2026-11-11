---
created: 2026-11-11
updated: 2026-11-11
type: task
reporter: jari
status: open
priority: normal
preparation: ready
lane: home-backup
lane_seq: 1
collision: [docs/user]
---

# Back up and restore the company home's state outside git

## Description

The company home is a git repository, but some state lives outside git, for
example the broker outbox that records what was sent to an authority and
whether an answer arrived. Losing it means aggountant no longer knows whether a
filing already went out, which can lead to a duplicate submission.

Document which files a backup must contain and test that a restored home
keeps that state, while secrets (certificates, keys) and raw source data stay
under their own explicit policy. Runtime databases are not moved into git.

## Acceptance Criteria

- [ ] The user guide (fi and en) lists exactly what a backup must include and what it must not.
- [ ] A test restores a home with a pending outbox entry and the filing is still fenced.
