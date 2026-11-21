# Result: employer-report-test-stdin-flake

Fixed together with `company-knowledge-cli-broken-pipe-flake` in one change
(lane `test-flakes`); test code only.

Every CLI integration test that feeds stdin to the `aggountant` binary now
writes it through `crates/cli/tests/common/mod.rs::feed_stdin`, which ignores
`BrokenPipe` (the child refused and exited before reading) and panics on any
other write error. That covers the `employer_report_history_cli.rs` helper and
all 24 other sites with the same unwrapped write in 22 test files.

`exit_codes.rs` gains
`stdin_fed_to_a_command_that_already_exited_is_not_a_test_failure`, which
holds stdin open across the child's exit and then writes 1 MiB, so it hits the
closed pipe every time; with the old unwrapped write it fails with the
issue's panic.

Review and assessment: `review.md`, `assessment.md` in this directory.
`scripts/ci-local.sh` passed.
