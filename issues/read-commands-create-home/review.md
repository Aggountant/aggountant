# Review: read-commands-create-home

- Skill: `/llm-review`, rigor **routine**. The change only adds a refusal in
  front of existing paths; its worst failure is refusing a real home (a
  visible stop, no write), not wrong books. A missed call site would keep the
  old behaviour, which the regression test covers.
- Scope: commit `b22ab4e` against `8a8d4d9` (`crates/cli/src/main.rs`,
  `crates/cli/src/bank_import.rs`, CLI tests, user guide).
- Finder: `gpt-6-astra`, one call, given the diff plus `main.rs`,
  `crates/ledger/src/lib.rs` and `crates/process/src/lib.rs`, and asked for
  unguarded home-opening paths, commands wrongly blocked, a misleading
  resolution source, real homes wrongly refused, and harmful error-ordering
  changes. Verifier: none (finder clean, surface not high-risk by the skill's
  list). One model call in total.

## Findings

No findings. The finder answered "No findings" after reading the whole
`run()` match and the callees.

## Local checks (host)

- Every remaining `Ledger::open`, `ProcessEngine::open` and `Intake::open` in
  `crates/cli/src` receives a path that came from `existing_home`; `home_path`
  is used by `init` only. `reports.rs` and `capture-receipt record` call
  `Ledger::open(home)` on a home already checked by the caller.
- Commands that do not open a home are untouched: `config path/show`,
  `explain`, `skill`, `voucher draft`, `voucher vat-codes`,
  `import camt053 parse`.
- `file-wage preview`, which reads as offline, still needs the home: the
  process reads the employee's HETU from the home's PII store
  (`crates/process/src/file_wage.rs`, `wage_request`). Gating it is correct.
- The definition of an initialized home was checked against the fixture of a
  home written by an earlier binary (`tests/fixtures/main-home-calendar-2026`,
  which has no `artifacts/` because git drops empty directories): requiring
  `artifacts/` and `pii.sqlite`, as the earlier voucher-read preflight did,
  refused that home. The gate now requires `fiscal.sqlite` and
  `masterdata/reference/` only, with a test for both directions.
- `scripts/ci-local.sh` passes (fmt, clippy, deny, stable and MSRV build and
  test, practice month).

## Dropped candidates

None proposed.

## Coverage and residual risk

Read: the diff, `run()` in full, `bank_import.rs`, `reports.rs`, `wage.rs`,
`tulorekisteri.rs`, `home.rs`, `knowledge.rs`, `Ledger::open/init`,
`ProcessEngine::open`. Not covered: a production home damaged so that
`fiscal.sqlite` is missing but other parts remain is now refused instead of
silently opened with empty books; that is the intended behaviour.
