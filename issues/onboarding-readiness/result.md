# Result: onboarding-readiness

The `aggountant-init` skill now has a readiness checklist and one tailored
source request.

- **Valmiuslista** (seven items: identity, fiscal year, cutover, source
  coverage, account mapping and opening balance, reconciliations, first
  close). The agent builds it at the start of every session from what the
  home already holds: the home's `AGENTS.md` and `TODO.md`, and the read-only
  commands `config show`, `knowledge company list`, `fiscal-year list`,
  `voucher list` and `period show`. It is a view, not a new state file, so a
  home half-way through onboarding continues where it stopped: completed
  items are not restarted and recorded facts are not asked again.
- **Yksi aineistopyyntö.** The agent gathers everything the user still has to
  supply into one request tailored to the company (its own bank accounts and
  periods, only items that apply, why each is needed) and notes it in
  `TODO.md`. A home already part-way through onboarding gets a request for
  only what has not been delivered, merged into the waiting items of its
  `TODO.md`. The Esimerkki Oy example is the new reference file
  `aineistopyynto.md`.
- Items that apply only to some companies say so (account mapping and
  opening balance only when moving from another system; OmaVero statements
  only with a tax-account balance; payroll material only with employees). The
  "what the balance list does not tell" questions now also apply to a new
  company and ask about private property used for the business (durable
  business-use facts).
- To keep the skill body near the thousand-word limit of `skills/AGENTS.md`,
  the opening-voucher dating cases, account mapping and the post-opening
  check, which apply only to a company moving from another system, moved
  unchanged into the new `avaavat-saldot.md`, linked from the body. Both new
  files are listed in `crates/cli/src/skill.rs`; nothing was renamed or
  dropped, so existing installs have no stale copies.

User guide: `start/set-up-company.md` (fi, en) has a new step 3 for the
checklist and the request, a done-when line and the read commands;
`start/install.md` (fi, en) names the checklist in the skill table.

No code path, store or migration changed. Tests: `skill_cli` checks the new
file list and that installing `aggountant-init` writes all four files; the
existing link test checks the new relative links. `scripts/ci-local.sh`
passed, including the user-guide gate and site build.

Review: focused self-review (skill and guide text only, no bookkeeping,
filing or store code).
