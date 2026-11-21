---
created: 2026-11-21
updated: 2026-11-21
type: bug
status: fixed
priority: normal
provenance: other
provenance_detail: observed in ci-local during a taskfleet spinoff
source_ref: taskfleet:01m7yfncx0mmfty92c32wzsm32/ci-flake:company-knowledge-broken-pipe
originating_run: 01m7yfncx0mmfty92c32wzsm32
originating_run_kind: spinoff
lane: test-flakes
lane_seq: 0
preparation: ready
closed: 2026-11-21
closed_by: claude-opus-5-5
commits:
- hash: a3b96af
  summary: Ignore BrokenPipe when CLI tests feed stdin to a child
---

# Flaky company_knowledge_cli test: stdin write hits Broken pipe when the CLI refuses early

## Description

`crates/cli/tests/company_knowledge_cli.rs` test
`update_and_retire_take_the_id_positionally_or_in_stdin_json` failed once in
`scripts/ci-local.sh` and passed on two reruns, with no change to the
knowledge code. In the next run
`crates/cli/tests/employer_report_history_cli.rs`
`configure_writes_only_the_employer_report_block_from_flags_or_stdin` failed
the same way (passed on four reruns); its helper has the same unwrapped stdin
write (line 156).

The panic is at the `invoke` helper's `write_all(stdin.as_bytes()).unwrap()`
(line 19): when a case expects the CLI to refuse before reading stdin (for
example an id given both positionally and in stdin JSON), the child can exit
first and the write sees `Broken pipe`. The test treats that as a failure.

Fix sketch: ignore `ErrorKind::BrokenPipe` from the stdin write in the test
helper (the exit code and stderr are what the test checks), as other CLI test
helpers do or should.

Impact: a red local CI run that blocks a push until rerun; no product impact.

## Resolution

### 2026-11-21T10:22:38Z · @claude-opus-5-5

Fixed with employer-report-test-stdin-flake; see result.md.
