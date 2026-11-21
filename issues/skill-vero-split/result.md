# Result: skill-vero-split

`vero-fi` is now the VAT skill, and the Incomes Register has its own skill,
`tulorekisteri`. Each has a body for the work done every month and reference
files for what is needed rarely.

## Word counts

| File | Before | After |
|---|---|---|
| `skills/vero-fi/SKILL.md` | 4 196 | 773 (723 without frontmatter) |
| `skills/vero-fi/kayttoonotto.md` | | 415 |
| `skills/vero-fi/hairiot.md` | | 404 |
| `skills/tulorekisteri/SKILL.md` | | 634 (596 without frontmatter) |
| `skills/tulorekisteri/kayttoonotto.md` | | 379 |
| `skills/tulorekisteri/kiinnikurominen.md` | | 411 |
| Total | 4 196 | 3 016 |

All skill text went from 8 884 to 7 707 words. What an agent reads for an
ordinary month's VAT return went from 4 196 words to 773.

The bodies are over the issue's "about 600" and "about 500". They were 680 and
510 before the review; the review found seven places where the agent would have
filed or recorded something wrong, and each fix is one to four sentences in the
body where the agent meets the case. Both stay well under the thousand-word mark
in `skills/AGENTS.md`.

## What moved where

| Old section of `vero-fi/SKILL.md` | Now |
|---|---|
| Purpose, irreversibility | `vero-fi/SKILL.md`, opening |
| Mikä ratkaisee valmiuden | two sentences in `vero-fi/SKILL.md` (status is the one assessment; relay blockers by owner); the list of readiness conditions is the status output |
| Salaisuudet, Profiili ja `configure`, Käyttöönotto, certificate renewal | `vero-fi/kayttoonotto.md` |
| ALV-ilmoitus | `vero-fi/SKILL.md`, "Tavallinen kuukausi" |
| Vastaanotettu, käsittely vahvistamatta | principle in the body; the dated observation (13–14 Sep 2026) and what to collect for a call in `vero-fi/hairiot.md` |
| Epävarma lopputulos, Keskeytynyt lähetys | principle in the body ("Kun lähetyksestä ei ole varmuutta"); `vero-fi/hairiot.md` keeps why one "not found" is not enough and the two cases where the program names no next command |
| Manuaalinen reitti | `vero-fi/SKILL.md`, "Käsin annettu ilmoitus" |
| Käyttäjän aika ja päätökset | SOUL already says it; the two set-up decisions are named in `kayttoonotto.md` where they arise |
| Määräajat kodin `TODO.md`-tiedostossa | one paragraph in each body on when a deadline may be removed; the rest is SOUL's |
| Tulorekisterin matkakorvaukset | `tulorekisteri/SKILL.md`; permission, profile, credentials and the earner in `tulorekisteri/kayttoonotto.md` |
| Työnantajan erillisilmoitus | `tulorekisteri/SKILL.md`; `configure` knowledge in `kayttoonotto.md`; catch-up in `kiinnikurominen.md` |

## What was dropped, and why

The program shows it (checked in the binary built from this worktree):

- stdin JSON for `vero-fi configure`, `file-vat start` and `file-wage preview`,
  and the `employer-report configure` flags: each has an INPUT section or
  full flag help.
- "`configure` replaces the whole profile": no longer true. It merges and
  refuses to drop verification evidence. The skill keeps one sentence on what
  `--replace` costs.
- State names of the filing state machine (`interrupted_filed`,
  `closed_not_filed`, `superseded`, `adopted`, …): `file-vat status` and
  `file-vat reconcile` answer with a `message` and `next_command` that say where
  the filing stands. I read every message in
  `crates/process/src/file_vat/status.rs`; each tells the agent what to do.
- About thirty error codes with explanations: `aggountant explain <CODE>` and
  the `hint` field carry them.
- `record_hash` handling detail and the test seam's mechanics: the output has
  `sensitive.record_hash` and `test_seam`. Each skill keeps the one sentence of
  consequence (the hash is derived from the identity code and is itself
  sensitive; with `test_seam` a period can be marked filed though nothing was
  sent).
- The comparison values of `registration` and the result fields of
  `import-transcript`: in their help and output.

Implementation and history: TLS and redirect behaviour, "not the hash of the
signed SOAP frame", the old `MALFORMED:`/`HTTP_2xx:` codes, `DDVS0280`, the
three verification endpoint names, the journal-file warning.

