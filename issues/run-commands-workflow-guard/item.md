---
created: 2026-11-10
updated: 2026-11-27
type: bug
status: fixed
priority: normal
lane: run-workflow-guard
collision: [crates/process/src/lib.rs]
closed: 2026-11-27
---

# Generic run commands must reject a Run of the wrong workflow

## Problem

`ProcessEngine::answer` and `ProcessEngine::resume`
(`crates/process/src/lib.rs`) implement close-period behaviour but do not check
that the folded Run's workflow is `close-period`. `check_version` accepts other
known workflows, so a non-close Run passes version validation and reaches
period-close reconciliation. An unknown or retired workflow falls back to the
close-period workflow version, and migration can then assign it that version.

## Impact

Passing a capture-receipt, file-vat, payroll or close-year Run id to the
generic `aggountant run answer` / `run resume` by mistake can make resume call
`ledger.begin_close` for that Run's period: an authoritative fiscal side effect
under the wrong Run identity.

## Expected

`answer` and `resume` fail closed unless the Run's workflow is `close-period`.
An unknown workflow's version lookup or migration returns an actionable error
instead of falling back.

## Acceptance criteria

- [x] `run answer` / `run resume` with a real non-close Run id fail without side effects.
- [x] A Run with an unknown workflow fails with an actionable error.
- [x] Tests fail if either guard is removed.

## Resolution

### 2026-11-27T18:06:35Z · @issuectl

Every Run-continuing command checks the Run's workflow in check_version; see result.md. Also covers run-resume-refuses-other-workflows (left for triage).
