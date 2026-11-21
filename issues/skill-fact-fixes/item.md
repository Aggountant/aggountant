---
created: 2026-11-21
updated: 2026-11-21
type: bug
status: open
priority: high
preparation: ready
lane: skills-contract
lane_seq: 0
collision: [skills/vero-fi/SKILL.md, skills/capture-receipt/SKILL.md, skills/aggountant-init/SOUL.md, crates/cli/src/skill.rs]
---

# Shipped skills: four wrong facts (journal show, consult route, SOUL unreachable through skill print, codes missing from the guide)

## Description

A fact check of the shipped skills against the binary (2026-11-21) found
claims that are wrong today. Each sends an agent the wrong way.

1. `skills/vero-fi/SKILL.md` says a rejected Incomes Register report is
   recorded with `TULOREKISTERI_REJECTED` and shown by `journal show`. There
   is no `journal` command; `run show --run-id` shows the rejection (and the
   same skill elsewhere tells the agent not to read journal files).
2. `skills/capture-receipt/SKILL.md` and `skills/aggountant-init/SOUL.md`
   present the consult route (`capture-receipt start/answer/resume`, the
   Advice documents it produces) as the way an exceptional receipt goes.
   The guide's `reference/status.md` says consultation is not in use and
   nothing is connected behind the route. The skills should describe only
   what works: what the agent does with an exceptional receipt today (ask the
   user, record the open question in the home's `TODO.md`, or an obligation
   where the code creates one), and leave the consult route to `status.md`.
3. Every skill says "read SOUL.md first", but `aggountant skill print <name>`
   prints only that skill's `SKILL.md`, and SOUL cannot be printed by any
   name (`SKILL_NOT_FOUND`). An agent that reads skills through `skill print`
   (rather than an installed directory) can never follow the instruction.
   Fix in the binary: `skill print` can print a skill's other files and the
   files of the skills it requires (for example `skill print aggountant-init
   --file SOUL.md`, or a `--with-requires` that prints SOUL first), and
   `skill list --json` lists each skill's files. The skill text names the
   command that works.
4. Seven error codes the skills name are missing from the guide's error-code
   reference (`docs/user/{fi,en}/reference/error-codes.md`):
   `EMPLOYER_REPORT_INVALID_EVIDENCE`, `EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED`,
   `NATURAL_KEY_ALREADY_SEALED`, `TULOREKISTERI_REJECTED`,
   `VAT_POSTING_UNTAGGED`, `VERO_TEST_NOT_READY`,
   `VERO_VAT_PERIOD_NOT_FILEABLE`. Add them (warnings marked as warnings).

The stale "ADR-9" citation in `vero-fi` from the same check was already fixed
by `eservice-skill-bundle`.

## Out of scope

The larger restructuring the same review recommends (SOUL as the one home for
house rules, a month-close skeleton, splitting `vero-fi`, stdin formats in
`--help`) is separate work; do not start it here.