Procedure a capable agent does anyway: "run `--dry-run` first", "tell the user",
"do not shape the value to fit", repeated approval reminders (SOUL states the
approval once).

Dropped where I could not be sure nothing was guarded: the note that
`run-payroll` is a synthetic trial. The body now says "Rahapalkkoja se ei
ilmoita" and names no path that is not connected, as `skills/AGENTS.md` asks;
`reference/status.md` in the guide carries the rest.

## Installs

`vero-fi` keeps its name and its `SKILL.md` is replaced in place, so nothing is
left stale: no file was renamed or removed. `vero-fi` now requires
`tulorekisteri`, and `tulorekisteri` requires `vero-fi`.

The first direction is for existing installs. Both production companies' agents
know `vero-fi` as the place for Incomes Register work. `skill install vero-fi`
must therefore still deliver that knowledge, and the VAT body ends with a link
to the new skill, which the link test only allows to a required skill. The
second direction is one fact, one home: both channels use the same credential
file, described once in `vero-fi/kayttoonotto.md`, and `tulorekisteri` links to
it. `with_requirements` already ends a cycle; the unit test now covers it.
`skill install` without a name installs everything as before.

## Other files changed with the split

- `crates/cli/src/skill.rs`, `crates/cli/tests/skill_cli.rs`,
  `crates/cli/src/main.rs` (help text of `skill install`).
- One-line pointers that named `vero-fi` for Incomes Register work:
  `skills/capture-receipt/SKILL.md`, `skills/e-services/SKILL.md`,
  `skills/e-services/tulorekisteri.md` (text only, no date moved),
  `skills/AGENTS.md`.
- Guide, both languages: `start/install.md` (six skills),
  `tasks/report-travel-allowances.md`, `reference/status.md`.

## Simplicity

Reused: the bundling of reference files and `requires` as they were; no
installer change. Removed: the second copy of the filing state machine, of the
commands' input formats and of SOUL's house rules. Left out on purpose: a
`tulorekisteri/hairiot.md` (the body's principle plus the program's messages
cover it), any alias or stub for old section names, and any change to
`e-services` beyond the pointer.

## Review

`review.md` (one model, executor and cross-skill lenses) and `assessment.md`:
10 fixes applied, 2 spun off, 4 dropped. The fixes that mattered most: the first
draft offered an empty OmaVero view as evidence for filing by hand, against its
own body; the missing-tax route pointed at "the preview figures are the figures
to file"; and both skills promised that a correction made in the e-service is
recorded with `complete-manual`, which it is not.

`wage-report-onboarding-docs` (open) had the exact certificate-service path and
labels; they are now in `tulorekisteri/kayttoonotto.md`. Its `record_hash`
criterion is still open. Its collision token was re-mapped from
`skills/vero-fi/SKILL.md` to `skills/tulorekisteri/SKILL.md`.
`vat-run-payment-terminal` keeps `skills/vero-fi/SKILL.md`: the paragraph it
must update is "Veron maksaa käyttäjä itse…" in that file.

## Product gaps

Filed unlaned through intake:

- `tulorekisteri-profile-no-command`: the Incomes Register profile has no
  command, no help and one undifferentiated readiness message. The skill keeps
  the JSON example until then; the issue names the lines to remove.
- `filing-correction-unrecordable`: a correction made in the e-service cannot
  be recorded, and `file-wage complete-manual` on an accepted Run succeeds
  without recording anything.
- `employer-replacement-external-unchecked`: a replacement for a month recorded
  from outside is built from the current profile with no pension-insurance
  comparison.

Not filed: the reviewer's claim that the statutory deadline for tax-free
reimbursements is the 5th of the following month, not payment date + 5. The
guide calls the program's date deliberately cautious, and I did not verify the
rule. The skill now says "varovainen määräpäivä".

## Verification

`scripts/ci-local.sh` passes. Every command path, flag, field and code the six
files name was checked against `./target/release/aggountant … --help`, against
output in a scratch home (`vero-fi status`, `file-vat status`,
`file-vat preview`, `explain`), or, for network commands, against the code.
Not verifiable here: the authorities' side (the 12-hour delay, the certificate
service's labels, guidance section 4.3, the two-year certificate life), carried
over from the old skill and the open issue as dated or attributed statements.
