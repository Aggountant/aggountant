---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: open
priority: normal
preparation: ready
lane: skills-rebuild
lane_seq: 2
collision: [skills/aggountant-init/SKILL.md]
blocked_by: ['@skill-soul-rebuild']
---

# Skill rebuild: streamline aggountant-init

## Description

Skill rebuild (after `skill-soul-rebuild`). `aggountant-init/SKILL.md` is mostly
knowledge an agent cannot infer, with reasons, and that stays: the opening
voucher's date and the three cases, the open items at cut-over. Weight to
remove: the `home upgrade` and `fiscal-year define` paragraphs repeat their
`--help`; `home upgrade` is not set-up at all; the `TODO.md` and `AGENTS.md`
rules (about 500 words) are house rules that now live in SOUL; the opening
voucher text is tangled.

## Wanted

About 800 words: set-up knowledge as it is, the opening voucher cases made
clear, `TODO.md`/`AGENTS.md` created here with the rules left to SOUL,
`home upgrade` reduced to when to run it (or moved to SOUL's tool part).
Only this file.
