# Review of vat-filing-resent-by-new-run-after-unknown

Focused `/llm-review` of the implementation diff against `main` (`e82307d`),
taken after the harvested work and the completion commit (`3fd1e7c`).

- Finder: `gpt-6-astra` (`--task review`). Attached: the diff, `recovery.rs`,
  `file_vat.rs`, `status.rs`, `bank_days.rs`, `broker.rs` and the issue. One call.
- Verifier: `claude-fable-5-2`, one call. The finder's one candidate is a
  deliberate scope boundary (below), so no actionable candidate survived. The
  surface is high-risk, so the verifier got one focused false-negative question:
  is there any path to `Broker::submit` for FileVATReturn while an unknown is
  unresolved, or any close or adoption on weaker evidence than the rule?
- Rigor: critical (an irreversible submission to an authority). Risk class:
  authority filing / data integrity.

## Finder

### A1 (must-fix as raised): an interrupted filing has no reconcile exit

`crates/process/src/file_vat/recovery.rs`, `unresolved_filing` and
`reconcile_file_vat_at`. A Run whose filing effect is `Executing` (or
`Succeeded` without `vat_filed`) blocks every other Run, but `file-vat reconcile`
accepts only a `Rejected` filing, so neither "found" nor two "not found" answers
can resolve it.

Local check: true as described. This is the boundary the brief drew: the next
issue in the lane, `perfectly-uppity-knife`, is the recovery for an interrupted
Run, and its acceptance criteria require "read fails → a new Run is still
refused". Until it lands, `file-vat complete-manual --remote-status-evidence-ref`
is the exit, as `file-vat status` already advised ("do not file again until the
Run has been resumed"). The recording and closing helpers in `recovery.rs` take
no position on why a filing is unresolved, so that issue can reuse them.

## Verifier

"No path found" to a second FileVATReturn while an unknown is unresolved, and no
path by which a Run closed as not filed dispatches. Checked: the only filing
submit (`run_submit_filing`), `start_file_vat`, the `ApproveFiling` answer,
`retry_file_vat_submit`, `resume_file_vat` (including the closed-Run guard), the
generic `run answer`, answers recorded before the unknown or by another Run,
failed reads, `finnish_date`, rejection-code classification and fail-closed
reads of corrupt journals. Two candidates:

### V1 (must-fix): two "not found" answers can be minutes apart

`crates/process/src/file_vat/recovery.rs`, `not_found_streak`, with
`crates/domain/src/bank_days.rs`, `a_bank_day_has_passed`. The gap was measured
on Finnish calendar dates only: "Missing" at 23:59 Monday and again at 00:00
Tuesday closed the Run, minutes after the send, and a new Run could then send
again. That meets the rule's wording ("the second on a later bank day") but not
its purpose: "not found" right after sending proves nothing because visibility
lags.

Local check: confirmed by the existing tests themselves (`2026-11-16T21:59:59Z`
is the 16th, `22:00:00Z` the 17th, and 16th → 17th counted).

### V2 (should-fix, pre-existing): adoption without a `Status`

`crates/broker/src/vero_vat_api.rs`, `FiledVatReturn::semantically_matches_attempt`.
The check refused only a status that was present and not `Processed`; a body
with no `Status` passed, and a no-activity return has no amounts to compare, so
a submission date alone could adopt the filing and mark the period filed.

Local check: confirmed by reading; whether the authority ever omits `Status`
cannot be verified from the repository.

## Dropped

None. One finder candidate, two verifier candidates; all three confirmed.

## Coverage and residual risk

Read: the whole diff, the dispatch, answer, retry, resume and reconcile paths,
the broker's submit idempotency and the status plan. Tested: the acceptance
tests in `crates/process/tests/file_vat.rs`, the CLI test in
`crates/cli/tests/file_vat_status_cli.rs`, and the calendar and streak unit
tests. Outside scope: recovery of an interrupted Run (`perfectly-uppity-knife`),
and whether a new Run whose request differs from the earlier attempt should be
allowed at all (it is refused, see the decisions in `result.md`).
