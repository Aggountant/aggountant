# Review: file-vat-retire-unfiled-run

- Skill: `/llm-review`, rigor **critical**: the change decides when a
  `file-vat` Run counts as finished with nothing sent. Calling a Run that may
  have sent a return `not_filed`, or letting it stop blocking a new filing,
  could lead to a second, irreversible VAT return.
- Scope: commit `3f5bc41` against `main` (`crates/process/src/file_vat.rs`,
  `crates/process/src/lib.rs`, `crates/cli/src/main.rs` help text, tests,
  guide fi/en, `skills/vero-fi/SKILL.md`, CHANGELOG).
- Finder: `gpt-6-astra`, one call (the diff plus `file_vat.rs`, `lib.rs`,
  `file_vat/recovery.rs`, `file_vat/status.rs` and the issue). The finder
  came back clean, so the verifier asked one focused false-negative question
  instead: `claude-fable-5-2`, one call, with the same files plus
  `authority.rs`. Two model calls in total.
- Risk class: authority filing state (duplicate send).

## Findings

None confirmed.

The finder returned "No findings" after checking dispatch ordering, the
status and recovery guards, whether the error codes are the same as before,
and the view enrichment.

The verifier was asked whether any reachable path lets a Run that meets the
`not_filed` / settle conditions have sent a FileVATReturn, or lets the
unresolved-filing guard or `file-vat status` wrongly allow a new filing. It
answered "No findings" and cited the evidence:

- The only filing `broker.submit` is in `run_submit_filing`. It runs after
  `assert_filing_gate_approved_and_bound`, which needs an approved
  `ApproveFiling`, and after a durable `EffectExecuting{submit-filing}`. A
  dispatched filing therefore always leaves a gate and a non-planned
  `submit-filing` in the journal, and either one makes `filing_never_began`
  false. No event removes a gate or returns an effect to Planned.
- No other path sends a filing. `resume_file_vat` uses
  `run_submit_filing` behind the approved gate. `reconcile_file_vat` only
  fetches. `retry_file_vat_submit` refuses `submit-filing`, and
  `complete-manual` makes no broker call.
- The oldest public revision (`cfca9c6`) has the same order: gate guard,
  `EffectExecuting`, then submit. The verifier checked that endpoint, not
  every revision in between.
- `unresolved_filing` and `file_vat_filing_runs` read only the
  `submit-filing` effect, never `RunStatus` or `reconcile`. A Run stopped
  before filing was already skipped there, so nothing that blocked before
  is released, and other Runs for the period are still each evaluated.
- The CLI's portal verification submit belongs to no Run and uses a
  `verify-portal:` outbox key, so it does not bear on the conditions.

## Local evidence (host)

- Tests: legacy fixtures in `crates/process/src/file_vat/tests.rs` write
  the journal shapes older builds left in a home of the current store
  version (v9; no migration). They cover reconcile executing and reconcile
  rejected, each for a first return and for a replacement, plus two positive
  controls (filing executing; gate opened) where resume refuses and writes
  nothing. `tests/file_vat.rs` checks a refused start (`not_filed`, the
  `error_code` on the view, `file-vat status` = `not_filed`, the corrected
  retry reaching `ApproveFiling`). The CLI test checks `run show` and
  `run resume` JSON.
- `scripts/ci-local.sh` passed (fmt, clippy, deny, stable and MSRV build
  and test, guide gate).

## Dropped candidates

None. Neither model proposed a candidate.

## Coverage and residual risk

- No test injects an error into the reconcile `?` paths that used to skip
  the journal (`Ledger::open`, the state write). They cannot be reached
  without a test seam. The guarantee is structural: the whole stretch from
  `EffectExecuting` to the outcome is one closure whose only error exit
  journals the rejection.
- Outside the brief's scope and noted rather than fixed: `run resume` on
  other workflows' Runs (filed as `run-resume-refuses-other-workflows`),
  and the matching close-period shape (a start refused at `begin_close`
  stays `needs_attention`).
