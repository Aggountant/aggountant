# Result: a rejected close returns the period to Open

## What changed

- **Ledger.** New legal period edge `Closing→Open` (`is_legal_period_edge`,
  so `verify_periods` accepts it) and `Ledger::reject_close(period, authority,
  actor)`. It takes the `ClosingAuthority`, checks the token is for this period,
  and runs the same CAS as `seal_period` with `match_owner`: only the run that
  owns the Closing period can take it, a stale or foreign token is refused
  `VOUCHER_NOT_IN_RUN` and the refusal is written to the rejection log. The
  edge is appended to the hash-chained `period_transition` log with that run as
  `owning_run_id`, so after a rejection the Open row still names the rejecting
  run (as it already does after a `Sealed→Open` reopen).
- **Process.** Answering `ApproveClose` with `reject` records `GateAnswered`
  and then runs a new effect `reject-close` (type `RejectClose`), planned at
  that moment. `run resume` finishes it when the gate is answered `reject` and
  the effect has not succeeded: if fiscal still shows the period Closing under
  this run it takes the edge, otherwise it reconciles to fiscal (the edge had
  committed before the crash). The Run's status stays `rejected`.
- **Resume retry.** The retry for a close that crashed before its
  `Open→Closing` commit no longer requires an Open period without an owner
  (after a rejection the row names the rejecting run), and it passes the same
  open-obligation gate as a fresh start.
- **CLI.** No output shape changes; `run answer`/`run show` list the extra
  effect.
- **Guide (fi + en).** The "never reject a close" warnings and the "Rejected
  close" limitation are removed; `concepts/periods`, `tasks/close-month`,
  `start/practice-month`, `troubleshooting` (the "stuck in closing" entry now
  covers an awaiting gate and a run rejected by an older binary or interrupted
  mid-rejection, both fixed by answering or `run resume`) and the
  `PERIOD_TRANSITION_CONFLICT` row say what a rejection does.

## What a close-period Run writes before `ApproveClose`

Read from `crates/process/src/lib.rs` (`start_close_period_seam`,
`open_close_gate`, `snapshot_hash`) and every caller of `ClosingAuthority`:

| Written | Where | After the rejection |
|---|---|---|
| `Open→Closing` transition | fiscal `period_transition` + `period` row | stays in the chain; followed by `Closing→Open` |
| `RunStarted`, `EffectPlanned/Executing/Succeeded` for `transition`, `EffectPlanned` for `seal` | the Run's journal | kept; `seal` stays `planned` on a rejected Run, as before |
| `GateOpened(ApproveClose)` with a snapshot hash of the period row and chart version | the Run's journal | kept; the hash binds only this Run's approval, which can no longer be given. A new close computes its own |
| vouchers sealed with the Run's `ClosingAuthority` | fiscal vouchers | the close-period Run seals none. The library path `seal_with_authority` exists, but no production code calls it with an authority. Should one be added, its vouchers are ordinary sealed vouchers of the Open period afterwards; nothing is discarded (covered by `owner_rejects_a_close_back_to_open_and_the_month_closes_again`) |

The close-period Run computes no VAT figures and writes no report snapshots or
obligations. VAT figures are computed later from a Sealed period by
`file-vat`, so nothing VAT-related depends on a Closing period. Nothing needed
an explicit refusal.

## Compatibility (changelog-facing)

A data home in which a close has been rejected contains the new
`Closing→Open` period transition. **An older aggountant binary does not
verify such a home**: `voucher verify` reports `HASH_CHAIN_BROKEN` (illegal
edge `Closing→Open`). Upgrade every binary that opens the home. Homes in which
no close was rejected are unaffected.

A home stranded by an older binary (period `closing`, close run `rejected`)
is repaired with `aggountant run resume --run-id cp-…`, which takes the edge.

`CHANGELOG.md` was not edited: its `[Unreleased]` section is the first
public release, and this fixes a bug that release would otherwise ship.

## Tests

- `crates/ledger/tests/period.rs`: owner rejects → voucher sealed while
  Closing kept → ordinary seal → new close → Sealed → `verify().ok`; a stale
  token (after another run took the period) and another period's token are
  refused `VOUCHER_NOT_IN_RUN` with a rejection logged; a Sealed period cannot
  take `Closing→Open`.
- `crates/process/tests/close_period.rs`: reject returns the period Open and
  the gate cannot be answered again; reject → record → close → seal with full
  home verification, and resuming the old rejected Run leaves the new seal
  alone; crash after `GateAnswered(reject)` before the edge, recovered by
  `resume` and idempotent on a second resume; edge committed but not
  journalled, reconciled by `resume` without a second transition.
- `crates/cli/tests/close_period_cli.rs`: the issue's reproduction through the
  binary (reject → `period show` open → `voucher seal` → new close → approve
  → `voucher verify` ok).

- Review fixes (see `review.md`, `assessment.md`):
  `a_new_close_after_a_rejection_resumes_a_crash_before_its_fiscal_commit`
  covers a close started after a rejection that crashes before its
  `Open→Closing` commit: `resume` retries even though the Open row names the
  rejecting run, and refuses with `OPEN_OBLIGATION` while an obligation is open.

`scripts/ci-local.sh` and `scripts/check-user-guide.sh` pass.

## Simplicity

Reuses `transition_period`'s CAS, owner fence and rejection logging unchanged;
`reject_close` is the same shape as `seal_period`. The process side reuses the
effect/journal vocabulary (`EffectPlanned`, `EffectReconciled`) rather than a
new event. No separate command, no new error code. Left out: marking the
never-run `seal` effect `skipped` on a rejected Run (the status is already
`rejected` and the change would only touch presentation).

## Decisions for later review

- **Workflow version unchanged.** `CLOSE_PERIOD_WORKFLOW` (whose hash pins a
  Run's version) was not edited. Bumping it would make every Run parked at
  `ApproveClose` fail with `WORKFLOW_VERSION_MISMATCH` until `run migrate
  --confirm`, and would put the repair of stranded rejected Runs behind a
  migration, while an old journal is fully valid under the new code (the
  reject effect is planned when needed). Alternative: bump to `v3` and accept
  the migration step.
- **Reconcile rule.** When a rejected Run's `reject-close` is not recorded and
  fiscal no longer shows the period Closing under that Run, the effect is
  reconciled instead of retried. Reasoning: once `ApproveClose` exists the
  period was Closing under this Run, only the owner can move a Closing period,
  and this Run's gate can no longer approve a seal, so the only way out was
  this Run's own `Closing→Open`. Alternative: read the transition log for the
  exact edge; not done because it adds a Ledger read surface for a case the
  owner fence already rules out.
- **`require_unfiled` on the edge.** Set, although a Closing period cannot be
  filed; it costs nothing and keeps the edge from ever touching a filed
  period.
