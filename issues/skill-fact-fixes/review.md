# Review: skill-fact-fixes

- Skill: `/llm-review`, rigor **routine**: agent instructions, user-guide
  tables and a read-only `skill print` option. Nothing here writes the books
  or talks to an authority.
- Scope: the working-tree diff against `main` (`crates/cli/src/skill.rs`,
  `crates/cli/src/main.rs`, `crates/cli/tests/skill_cli.rs`, the four
  skills, `docs/user/{fi,en}/reference/error-codes.md`,
  `docs/user/{fi,en}/start/install.md`).
- Finder: `claude-fable-5-2`, one call (diff, the issue, `skill.rs`,
  `file_vat.rs`). Verifier: not called. Both candidates were text claims that
  a direct read of the cited code settled; see the note at the end. One model
  call in total.
- Risk class: documentation correctness (an agent following a wrong
  instruction).

## F1 (should-fix): `VERO_VAT_PERIOD_NOT_FILEABLE` is not an `error_code`

- Location: `docs/user/{en,fi}/reference/error-codes.md`, the new
  `VERO_VAT_PERIOD_NOT_FILEABLE` row.
- Claim: the page says `error_code` carries the code. This code is produced
  only as a preflight failure: `crates/process/src/file_vat.rs`
  (`read_fileable_vero_period` caller) journals it on the `reconcile` effect
  and returns `ProcessError::Store("<code>: …")`, which the CLI reports as
  `PROCESS_STORE`. A reader matching on `error_code` never finds the row.
- Evidence: local read of `file_vat.rs` (the `PreflightFailure` arm) and
  `crates/process/src/error.rs` (`Store` → `PROCESS_STORE`).
- Fix: the row now says where the code shows: in the message of a failed
  `file-vat start` (whose `error_code` is `PROCESS_STORE`) and on the run's
  `reconcile` step in `run show`. The misclassification as a system error is
  older than this change and outside the issue.

## F2 (should-fix): `aggountant-init` still says obligations come from the exceptional-receipt route

- Location: `skills/aggountant-init/SKILL.md`, the `aggountant obligation`
  bullet.
- Claim: after this change "the exceptional-receipt route" means asking the
  user and noting it in `TODO.md`. The sentence "Niitä syntyy vain
  poikkeuksellisten kuittien reitiltä" then promises a close-blocking
  obligation that never appears, so an agent could leave such a receipt out
  of `TODO.md`.
- Evidence: obligations are opened only by the consult Run
  (`crates/process/src/capture_receipt.rs`, `OpenObligation`); no other call
  site.
- Fix: the bullet now says obligations are not created by hand or by receipt
  work, so anything awaiting the close goes in `TODO.md`, matching the
  `capture-receipt` wording.

## Also applied

The finder noted that the new `TULOREKISTERI_REJECTED` row did not link the
troubleshooting section that covers it; the row now links
`troubleshooting.md#tulorekisteri-rejected-the-report` (fi:
`#tulorekisteri-hylkäsi-ilmoituksen`).

## Dropped

None. Two narrower-than-code remarks were left as the finder rated them, below
the threshold: the `EMPLOYER_REPORT_INVALID_EVIDENCE` row does not list "the
evidence names another report", and the `TULOREKISTERI_REJECTED` row does not
mention re-sending after the fix (the linked troubleshooting section does).

## Coverage

Finder ran the built binary for `--file` with own files, `../<required>/`,
a non-required skill, `./SKILL.md`, a doubled `../`, a fragment and an
unknown skill, and compared `skill print --file` output byte-for-byte with
the sources. `run show --run-id` as the place a rejection shows was checked
against `authority.rs`. The remaining new rows were checked against their
source sites. `scripts/ci-local.sh` passes.

Verifier note: `/llm-review` prescribes a verifier call for surviving
candidates; here both were documentation claims settled by reading the one
code site each cites, so a second model would have added cost and no
evidence.
