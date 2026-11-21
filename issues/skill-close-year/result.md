# Result: skill-close-year

`skills/close-year/SKILL.md` is new: the year-end close had no skill. It is
registered last in `crates/cli/src/skill.rs` (requires `aggountant-init` for
SOUL), listed in the guide's install page in both languages, and
`crates/cli/tests/skill_cli.rs` counts five skills.

## Words

| | Before | After |
|---|---:|---:|
| `skills/close-year/SKILL.md` | 0 (no skill) | 855 |

The first draft was 721 words; the review's eight accepted fixes added 134.
The issue asked for about 800.

## What the skill says, and where it came from

There was no earlier file to sort, so the sorting was done on the sources: the
guide's `tasks/year-end.md` and `reference/status.md`, the run's code and help,
and a full synthetic close walked in a scratch home with the binary built from
this worktree (twelve months closed, one asset, one accrual, one correction,
four gates, year sealed, then the refusals around it).

| Content | Source | Sorted as |
|---|---|---|
| The close is for getting year-end entries into the closed year and sealing it; the official document, signatures and filings are outside | guide | purpose: kept |
| Seal only when the preparer's figures are final; nothing enters a sealed year | `crates/ledger/AGENTS.md`, guide | knowledge and approach: kept |
| The new year's monthly reports refuse with `PERIOD_NOT_SEALED` until the prior year is sealed, while vouchers, month closes and VAT continue | observed in the scratch home | knowledge the guide does not state: kept |
| Only the run's own entries are attributed to the closed year; an ordinary voucher lands in the new year's result | guide, code | knowledge: kept |
| `close-year start` writes nothing to the books and shows the computed depreciation | code, observed | knowledge: kept (it is what makes start safe to run) |
| Depreciation is computed at the class's maximum rate and cannot be given; a different plan goes in as a correction | `domain/closeyear.rs`, the refusal for a year that is not 12 months | trap: kept |
| No fixed-asset register: assets and opening book values are given every year; record the closing values in `knowledge company` | code (stateless input) | trap, and the point where a SOUL house rule applies: kept |
| Accruals are not reversed in the new year; the invoice may already be expensed there | code (no reversal), review F2 | trap: kept |
| A correction is sealed on its own, before the first gate, so it needs the user's approval first | code, observed | permanence: kept |
| What the user decides at each of the four gates | guide (names only), gate content observed | kept; the judgment per gate is new text |
| Notes cover program-computed depreciation only; narrative notes are made elsewhere | `build_liitetiedot`, `narrative_pending` | kept |
| Rejecting a gate does not undo sealed vouchers; a new run carries them with the same id | guide, natural keys in code | permanence: kept |
| `close-year resume` completes what was approved, the year seal included | code | kept: it is not a read, which SOUL's "reads are free" would otherwise suggest |
| Preconditions (months sealed, open obligations, unresolved intake), duplicate ids, a 13–18 month year, no open period in the next year | the program refuses with a message naming the blocker | dropped: the program shows it |
| Input format, gate and flag names, command sequence | `close-year start --help` (INPUT section), `close-year answer --help` | dropped; gate names appear once each as labels for what is decided there |
| One run sealing the year leaves the others only rejectable; run state names | the program shows it | dropped |
| Assembling the hand-over material for the preparer (reports, voucher list, attachments) | guide steps 1–3; ordinary work for a capable agent | dropped |
| Approval rules, secrets, how to read the tool | SOUL | not repeated; the skill names the two points where an approval applies |

Nothing about the month close is in the file.

## Fact check

Every command the skill names exists in the built binary (`close-year start`,
`close-year seal-correction`, `close-year show`, `close-year resume`,
`file-vat status`, `knowledge company`); the packaging test checks the paths.
Checked by hand against the binary and code: the four gate ids, the error code
`PERIOD_NOT_SEALED`, that the archive is written under `archive/<year>/`, that
a correction can only be sealed while the first gate is open, and that approval
of the first gate is refused until every correction is sealed.

## Review

`/llm-skill-review` (one model, executor and edge-case lenses) is in
`review.md`, the assessment in `assessment.md`: 8 FIX, all applied; 4 DROP.

Simplicity: the skill reuses SOUL for the approval and house rules and the
program's help and refusals for everything mechanical; it adds no reference
file and no second copy of the gate sequence. The guide's task page still
lists the commands for people; that repetition is the guide's job.

## Product gaps

None of these is fixed here; each is outside the slice's files. No skill line
waits on them except where noted.

1. **`close-year` output carries no `message` or `next_command`.** `start`,
   `answer`, `seal-correction` and `resume` return the run view only. SOUL
   tells the agent to follow `next_command`; here it has to read the next gate
   from the `gates` list. A message naming the open gate and what its approval
   seals would let the skill drop the "kaksi vaiheista on pysyviä" labelling.
2. **`close-year answer --answer` defaults to `approve`.** Leaving the flag
   out approves, also on `ApproveStatements`, which seals the year. An
   irreversible approval should be explicit. The flags `--gate`, `--answer`,
   `--run-id` and `--correction` also have no help text of their own; the gate
   names are only in the command summary.
3. **Help text speaks in internal labels.** `close-year --help` and its
   subcommands say "RF6/S6", "D7 corrections", "reconciles `SealYear` to
   fiscal". An agent cannot resolve these; the skill had to say what a
   correction is.
4. **No way to see depreciation rates before starting a run.** The rate and
   method per asset class appear only in the output of `close-year start`.
   Starting is harmless to the books, which the skill now says, but a read or a
   `--dry-run` would be the canon's way.
5. **No fixed-asset register.** Opening book values are typed in every year
   and the program cannot notice a wrong one; the skill routes them through
   `knowledge company`. A wrong value silently gives wrong depreciation in a
   year that is then sealed. When a register exists, the paragraph "Poistot"
   loses its last three sentences.
6. **The archive write after `SealYear` is not checked.** `run_seal_year`
   discards the result of `write_archive_stub`, so a failed write (full disk,
   permissions) leaves a sealed year without `archive/<year>/tilinpaatos.json`
   and nothing says so; `resume` rewrites it only when it reconciles a seal it
   had not recorded. The books are right either way. Assessment F12.
7. **The guide does not say that the new year's monthly reports wait for the
   year seal.** `report statements` for a month of the new year refuses with
   `PERIOD_NOT_SEALED` while the prior year is open. `tasks/year-end.md` and
   `tasks/monthly-report.md` should say so; a user will meet it every spring.
