# Review: wage-report-onboarding-docs

- Skill: `/llm-review`, rigor **routine**: the change adds an input form for an
  existing approval check and does not change what is sent or when. The
  approval itself (the engine compares the given hash with the recomputed one)
  is untouched.
- Scope: the slice's first commit against `97a81fc` (`crates/cli/src/wage.rs`,
  `crates/cli/tests/file_wage_cli.rs`, `skills/tulorekisteri/SKILL.md`,
  `skills/tulorekisteri/kayttoonotto.md`, the user guide's travel-allowance
  page).
- Finder: `gpt-6-astra`, one call. Verifier: none; the one candidate was
  confirmed against the code locally and fixed, so a refutation call had
  nothing left to decide. One model call in total.
- Risk class: approval binding of an irreversible authority filing.

## F1 (must-fix): the documented pipe regenerated the approval instead of carrying the reviewed one

- Location: the first commit's examples in `crates/cli/src/wage.rs`
  (`Approve` and `CorrectionCmd::Submit` help),
  `skills/tulorekisteri/SKILL.md` and
  `docs/user/*/tasks/report-travel-allowances.md`.
- Claim: the examples ran a new preview at approval time and piped its hash
  into `approve` / `correction submit`. The engine checks the given hash
  against the hash recomputed from the input at that moment
  (`crates/process/src/file_wage_correction.rs`, `submit_wage_correction`), so
  a report file changed after the user reviewed it would be approved with the
  changed content. For `correction submit` nothing else binds the reviewed
  preview; for `approve` the run stored at `start` binds only what `start`
  read.
- Evidence: local read of `submit_wage_correction` and `approve_wage`; the
  existing process test that refuses a changed report relies on the hash being
  the earlier one.
- Fix: the examples now capture the hash from the very preview the user
  reviews into a private `mktemp` file outside the home
  (`... | tee >(jq -r .record_hash > FILE) | jq 'del(.record_hash)'`), give
  that file to `--record-hash-file`, and delete it afterwards. Help, skill and
  guide say not to regenerate the hash at approval time. The pipeline was run
  by hand against a scratch home: the captured file is mode 600 and equals the
  preview's `record_hash`.

## Dropped

None.

## Coverage

Read: the whole diff, `approve_wage`, `submit_wage_correction`,
`submit_employer_report` hash checks and their error messages (none echoes the
hash), clap's conflict message (prints placeholders, not values). Tested: the
CLI tests for stdin, file, empty, multi-token, missing-file, both-forms and
neither-form inputs; `-` refused for a replacement correction. Outside scope:
the engine's approval logic, which this slice does not change.
