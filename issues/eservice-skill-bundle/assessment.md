# Assessment of the eservice-skill-bundle review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| R1 | vero-fi manual route vs omavero.md on filling a VAT return | CONFIRMED (as inconsistency; ADR contradiction refuted) | OCCASIONAL | IMPROVES | NONE | HIGH | FIX — applied |
| R2 | Stop point at the OK dialog instead of before submit | CONFIRMED | OCCASIONAL | IMPROVES | NONE | HIGH | FIX — applied |
| R3 | Seed dates are document dates, not witness dates | CONFIRMED | ALWAYS | NEUTRAL | LOW | MEDIUM | DISCUSS — decider, in `result.md` |

FIX: 2   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 1   DROP: 0

- R1 and R2 are the agent's boundary around irreversible filings, so they were
  fixed before merge rather than deferred.
- R3 is not filed as an issue: the dates are within weeks of the production
  observations, and the open question (whether seeds need their own state or
  real dates from the maintainer's notes) is the decider's and is raised in
  `result.md` and belongs with `eservice-knowledge-granularity` / slice 2.

## Simplicity

- Reuses: the existing install loop, `install_file` and its per-file
  `action` reporting; `aggountant_model::Date::parse` for the packaging test,
  so the test owns no date parsing and is unaffected by the chrono→jiff
  migration.
- Removed: the `aggountant-init` / `SOUL.md` special case and the per-name
  `match` of dependencies; one `skill_files!` list per skill now drives
  install, print (`SKILL.md` first) and `path_in_repo`, which is derived
  instead of written out.
- Left out: pruning files dropped from a bundle (the installer never deletes;
  noted in `skills/AGENTS.md`), subdirectories inside a skill, and any
  parsing of entries at run time; the entry format is checked only by a test.
