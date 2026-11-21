# Review: distinctly-tired-hearing

- Skill: `/llm-skill-review`, rigor **routine** (text-only change to the
  `aggountant-init` skill and the user guide; no code).
- Target: `skills/aggountant-init/SKILL.md` and the diff to
  `docs/user/{fi,en}/start/set-up-company.md`, `tasks/upgrade.md`,
  `reference/status.md`.
- Lenses: executor, edge-case. Model: `gpt-6-astra`, one call. The prompt
  stated the decision and the repository rule that skills must not grow.

## Findings

1. **No suitable destination account.** "aiemman järjestelmän tilit
   muunnetaan sille". The passage assumes every source account maps; an agent
   could force a classification. Proposed: wait for clarification when no
   reasoned target exists.
2. **Collapsed distinctions are left to "jos mahdollista".** The default
   chart has no named shareholder-debt or tax-account account, so "pidä eri
   tileillä, jos mahdollista" permits a silent merge whose later movements
   cannot be told apart. Proposed: show the merged distinctions and agree how
   they are kept apart.
3. **User insists on the old chart.** Handled by SOUL (explain limits, user
   approves before sealing). No change.
4. **Partial set-up with a stored "keep the old chart" record.** Handled by
   SOUL's rule to update or retire knowledge that turned out wrong. No change.
5. **Guide's opening date vs. the non-1.1. fiscal-year exception.** Claimed
   the guide contradicts the skill.
6. **Guide says there is no upgrade command.** `set-up-company.md`
   limitations deny the `home upgrade` command the skill and
   `tasks/upgrade.md` describe.
