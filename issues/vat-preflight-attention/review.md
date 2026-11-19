# Review: vat-preflight-attention

- Skill: `/llm-review`, rigor **routine**. The change touches only what
  happens when `file-vat start` fails before anything is sent: no submission,
  outbox or filing-authority path changes, so it is not a critical boundary.
- Scope: the working-tree diff against `0b6f035`
  (`crates/process/src/file_vat.rs`, `crates/process/tests/file_vat.rs`,
  `skills/vero-fi/SKILL.md`, `docs/user/{en,fi}/tasks/file-vat-direct.md`),
  with `file_vat/recovery.rs` and `file_vat/status.rs` attached so the
  unresolved-filing guard could be checked.
- Finder: `gpt-6-astra`, two calls on one thread: the preflight change, then
  the projection wrap added afterwards. Verifier: not called. The finder found
  nothing, and the surface is not high-risk.
- Risk class: Run state after a refused start (no remote write).

## Findings

None confirmed. The finder answered "No findings." both times.

## Local checks

- A rejected `reconcile` effect makes the Run `NeedsAttention`
  (`Fold::status`, `crates/process/src/lib.rs`). Tests
  `a_failed_period_read_parks_the_run_needs_attention` (transport timeout on
  `GetVATPeriods`) and
  `a_period_the_authority_will_not_accept_parks_the_run_needs_attention`
  (remote status `Filed`) assert this through `show`. Both fail on the
  pre-change code.
- `unresolved_filing` reads only the `submit-filing` effect, so a Run parked
  here does not block a new Run for the period.
  `failed_remote_period_preflight_does_not_claim_local_filing_authority`
  still passes.
- `resume_file_vat` on such a Run fails while reading its missing state file
  and sends nothing; `retry` requires a rejected `submit-filing`.
- `scripts/ci-local.sh` passed: fmt, clippy, deny, and stable and MSRV tests.

## Dropped candidates

None proposed.

## Residual risk (outside scope)

- `write_file_vat_state` runs after `acquire_filing_authority`. If that local
  write fails, the Run keeps the period's filing authority and `reconcile`
  stays `executing`. This is a local disk failure, not a remote read, so it
  is not part of this issue and no issue was filed.
- The projection wrap has no dedicated test. The adapter's
  `VERO_VAT_CROSS_FIELD` refusal could not be reached from a synthetic sealed
  ledger without contriving the field set.
