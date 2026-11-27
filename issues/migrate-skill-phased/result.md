# Result: migrate-skill-phased

`skills/migrate/` replaces `skills/migrate-books/`. The skill moves a company
from another program one domain at a time, each on its own cutover date.

## What was built

- `skills/migrate/SKILL.md`: what every domain shares. That covers the domain
  table with dependencies and the reasons for them: VAT is computed from
  aggountant's sealed vouchers, the Incomes Register and claims read the
  employee register, and claim payments and unreported reimbursements go
  through the books and the Incomes Register. It also says that a domain the
  company does not have is skipped without its questions, and that each moved
  domain gets a `knowledge company` fact (old program and last day, first
  aggountant day, what moved, where the archive is) with pending items in
  `TODO.md`. The read-only rule, inventory, export and check, and the archive
  rules complete it. Body about 880 words.
- One reference file per domain, opened only when that domain starts:
  `kirjanpito.md` (the old migrate-books collection list and hand-off),
  `alv.md` (aggountant can only file a period that lies wholly in its books,
  who files the old system's last period, the first period and the opening VAT
  balances), `tulorekisteri.md` (only the travel-allowance and employer reports move, never cash wages; the
  recipients, filed reports and their identifiers, transcript 630 through
  `tulorekisteri/kiinnikurominen.md`, and payments made but not yet reported),
  and `kululaskut.md` (ADR-11 section 10: old claims stay archived, what
  carries over, the regular workplace through `claimant add`, and earlier
  customer-site work). The claim commands that have not landed yet (draft,
  legacy attachment, three-year clock) are a marked "Odottaa ohjelmaa"
  section that tells the agent to prefer the commands once `--help` shows
  them.
- `crates/cli/src/skill.rs`: `RENAMED` (`migrate-books` → `migrate`).
  `skill install` / `skill print migrate-books` fail with `SKILL_RENAMED` and
  `details.renamed_to`. Installing `migrate` replaces an earlier install's
  `migrate-books/SKILL.md` with a pointer to `../migrate/SKILL.md` (reported
  under `renamed`; nothing is deleted, and a target without the old directory
  gets none). `skill list --json` carries `renamed`. `migrate` requires
  `aggountant-init`, `vero-fi` and `tulorekisteri`.
- Guide (fi/en): `start/move-from-another-program` gains the phased
  introduction and a section "Muut alueet myöhemmin / Other areas later".
  `start/install` lists `migrate` and explains the rename. `SKILL_RENAMED`
  has an error-code row. A CHANGELOG entry is under Changed.

No store migration.

## Tests

- `skill_cli.rs`: the list order and files, `renamed` in the list, the old
  name refused on print and install, and the pointer written only into a
  target that had the old skill (unchanged on reinstall, stale files kept,
  text output).
- `skill.rs`: `migrate_keeps_expense_claims_in_their_own_domain_file` pins
  the acceptance criterion for companies without claims. No file but
  `kululaskut.md` names a claim command, and only the `SKILL.md` domain table
  links to it. The existing link and command-path tests cover the new files.
- `scripts/ci-local.sh` passes (fmt, clippy, deny, stable and MSRV, the guide
  gate with the site build and the practice month).

## Review

Focused self-review, because the change is skill text plus the installer's
handling of the rename. It changes no bookkeeping, filing, store or authority
output. Checked: every command named exists in the binary (test), every link
resolves in the install (test), and the claimant input and the
`file-wage employer-report` subcommands match `--help`. Mixed VAT periods are
filed in OmaVero; the skill does not claim `complete-manual` can record them,
because that was not verified.

## Follow-ups

- When `expense-claim-draft`, `expense-claim-legacy-migration` and
  `three-year-site-clock` land, each should replace the matching lines of
  `skills/migrate/kululaskut.md` ("Odottaa ohjelmaa").
