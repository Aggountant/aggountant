---
created: 2026-11-24
updated: 2026-11-24
type: bug
status: untriaged
priority: normal
provenance: other
provenance_detail: observed while implementing file-vat-retire-unfiled-run
source_ref: taskfleet:01m87655v08c65sk89wyxc8ggp/observation:run-resume-other-workflows
originating_run: 01m87655v08c65sk89wyxc8ggp
originating_run_kind: spinoff
---

# run resume runs month-close recovery on other workflows' Runs and can move an open month to closing

## Description

## Problem

`aggountant run resume --run-id <id>` runs the month-close recovery
(`ProcessEngine::resume`, `crates/process/src/lib.rs`) on any Run id it is
given. Only a `close-period` Run should reach it. `file-vat` Runs are now
routed away (issue `file-vat-retire-unfiled-run`); `file-wage` and other
workflows whose version is not mapped in `expected_version` are refused by
the version check. But `run-payroll`, `capture-receipt`, `hunt-receipts` and
`close-year` each have their own version in `expected_version`, so for them
the check passes and the close-period steps run.

If the Run's period is `open` (normal for a `capture-receipt` or
`hunt-receipts` Run in the current month), the "crash before the fiscal CAS"
branch passes the obligation gate and the content check, then calls
`ledger.begin_close(period, <that run id>)`. That moves the month to
`closing`, owned by a Run that will never seal it. Then it journals
`EffectSucceeded { effect_key: "transition" }` into a journal that never
planned that effect, so every later fold of that Run fails with
`JOURNAL_CORRUPT` ("state for unplanned effect transition").

Found by reading the code while fixing `file-vat-retire-unfiled-run`; not
reproduced against a real home. Shape with made-up data: Esimerkki Oy,
month 2026-03 open, a `capture-receipt` Run `cr-…` for 2026-03, then
`aggountant run resume --run-id cr-…`.

## Expected

`run resume` refuses (for example `INVALID_ANSWER` naming the workflow's own
resume command) for any Run that is not `close-period` or `file-vat`, before
it reads or writes fiscal state.

## Acceptance

- [ ] `run resume` on a `capture-receipt`, `hunt-receipts`, `run-payroll` or
      `close-year` Run changes neither the period nor the Run's journal.
- [ ] A test that would have moved an open month to `closing`.
