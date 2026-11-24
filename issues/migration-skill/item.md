---
created: 2026-11-24
updated: 2026-11-24
type: feature
status: open
priority: normal
---

# Bundled skill for moving books from another program

## Description


## Description

`docs/user/fi/start/move-from-another-program.md` describes the whole
migration procedure: the inventory of what exists in the old program, the
read-only export, checksums of the retained archives, and the hand-off into a
new company home. No bundled skill carries this procedure, so the agent only
knows it if the user hands the page over. Every other recurring task the
agent performs has its own skill.

Decision (maintainer, 2026-11-24): the procedure becomes a bundled skill. The
user guide page then keeps what the user does and decides (what to collect,
signing in to the old service, what to check), and the agent's procedure
lives in the skill.

## Acceptance Criteria

- [ ] A new skill under `skills/` carries the migration procedure, written by
      `skills/AGENTS.md` (Finnish, what the program cannot tell the agent,
      relying on `SOUL.md` instead of repeating it), and `aggountant skill
      list` / `skill install` include it.
- [ ] The program-specific notes (Briox, Holvi) live where they serve the
      agent; the guide keeps what the user needs.
- [ ] `docs/user/{fi,en}/start/move-from-another-program.md` describes the
      user's part and points the agent to the skill; both languages in step.
- [ ] Skill and guide tests and `scripts/check-user-guide.sh` pass.
