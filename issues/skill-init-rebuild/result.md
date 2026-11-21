# Result: skill-init-rebuild

`skills/aggountant-init/SKILL.md` keeps the set-up knowledge, states the
opening voucher's cases once each, creates the home's `AGENTS.md` and
`TODO.md` without restating their rules, and reduces `home upgrade` to when to
reach for it. No other shipped file changed.

## Words

| | Before | After |
|---|---:|---:|
| `SKILL.md` (whole file, `wc -w`) | 1 398 | 935 |
| lead and technical initialisation, with `home upgrade` | 252 | 182 |
| basic facts, fiscal year, opening balances, open matters | 573 | 530 |
| `AGENTS.md` and `TODO.md` | 506 | 189 |
| Vero.fi pointer | 43 | 15 |

Section figures are measured from the body per heading, grouped so that old
and new compare (the old file had one section for facts, fiscal year, opening
and open matters; frontmatter makes up the rest). The issue asked for about
800. The file is above that because the house-rule sections gave up about 320
words while the set-up knowledge, which the issue says stays, gave up about
40: the `fiscal-year define` restatement went, and closing four real gaps the
review found in the opening-voucher text (which fiscal year to define,
mid-month cut-over, the order of questions and approval, the year-to-date
check) took most of that saving back. That is the part where a mistake is
permanent.

## What the old file said, and where it went

| Old content | Sorted as | Now |
|---|---|---|
| Purpose; the same skill continues an unfinished set-up | purpose | kept |
| Commands for looking at what the home has (`config show`, `chart list`, `voucher verify`, …) | what a capable agent does anyway | dropped; "look at what the home already has" |
| `init` writes structures, chart and `.gitignore`; asks nothing about the company; no git repository | knowledge (`init --help` does not say it) | kept, shorter |
| Home lookup order is in `aggountant --help` | the program shows it | dropped |
| `init` refuses an existing home | knowledge with a consequence | one clause under "Myöhemmin" |
| `home upgrade`: what it adds, `skipped`, `differs`, `unavailable`, `masterdata_opens`, commit the files | the program shows it (`home upgrade --help`) | dropped; kept the two triggers and why files are not repaired by hand |
| Basic facts into `knowledge company`, with a document as the source | knowledge | kept |
| `fiscal-year define`: first/last day of month, 18 months, KPL 1:4, later years 12 months, identified by ending year | the program shows it (`fiscal-year --help`) | dropped |
| Define before the first voucher; permanent; no year before the first defined one unless it starts 1 January | knowledge, order with a reason | kept |
| Opening voucher through `voucher seal`, balance list attached | knowledge | kept |
| Mapping file; keep tax account and shareholder loans apart; opening VAT balance shows as untagged, `vero-fi` settles it | knowledge | kept |
| Opening date and the three cases | knowledge | kept, each case once; the stray warning inside the third bullet is now the section's lead |
| `external_cutover` in the report's sources | implementation | dropped |
| Correction leaves three vouchers instead of one | consequence | kept as the reason to show the user first |
| Check with `report statements` after the opening | knowledge | kept, corrected for the year-boundary case |
| Open matters at cut-over; one overall picture; confirmed readings into `knowledge company` | knowledge | kept |
| `AGENTS.md`: what it is, that the month's work reads it, keep an existing one | house rule (SOUL) | dropped |
| `AGENTS.md`: what to ask the company, mileage-log fields, the git boundary | set-up knowledge | kept as a list |
| `AGENTS.md` tells every session to read `TODO.md` first and update it; commit together; no secrets | house rule (SOUL says all three) | dropped |
| `TODO.md`: why it exists, belongs to git, do not overwrite, three sections | house rule (SOUL) | dropped; a new file gets a section for each thing SOUL names, and set-up's leftovers go in at once |
| Esimerkki Oy example of a `TODO.md` | knowledge by example | kept |
| "What goes where": `TODO.md` / `knowledge company` / `obligation` | house rule (SOUL) | dropped |
| Vero.fi section | pointer | one sentence |

## Changed meaning, to be checked by the owner

- **Year-boundary cut-over when the fiscal year does not start 1 January.**
  The old text allowed two routes without saying which: define the old year
  and open on its last day, or define the new year and open on its first day.
  The rewrite recommends the second and reserves the boundary cut-over for
  calendar-year companies. That is my reading of which is simpler (no strict
  condition to meet, no old year in the home), verified to work on a synthetic
  home but not a decision found in the code or the old text.
- **Mid-month cut-over.** New sentence: the imported result lands in that
  month's result whatever the date, so the agent proposes a month boundary.
- The instruction that the home's `AGENTS.md` should itself tell sessions to
  read `TODO.md` is gone, because SOUL says it to every session. Homes set up
  earlier keep the line; it is harmless.

## Fact check (binary built from this worktree, synthetic home)

- `init` writes `fiscal.sqlite`, `artifacts/`, `masterdata/` and a
  `.gitignore` covering the database, artifacts and the PII store; no `.git`;
  a second run answers `ALREADY_INITIALIZED`.
- `fiscal-year define --start 2026-07-01 --end 2027-06-30` gives year 2027;
  `fiscal-year show --date 2026-06-30` then answers `FISCAL_YEAR_UNDEFINED`.
  The 1 January exception is in `crates/model/src/fiscal_year.rs` (`year_of`).
- A balance-sheet-only voucher seals on 2026-07-01 in that home and
  `report statements --period 2026-07` runs.
- The boundary cut-over condition and `PERIOD_NOT_SEALED` are as
  `crates/ledger/AGENTS.md` states and `crates/ledger/tests/opening_balances.rs`
  asserts.
- `home upgrade` requires `--dry-run` or `--apply`; its help covers what it
  adds and skips. A home missing `vat-account-roles.json` answers
  `MASTERDATA_CORRUPT` on `chart list` and `voucher verify`.
- `report statements`, `knowledge company` and `chart list` exist as named.
  `vero-fi/SKILL.md` still explains the opening VAT balance (line 164).
- The skill names `vero-fi` without a link: `aggountant-init` does not require
  it, so a link would not resolve in an install (the packaging test checks
  this).

## Product gaps

- **Filed, `nominally-undesirable-afterthought`**: `MASTERDATA_CORRUPT` for a
  missing bundled file names the file but not `home upgrade`. Until it does,
  the skill's last paragraph keeps that trigger; the issue names the clause to
  remove.
- **Filed, `distinctly-tired-hearing`**: the skill and the guide ask whether
  to continue the previous chart of accounts, and nothing installs one. The
  question stays in the skill as inherited until that is decided.
- Not filed: `init --help` does not say that it writes a `.gitignore` or
  leaves git alone. The skill carries it in two sentences; small, and the
  guide's set-up page says it too.

## Gates

`cargo test -p aggountant skill` passes (command paths and relative links).
`scripts/ci-local.sh` passes (fmt, clippy, cargo-deny, build and test on stable and the MSRV).
The guide needed no change: no command changed, and its set-up page already describes the same flow.
