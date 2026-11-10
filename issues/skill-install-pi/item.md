---
created: 2026-11-10
updated: 2026-11-10
type: feature
status: open
priority: normal
lane: cli-canon
lane_seq: 70
collision: [crates/cli/src/main.rs, crates/cli/src/skill.rs]
---

# skill install: also install skills for the pi agent harness

## Problem

`aggountant skill install` installs the bundled skills into `~/.claude/skills/`
only (`crates/cli/src/skill.rs`), so they are not discoverable by the pi agent
harness, which reads `~/.pi/agent/skills/`, `.pi/skills/` and `.agents/skills/`
and invokes skills as `/skill:name`. pi resolves bare `/name` cross-references
through its skill list, so only the install target differs; skill text needs no
rewrite. `install` already accepts several target roots internally.

## Expected

`aggountant skill install` can also install each skill under
`~/.pi/agent/skills/<name>/`, without changing the Claude Code path. Only
`SKILL.md` (and the files a skill actually references) is copied to the pi
target. Selecting targets uses an explicit `--agent` option rather than
guessing from what is installed.

## Acceptance criteria

- [ ] `skill install --agent pi` (and `--force`) creates each skill under `~/.pi/agent/skills/`; re-running is idempotent.
- [ ] The Claude Code install path is unchanged.
- [ ] Documented in `docs/user/` and the README.
