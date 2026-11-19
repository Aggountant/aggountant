# Review: employer-report-configure

- Skill: `/llm-review`, rigor **routine**: the command writes company
  configuration, not the books or a filing, but `registered_from` decides which
  months `missing` lists, so a silently wrong value hides late reports.
- Scope: commit `8a24014` against `ad4c219` (`crates/broker/src/tulorekisteri.rs`,
  `crates/process/src/file_employer_report.rs`,
  `crates/process/src/employer_report_history.rs`, `crates/cli/src/wage.rs`,
  `crates/cli/src/main.rs`, `crates/domain/src/payroll.rs`, tests, guide,
  skill).
- Finder: `gpt-6-astra`, one call on the diff. Verifier: `claude-fable-5-2`,
  one focused false-negative call on the high-risk boundary (the byte-keeping
  splice and the atomic write of the profile). Two model calls in total.
- Risk class: configuration integrity (profile corruption, wrong
  `registered_from`).

## F1 (should-fix): non-JSON output hid the before/after

- Location: `crates/cli/src/main.rs`, `human_line`, the generic
  `Command::FileWage` branch.
- Claim: without `--json` the command printed only `file-wage <verdict>`, so
  the guide's documented dry run showed none of the settings being reviewed,
  and a dry run, a write and an unchanged run looked the same.
- Evidence: local read and a manual run; the guide's commands omit `--json`.
- Fix: a configure-specific line that says written / unchanged / dry run and
  prints `before` and `after`. The CLI test checks the non-JSON dry run.

## Verifier on the splice (no defect found)

The verifier ran `with_employer_report` and `write_wage_profile` on 15 valid
and 11 hostile profiles (duplicate and escaped keys, the block first, middle
and last, look-alike text inside strings, CRLF, Unicode whitespace, `{}`)
and found no input that yields invalid JSON, alters another member, writes a
block different from the settings, or loses the old file. Two observations on
the same boundary:

- **V1, no repair path.** An existing `employer_report` block that no longer
  parses (a bad hand edit) made `configure` refuse, so the command that owns
  the block could not fix it.
- **V2, double read.** `configure_employer_report` read the file twice, once
  as text for the splice and once through `read_wage_profile` for `before`
  and the Business ID.

Side effects it listed as not corruption: a symlinked profile is replaced by a
regular file, the file mode is not kept, no directory fsync (power loss leaves
the old file), and a hand-formatted block with identical values is rewritten
once into the canonical layout (`changed: true`).

## Dropped

None from the finder; it returned one candidate.

## Coverage

Read: the whole diff, `compare_registration`, `read_wage_profile`, the
`vero-fi configure` command it imitates. Tested: workspace tests and clippy,
`scripts/ci-local.sh`, a manual run against the real YTJ (fictional Business
ID → `YTJ_COMPANY_NOT_FOUND`, nothing written). Outside scope: the YTJ client
itself (unchanged, reviewed in `employer-report-history`).
