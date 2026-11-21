---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: open
priority: normal
preparation: ready
lane: skills-rebuild
lane_seq: 4
collision: [skills/vero-fi/SKILL.md, crates/cli/src/skill.rs]
blocked_by: ['@skill-soul-rebuild', '@cli-stdin-input-help', '@vero-configure-merge', '@cli-error-hints', '@filing-output-markers']
---

# Skill rebuild: split vero-fi into VAT and a new tulorekisteri skill with reference files

## Description

Skill rebuild (after `skill-soul-rebuild`, `cli-stdin-input-help`,
`vero-configure-merge`, `cli-error-hints`, `filing-output-markers`).
`vero-fi` is 54 % of all skill text. It retells the filing state machine in
prose (against `crates/AGENTS.md`: a skill keeps no copy of a state machine),
repeats `--help` for the employer report commands, carries implementation
notes (TLS, SOAP hash, old error codes), and its name does not cover the
Incomes Register, a different channel, permission and e-service.

## Wanted

Split into two skills:

- `vero-fi` (VAT): purpose, irreversibility, the ordinary month from preview
  to receipt, and the few principles that hold in an incident (never resend
  on a guess; ask `status`; follow `next_command`; `Missing` proves nothing),
  about 600 words, with reference files for set-up (profile, attestations,
  verification, certificate renewal) and incidents.
- `tulorekisteri` (new): travel-expense earnings reports and the monthly
  employer's separate report, about 500 words, with a reference file for
  catching up history. Its description routes on "Tulorekisteri".

Keep the knowledge the review names as valuable: `configure` behaviour (if
still relevant after `vero-configure-merge`), the 12-hour processing delay as
a dated observation, `record_hash` sensitivity (if not already in the output),
`Missing` proving nothing, report 630 ordering parameters, the test seam, the
reconciliation acknowledgement not being a way past missing tax. Both link to
`e-services`. Installs: `vero-fi` keeps working for existing users.
