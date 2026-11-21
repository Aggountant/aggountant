---
created: 2026-11-21
updated: 2026-11-21
type: bug
status: untriaged
priority: normal
provenance: other
provenance_detail: observed in scripts/ci-local.sh while landing skill-month-rebuild
source_ref: taskfleet:01m7yfttyc2h090xfx2xtre7aa/gap:employer-report-test-stdin-flake
originating_run: 01m7yfttyc2h090xfx2xtre7aa
originating_run_kind: spinoff
---

# Flaky test: employer_report_history_cli writes to the stdin of a child that may already have exited

## Description

## Observed

`scripts/ci-local.sh` on 2026-11-21, during the test phase with the machine
under load, on a change that touched no Rust besides a description string in
`crates/cli/src/skill.rs`:

```
---- configure_writes_only_the_employer_report_block_from_flags_or_stdin stdout ----
thread '…' panicked at crates/cli/tests/employer_report_history_cli.rs:157:10:
called `Result::unwrap()` on an `Err` value: Os { code: 32, kind: BrokenPipe, message: "Broken pipe" }
test result: FAILED. 3 passed; 1 failed
```

Eight reruns of `cargo test -p aggountant --test employer_report_history_cli`
right after passed.

## Cause

The helper around line 141 spawns the binary with a piped stdin and
unconditionally `write_all(stdin).unwrap()`s to it. When the invocation takes
its input from flags, or fails validation before reading, the child exits
without reading stdin; if it exits before the parent writes, the write gets
`EPIPE`. The race is lost only under load, which is when the full CI runs.

## Impact

A red `ci-local` (and potentially a red `main` in GitHub Actions) on changes
that have nothing to do with the employer report. No effect on users.

## Direction

In the helper, ignore `BrokenPipe` from the stdin write (the assertion is on
the child's output and exit code anyway), or pipe stdin only for the calls that
pass input. Other test helpers with the same shape are worth the same look.
