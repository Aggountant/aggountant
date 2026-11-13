# Review: rejected-close-reopens-period

- Skill: /llm-review, rigor routine; high-risk surface (fiscal period state machine, authority over Closing periods, crash recovery of an irreversible-seal workflow).
- Finder: gpt-6-astra (one call, `--task review`, diff HEAD~1..HEAD = a758514 with `crates/process/src/lib.rs`, `crates/ledger/src/store/sqlite_fiscal.rs`, `crates/ledger/AGENTS.md`, `crates/AGENTS.md` attached).
- Verifier: claude-fable-5-2 (one call: refute F1, and probe whether the fix itself lets `resume` take Open→Closing for a Run that should not).
- Model calls: 2. Scope: `crates/ledger/src/lib.rs` (reject_close, legal edges, verify_periods), `crates/ledger/src/period.rs`, `crates/process/src/lib.rs` (answer, resume, run_reject_close), tests, `docs/user/{fi,en}` close/period/status/troubleshooting/practice/error-code pages.
- Risk class: high.

## F1 (should-fix): a new close after a rejection cannot resume a crash before its fiscal commit

- Location: `crates/process/src/lib.rs`, `resume`, the retry branch for a crash between `EffectExecuting("transition")` and `begin_close`.
- Claim: the branch required `fiscal.state == Open && owning_run_id.is_none()`. Every transition writes its run as owner (`sqlite_fiscal.rs`, `transition_period`), so after `reject_close` the Open row names the rejecting run and the retry never fires. The new Run stays `running` with no gate.
- Evidence: local reading confirmed; regression test `a_new_close_after_a_rejection_resumes_a_crash_before_its_fiscal_commit` fails with `left: Running, right: AwaitingGate` when the old condition is restored. Verifier: survives.
- Impact: the Run is wedged; the period stays Open and a fresh close still works, so no books damage.
- Fix: drop the `is_none()` condition (an Open period's owner is history, `begin_close` does not match it).

## F2 (should-fix): the relaxed retry skipped the open-obligation gate

- Location: `crates/process/src/lib.rs`, same branch.
- Claim (raised by the verifier while checking F1's fix): `start_close_period_seam` refuses a period with an open obligation, but the resume retry called `begin_close` without the check, so a crashed Run could close and seal past an obligation opened after the crash. Pre-existing for never-closed periods; F1's fix extends it to the post-rejection case.
- Evidence: verifier ran the sequence (reject → crashed start → obligation created → resume → approve → Sealed with an open obligation). Local reading confirmed no check in `resume`, `answer_seam` or `run_seal`.
- Impact: breaks the `crates/AGENTS.md` rule that an open obligation blocks close-period.
- Fix: one `refuse_open_obligations` helper used by start and by the retry; the regression test asserts `OPEN_OBLIGATION` with the period still Open, then succeeds after the obligation is resolved.

## Unresolved uncertainty

- An obligation created while the period is already Closing (after a successful start) is not re-checked at approve. Pre-existing, not touched by this change; in this version obligations come only from the consult route, which is not usable yet.

## Dropped candidates

None. The finder raised one candidate (F1); it survived. F2 came from the verifier's safety probe of the fix.

## Coverage and residual risk

Read: full diff, `transition_period` CAS and rejection logging, `verify_periods`, `acquire_authority`, every `ClosingAuthority` caller, the close-period answer/resume paths. Tested: `cargo test -p aggountant-ledger --test period`, `cargo test -p aggountant-process`, `cargo test -p aggountant --test close_period_cli`, full `scripts/ci-local.sh`, `scripts/check-user-guide.sh`. Outside scope: VAT (`crates/domain/src/vat`, `file_vat.rs`), concurrent processes (single-user v1).
