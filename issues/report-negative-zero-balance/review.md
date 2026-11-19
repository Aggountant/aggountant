# Review: report-negative-zero-balance

- Skill: `/llm-review`, rigor **routine**: a display-format fix in statement
  output; no filing, sealing or ledger write path changes. The statement hash
  is part of the close-year approval binding, which is why the review was asked
  to look at hash compatibility.
- Scope: commit `65dcb25` against its parent (`crates/domain/src/statements.rs`,
  `crates/domain/src/lib.rs`, `crates/ledger/src/reports.rs`,
  `crates/ledger/tests/monthly_reports.rs`, `scripts/render-statement-pdf.py`).
- Finder: `claude-fable-5-2`, one call (diff plus the full `statements.rs` and
  `reports.rs`). No verifier: both candidates were settled by local evidence
  and the surface is not high-risk. One model call in total.

## F1 (must-fix): committed Python bytecode carries a home-directory path

- Location: `scripts/__pycache__/render-statement-pdf.cpython-312.pyc`, added
  by the slice when the PDF renderer's `fi_amount` was exercised by hand.
- Claim: the bytecode embeds the absolute worktree path, which the publication
  rule forbids, and `.gitignore` did not exclude `__pycache__`.
- Evidence: `git ls-files` listed the file. The commit was not yet pushed.
- Fix: removed from the commit (amended before push) and `__pycache__/` added
  to `.gitignore`.

## F2 (note): close-year statement bytes change for years with a cleared row

- Location: `crates/domain/src/statements.rs` (`statement_amount_str`),
  consumed by the close-year approval binding in
  `crates/process/src/close_year.rs` (`assert_statements_binding`).
- Claim: `Decimal::ZERO + (-0.00)` stays `-0.00`, so before this slice the
  close-year `Statements` could also carry a `"-0.00"` row (a credit-normal
  row that sums to zero). Those bytes, and so `statements_hash`, now change.
- Evidence (local read): sealed years are not recomputed — `verify` folds the
  stored transition hashes, the adoption check compares persisted statements,
  and `close_package_hash_for_year` has no production caller. A close-year Run
  that computed statements under the old binary and is approved under the new
  one fails closed ("the statements changed after approval was bound"); it
  never seals different figures. Such a Run can be rejected
  (`answer_close_year … reject`) and a fresh close started.
- Fix: none in code. Keeping `-0.00` for compatibility would keep the defect in
  statutory statements. Recorded in `result.md`.

## Dropped

None. The finder's "checked and clean" list (no other negative-zero path in
statement or report output, rescale-then-normalise order, tase-erittelyt skips
zero lines, the Python formatter, both new tests failing on the old code)
matches what was checked locally while implementing.

## Coverage

Read: the diff, `statements.rs`, `reports.rs`, the close-year binding and
reject path. Tested: `scripts/ci-local.sh` (fmt, clippy, deny, stable + MSRV
tests, practice month) passed; both new tests were confirmed to fail with the
normalisation removed. Not covered: VAT, payroll and wage-report formatters,
which negate amounts but are outside statement/report output.
