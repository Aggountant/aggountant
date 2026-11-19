---
created: 2026-11-19
updated: 2026-11-19
type: feature
status: open
priority: normal
related: ['@eservice-knowledge']
preparation: pending
lane: skills-contract
lane_seq: 2
---

# Users' agents contribute friction reports and e-service observations as standard-format issues and pull requests

## Description

Users' agents meet friction in aggountant every day: a command that reads its
input in a different shape from its sibling, an output that hides what the
command exists to show, a missing setup command that forces a hand-edited
file. Today that knowledge reaches the product only when a user happens to
pass it on. Production use showed that agents already write good friction
reports when asked at the end of a session; they just have nowhere to send
them.

The maintainer's wish (2026-11-19): aggountant's bundled skills ask the
user's agent, at the end of a session, to turn the friction it observed into
a standard-format contribution, and where the agent works with GitHub access
(for example `gh` authenticated as the user), to open it automatically as a
pull request or issue, always through the user and with nothing company-
specific in it. The same channel carries e-service observations
(`eservice-knowledge`, ADR-10), which could arrive as standard-format pull
requests against the e-service reference files instead of issues.

Example (Esimerkki Oy, fictional): during the month close the agent finds
that `chart list` prints only the account count. At the end of the session it
shows the user a draft: title, the command and flags, what it expected, what
happened, a synthetic reproduction; the user approves, and the agent opens
the issue with `gh issue create` (or, for an e-service observation, a pull
request editing `skills/e-services/references/omavero.md`).

## To decide (design first, no implementation yet)

- Where the instruction lives: `SOUL.md` / a small `feedback` skill, read at
  session end; how it relates to the company home's `TODO.md`.
- The standard formats: friction issue form, e-service observation PR shape
  (entry header from ADR-10), and how a maintainer recognises them (labels,
  templates).
- Consent and privacy: the user sees and approves every submission; the agent
  rewrites to synthetic data and the skill says plainly what never goes out
  (names, business IDs, amounts, URLs with identifiers, screenshots, file
  paths). Without GitHub access the agent leaves a ready draft for the user.
- Duplicates: search existing issues first; add a "seen again" comment rather
  than a new issue.
- Maintainer side: triage into `issues/` (they arrive unlaned), and what the
  contributor hears back.
- Relation to ADR-10's accumulation path, which this would generalise; decide
  together or amend ADR-10.
