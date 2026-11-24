# Review: migration-skill

- Skill: `/llm-review`, rigor **routine**: agent instructions, user-guide text
  and one entry in the bundling table. Nothing here writes the books or talks
  to an authority.
- Scope: the working-tree diff against `main`: the new
  `skills/migrate-books/{SKILL.md,briox.md,holvi.md}`,
  `crates/cli/src/skill.rs`, `crates/cli/tests/skill_cli.rs`,
  `skills/aggountant-init/SKILL.md`,
  `docs/user/{fi,en}/start/{move-from-another-program,install}.md` and
  `CHANGELOG.md`.
- Finder: `gpt-6-astra`, one call (the diff, the issue, `skills/AGENTS.md`,
  `docs/user/AGENTS.md`, `docs/user/AGENTS-STYLE.md`, `SOUL.md`, the
  `aggountant-init` skill, `skill.rs`, both guide pages). Verifier: not
  called; the finder returned no candidates and the surface is not high-risk.
  One model call in total.
- Risk class: documentation correctness (an agent following a wrong
  instruction in someone's old bookkeeping system).

## Confirmed findings

None from the finder ("No findings.").

## L1 (should-fix, found locally): the cutover invoice check named a different source than before

- Location: `docs/user/{fi,en}/start/move-from-another-program.md`, last
  paragraph of step 7; `skills/migrate-books/SKILL.md`, "Luovutus
  käyttöönottoon".
- Claim: the page used to say the agent checks an invoice dated before the
  cutover against the balance list. The first draft of this change replaced
  that with the list of open invoices. The balance list is where the trade
  payable balance is; the open-invoice list says which invoices make it up.
  Dropping the first changed a fact the issue did not ask to change.
- Evidence: the removed lines of the guide page in the diff; the
  `aggountant-init` skill ("Mitä saldoluettelo ei kerro") says the balance
  list does not itemise payables.
- Fix: both sources are named, in the guide (fi, en) and in the skill.

## Unresolved uncertainty

- The Briox and Holvi observations carry the year 2026 and no day, as they
  did in the guide. They were moved, not re-observed.

## Dropped candidates

None (the finder proposed none).

## Coverage

Read: the whole diff and every file the finder was given. Checked locally
while writing: each sentence of the three skill files against the removed
guide text, `SOUL.md` and the `aggountant-init` skill; three sentences that
stated more than the guide had (what every Holvi transaction has, which
purchase invoices a date filter drops in Briox, that list defaults are short
in programs generally) were removed or softened before the finder ran.
Tested: the skill unit tests (metadata, relative links, named commands) and
`crates/cli/tests/skill_cli.rs`; `scripts/check-user-guide.sh`. Outside
scope: whether the two services still behave as observed.
