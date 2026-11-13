# Assessment: close-year-adjustment-attribution review

Source review: `history/review-close-year-adjustment-attribution.md` (copied beside the issue as `review.md`). Head: `5c0727affeae`.

| id | title | confirmation | likelihood | recommendation | applied |
|---|---|---|---|---|---|
| F1 | Resume seals approved statements without re-checking the binding | CONFIRMED | RARE | FIX | yes |
| F2 | Guide's close-year command block skips two gates | CONFIRMED | REGULAR | FIX | yes |

## F1: Resume seals approved statements without re-checking the binding

- Target: `crates/process/src/close_year.rs`
- Claim: resume_close_year's approve-then-crash branch calls run_seal_year, which sealed the persisted statements hash without recomputing the statements; another close-year Run (or a migrated v2 Run's statements) could make the bound hash stale and the sealed year would not match its data.
- Fix: Call assert_statements_binding inside run_seal_year before EffectExecuting.
- Evidence: Regression test a_resumed_run_does_not_seal_statements_another_run_has_made_stale seals the year with the stale hash when the check is removed and refuses with JOURNAL_CORRUPT with it. Verifier (claude-fable-5-2) confirmed no recomputation on the resume path. Data integrity of a sealed year, so fixed despite RARE likelihood.
- cluster_key: `sha1:314590b9651bbce5f4fd2de8e7f8f9a21ac51309`

## F2: Guide's close-year command block skips two gates

- Target: `docs/user/{fi,en}/tasks/year-end.md`
- Claim: The user guide's command block jumps from ApproveAdjustments to ApproveStatements; ApproveTaseErittelyt and ApproveLiitetiedot must be answered in between, otherwise GATE_NOT_OPEN.
- Fix: List all four answer commands in both languages.
- Evidence: close_year.rs answer_close_year opens ApproveTaseErittelyt after ApproveAdjustments and ApproveStatements only after ApproveLiitetiedot; verifier confirmed.
- cluster_key: `sha1:537c0d69ce4581dcbe82d2d70f439717c47b459f`

No finding was dropped, spun off or left for discussion. No issue was filed from this review: both findings are fixed in this change.
