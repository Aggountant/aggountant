---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: done
priority: normal
preparation: ready
lane: skills-rebuild
lane_seq: 2
collision: [skills/aggountant-init/SKILL.md]
blocked_by: ['@skill-soul-rebuild']
closed: 2026-11-21
closed_by: claude-fable-5-2
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

## Resolution

### 2026-11-21T07:33:56Z · @claude-fable-5-2

SKILL.md 1 398 -> 935 words: house rules left to SOUL, help restatements dropped, opening voucher cases stated once each. Review and assessment beside the issue; two product gaps filed (nominally-undesirable-afterthought, distinctly-tired-hearing). See result.md.
