# Result: migration-skill

The migration procedure is a bundled skill, `migrate-books`.

- `skills/migrate-books/SKILL.md` (Finnish, about 950 words) carries what the
  agent does and why: the old system is only read, the inventory comes first
  and is the state of the work, what is collected and what later work needs
  it for, export before browser, checking every file from its content, the
  archive with `SHA256SUMS` and a verified second copy outside git, and what
  passes to `aggountant-init` (the cutover statement and the invoice already
  in the opening balances). It relies on `SOUL.md` for the role, secrets and
  the home's `AGENTS.md` and `TODO.md`, and names the point where each
  applies.
- `briox.md` and `holvi.md` are reference files of the skill, opened for the
  program the company uses. They hold the 2026 observations that were on the
  guide page.
- `aggountant skill list` shows seven skills; `skill install` writes
  `migrate-books` with its three files, and with `aggountant-init` when
  installed by name. The `aggountant-init` skill names `migrate-books` at the
  opening balances.
- `docs/user/{fi,en}/start/move-from-another-program.md` keeps the user's
  part: what to collect, signing in, reading the inventory, deciding where
  the archive is kept, going through what is missing, and the setup hand-off.
  The program notes are cut to what the user acts on (access rights, pricing
  plan, session expiry). `start/install.md` lists the skill in both
  languages.

Tests: the skill unit tests in `crates/cli/src/skill.rs`,
`crates/cli/tests/skill_cli.rs` (seven skills, the new skill's files),
`scripts/check-user-guide.sh` and `scripts/ci-local.sh` pass.

Review and assessment: `review.md`, `assessment.md` (finder clean; one
locally found text finding, fixed).
