---
created: 2026-11-21
updated: 2026-11-21
type: task
status: open
priority: high
preparation: ready
lane: skills-rebuild
lane_seq: 0
collision: [skills/AGENTS.md, crates/cli/src/skill.rs]
---

# Skill writing principles in skills/AGENTS.md and a test that named commands exist

## Description

The shipped skills grew slice by slice, each worker adding what its feature
needed, until `vero-fi` alone was over half of all skill text, restating the
program's state machine, copying `--help` and carrying implementation notes,
while the month close, the work an agent does every month, was in no skill at
all. A review of the skills (2026-11-21) traced this to the absence of any
rule for what a slice puts into a skill and what it puts elsewhere.

## Wanted

Write the principles into `skills/AGENTS.md`, which every slice that touches a
skill reads, as reasons a writer can apply rather than a checklist:

- A skill carries what the agent cannot get from the program: reasons,
  consequences, traps, the authorities' practice. Commands, flags, field
  names, states and error codes belong to `--help` and the output.
- Wanting to write an input format or a list of states into a skill is a
  sign the program should show it; fix the program (an INPUT section, a
  `message`/`next_command`, an error hint).
- One fact, one home. House rules for the company home (`AGENTS.md`,
  `TODO.md`, `knowledge company`, `obligation`, secrets) live in
  `aggountant-init/SOUL.md`; task skills do not repeat them.
- SOUL holds only what applies to every task: the role, the house rules,
  what is permanent and why the user approves it, how to read the tool.
- Rarely needed material (setup, incidents, catching up history) goes into a
  reference file next to the skill, read when needed.
- A boundary is stated once, with its reason; emphasis and prohibitions in
  every paragraph hide the real limits.
- A skill describes only what works; a built but unconnected path belongs in
  the guide's `reference/status.md`.
- No history and no implementation notes. A dated observation of the
  authorities' behaviour is knowledge; how the code used to behave is not.
- A slice that changes a command checks the skills that name it in the same
  change.
- Size is a signal: when a task skill's body passes about a thousand words,
  split or move before adding.

Add the packaging-test check that every `aggountant <command> [<sub>]` path a
skill names exists in the binary's command tree (the review found a skill
naming a `journal` command that never existed), in `crates/cli/src/skill.rs`.
