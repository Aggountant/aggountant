# Review: close-year-adjustment-attribution

- Skill: /llm-review, rigor critical (irreversible SealYear of a fiscal year; silently wrong sealed statements are a data-integrity failure).
- Finder: gpt-6-astra (one call, `--task review`, diff 0da5525..5c0727a with close_year.rs, ledger lib.rs, sqlite_fiscal.rs and the close-year tests attached).
- Verifier: claude-fable-5-2 (one call, asked to refute both surviving candidates).
- Model calls: 2. Scope: `crates/process/src/close_year.rs`, `crates/process/src/lib.rs` (workflow version), `crates/process/tests/close_year.rs`, `crates/domain/src/closeyear.rs` (one message), `docs/user/{fi,en}` year-end, status, for-bookkeepers and periods pages.
- Risk class: high (irreversible fiscal-year seal, statements binding).

## F1 (must-fix): resume seals approved statements without re-checking the binding

- Location: `crates/process/src/close_year.rs`, `run_seal_year` (called from `resume_close_year`'s approve-then-crash branch).
- Claim: `run_seal_year` passed the persisted statements hash to `Ledger::seal_year` after checking only that this Run's own vouchers were attributed. The fresh recompute-and-compare (`assert_statements_binding`) ran only on the answer path. If `SealYear` failed or was interrupted after `ApproveStatements` was recorded, and another close-year Run for the same year then sealed and attributed a correction (or a v2 Run carried over by `run migrate` had statements computed before attribution existed), resuming the first Run sealed a hash that no longer matched the year's data.
- Evidence: local reading confirmed (`Ledger::seal_year` checks hash syntax and subperiod state only). Verifier: survives, with the sequence reachable in single-user sequential use. Regression test `a_resumed_run_does_not_seal_statements_another_run_has_made_stale` reproduces it: without the fix the year is sealed with the stale hash.
- Impact: an irreversibly sealed year whose bound statements omit an attributed voucher.
- Fix: `run_seal_year` calls `assert_statements_binding` before `EffectExecuting`.

## F2 (should-fix): the guide's command block skips two gates

- Location: `docs/user/en/tasks/year-end.md` and `docs/user/fi/tasks/year-end.md`, "Commands the agent runs".
- Claim: the block answered `ApproveAdjustments` and then `ApproveStatements`; `ApproveTaseErittelyt` and `ApproveLiitetiedot` open in between, so the second command fails with `GATE_NOT_OPEN`.
- Evidence: `answer_close_year` gate sequence; verifier: survives.
- Fix: all four answer commands listed in both languages.

## Dropped candidates

None. The finder raised two candidates; both survived.

## Coverage and residual risk

Read: the full diff, `close_year.rs`, the ledger attribution and `fiscal_year_data` / `year_vouchers` / monthly-report projection paths, `migrate`. Tested: `cargo test -p aggountant-process --test close_year` (19 tests), full `scripts/ci-local.sh`, `scripts/check-user-guide.sh`. Outside scope: VAT behaviour (`crates/domain/src/vat`, `file_vat.rs`), concurrent processes (single-user v1).
