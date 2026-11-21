---
created: 2026-11-21
updated: 2026-11-21
type: feature
status: open
priority: normal
preparation: ready
lane: skills-rebuild
lane_seq: 5
collision: [crates/cli/src/skill.rs]
blocked_by: ['@skill-soul-rebuild']
---

# Skill rebuild: a close-year skill for the year-end close

## Description

Skill rebuild (after `skill-soul-rebuild`). The year-end close (`close-year`
run: depreciation and accruals, statements from policy, check gates,
corrections, `SealYear`) is in no skill, although it is built and the guide
describes it.

## Wanted

A new, rarely loaded `close-year` skill, about 800 words, that gives the agent
what it cannot get from `--help` and the run's gates: what the year-end close
is for, what the user decides at each gate and why, what is permanent, the
traps the code and the guide know. Nothing about the month close (that is the
month skill). Registered in the installer and the guide's skill list.
