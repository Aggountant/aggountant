# Assessment: rejected-close-reopens-period review

Source review: `history/review-rejected-close-reopens-period.md` (copied beside the issue as `review.md`). Reviewed head: `a758514`.

| id | title | confirmation | likelihood | recommendation | applied |
|---|---|---|---|---|---|
| F1 | A new close after a rejection cannot resume a crash before its fiscal commit | CONFIRMED | RARE | FIX | yes |
| F2 | The relaxed retry skipped the open-obligation gate | CONFIRMED | RARE | FIX | yes |

## F1: A new close after a rejection cannot resume a crash before its fiscal commit

- Target: `crates/process/src/lib.rs` (`resume`)
- Claim: the pre-commit retry required an Open period with no owner; after a rejection the Open row keeps the rejecting run as owner, so the next close Run wedges at `running` after a crash between `EffectExecuting` and `begin_close`.
- Fix: retry whenever the period is Open; the owner of an Open period is history and `begin_close` does not match it. New test seam `Seam::CrashBeforeFiscalCommit`.
- Evidence: regression test fails without the fix (`Running` vs `AwaitingGate`), passes with it; verifier confirmed. Rejections are now an ordinary path, so the window is no longer exotic.
- cluster_key: `sha1:37d840d9cc5952b97eeffe1582ba06a27b039067`

## F2: The relaxed retry skipped the open-obligation gate

- Target: `crates/process/src/lib.rs` (`resume`, `start_close_period_seam`)
- Claim: the retry is a start but did not apply the `OPEN_OBLIGATION` check, so a period could be closed and sealed past an open obligation.
- Fix: shared `refuse_open_obligations` called by start and by the retry; a refusal leaves the Run retryable and journals nothing.
- Evidence: verifier reproduced the sequence; regression test asserts `OPEN_OBLIGATION` with the period still Open. Rule violation of `crates/AGENTS.md`, so fixed despite RARE likelihood.
- cluster_key: `sha1:28ce82ea8ca13ffb67375a1a9a9077b51635f4d7`

No finding was dropped, spun off or left for discussion. No issue was filed from this review: both findings are fixed in this change. The noted uncertainty (an obligation created while a period is already Closing is not re-checked at approve) predates this change and is not reachable in this version, since obligations come only from the consult route, which is not usable yet; not filed.
