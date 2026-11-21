# Result: skill-soul-rebuild

`skills/aggountant-init/SOUL.md` keeps the role word for word and replaces the
tools section with three short ones: the house rules of the company home, what
is permanent and why the user approves it, and how to read the tool. No other
file changed; the task skills still carry their copies until their own slices.

## Words

| | Before | After |
|---|---:|---:|
| `SOUL.md` | 647 | 662 |
| role (unchanged) | 279 | 279 |
| the rest | 368 (partial command list) | 383 (house rules, permanence, reading the tool) |

The issue asked for about 500 words. The file did not get shorter: with the
role kept as it is, 500 would leave about 220 words for everything the other
skills are about to stop saying. The house-rule copies in `aggountant-init`,
`capture-receipt` and `vero-fi` come to roughly 590 words, and SOUL now says
the same in about 230; the saving lands when those slices delete their copies.
If 500 is a hard limit, the `TODO.md` bullet and the secrets paragraph are
where to cut, at the price of task skills carrying part of the rule again. The
review raised the same point; see F8 in `assessment.md`.

## What the old tools section said, and where it went

| Old content | Sorted as | Now |
|---|---|---|
| What the program is and checks (balance, accounts, attachment reference, numbering, chain) | the program shows it | dropped; one clause on sealing stays under "Mikä on pysyvää" |
| VAT calculation, income statement and balance sheet reports | the program shows it (`aggountant --help`) | dropped |
| `knowledge company` commands, flags, stdin shape, source-reference examples | the program shows it (each command's `--help` has an example) | dropped; the bullet says what belongs there |
| `knowledge list` and `knowledge show` belong to an unused consult route | a path that is not connected | dropped (`reference/status.md` records it) |
| Obligations can be listed and resolved | house rule | `obligation` bullet: the program's own list, not a notebook |
| Month close locks entries, an interrupted close can be resumed | permanence / the program shows it | "Suljettu kausi lukitsee kirjauksensa"; resuming dropped |
| Vero.fi readiness, the user's explicit approval, payment is separate, manual route | task knowledge (`vero-fi`) | dropped from SOUL; the approval principle is general under "Mikä on pysyvää" |
| Monthly employer's separate report reminder | a feature reminder placed in SOUL because everyone reads it | dropped from SOUL; see hand-over below |
| `aggountant --help`, a command's `--help`, `--json`, `--home`, `config show` | how to read the tool | kept, with `error_code`, `message`, `next_command` and free previews added |

New in SOUL, taken from the three task skills' copies: `AGENTS.md` (routes
and practices, not read by the program, ask instead of guessing a folder);
`TODO.md` (what it holds, kept current, the user edits it too, a memory list
and not the source of state); pending items are not `knowledge company`
facts; `obligation` is created by the program; secrets in none of them nor in
the conversation, with the reason; commit together with the work when the home
is a git repository; the three permanent acts, approval from what the user
sees, a changed proposal shown again; an uncertain submission is settled with
the program before anything is sent again.

## Fact check (binary built from this worktree)

- `config show` reports the home in effect and where the choice came from.
- `knowledge company` and `obligation` exist; `obligation` has only `list` and
  `resolve`, so obligations are never created by hand.
- A failing command answers with `error_code`, `error_kind` and `message` on
  stderr (`voucher show 999 --json`: `VOUCHER_NOT_FOUND`, exit 1).
- `message` and `next_command` are in status answers (`file-vat status`
  on a period without a filing: a `message`, `next_command: null`).
- `--dry-run` exists on `capture-receipt record`, `file-wage start`,
  `home upgrade` and `vero-fi configure`; `file-vat preview` and
  `file-wage preview` are read-only.
- Commands that contact an authority say so in their help (`file-vat status`,
  `vero-fi verify-portal`, `file-wage employer-report registration`).
- A closed month is not reopened by any command (`period` has only `show`).

## Review and assessment

`review.md` is the `/llm-skill-review` output (one external model, executor
and design-fidelity lenses), `assessment.md` the `/assess-findings` table.
Eight findings: four fixed in the text, two dropped, one sizing decision
(above), one product gap filed.

## Product gaps

- **Filed, `virtually-tidy-owl`**: the month close's `ApproveClose` gate
  answers with a snapshot hash and no content, so SOUL's "the user approves
  what they see" has nothing to show for the most frequent permanent act.
  `close-year-gates-show-content` fixed the same for the year-end close.
- **Not filed, no observed failure in this slice**: `next_command` appears in
  status answers but not in errors, so "follow `next_command`" covers less
  than it reads. The skills review already proposed an error hint.

## Hand-over to the following skill slices

- `skill-init-rebuild`: SOUL links to `aggountant-init/SKILL.md` for creating a
  missing `AGENTS.md` or `TODO.md`, so the questions the company's `AGENTS.md`
  answers and the three `TODO.md` sections stay there; the what-goes-where
  list, the secrets sentence and the commit sentence can go. SOUL does not
  mention `home upgrade`.
- `skill-month-rebuild`: the `TODO.md` and `knowledge company` paragraphs can
  go. Until the close gate shows content, the skill needs to say what the
  agent shows the user before a month close. The employer's separate report
  reminder that SOUL carried (`file-wage employer-report missing` at month
  end) needs a home at the month's hand-over to filings.
- `skill-vero-split`: the general `TODO.md` paragraph and "previews are free"
  can go. What is specific to filings stays: which `TODO.md` entry a filing
  that is received but not yet processed leaves, and which acts need the
  user's explicit decision.

## Simplicity

Reused: the existing file, link and packaging test; no new file, no change to
`skill.rs` or the guide (the guide calls SOUL the shared role description,
which still holds). Left out on purpose: a list of commands or states, any
approval procedure per task, and delegation rules for approvals, which the
task skills state where they apply.
