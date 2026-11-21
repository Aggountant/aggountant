# Review: test-flakes-stdin

Covers both issues in lane `test-flakes`
(`employer-report-test-stdin-flake`, `company-knowledge-cli-broken-pipe-flake`).

- Skill: `/llm-review`, rigor **routine**: test-only change, no product code,
  no bookkeeping or filing path touched.
- Scope: commit `a3b96af` (22 files under `crates/cli/tests/`, new
  `crates/cli/tests/common/mod.rs`).
- Finder: `gpt-6-astra`, one call with the diff, the helper and
  `exit_codes.rs`; asked for tests that could now pass wrongly, still flake,
  deadlock, lose assertion strength, or a missed site. Answer: "No findings."
- Verifier: not called (no surviving candidates, surface not high-risk).
  One model call in total.

## Findings

None confirmed.

## Local evidence

- Missed sites: after the change, no `write_all` to a child's stdin remains in
  any crate's `tests/`; the remaining `write_all` calls in
  `crates/broker/tests` and `crates/vero-vat-api-mock/tests` write TCP
  streams, not child stdin. No `Stdio::piped()` exists in tests outside
  `crates/cli/tests`.
- Assertion strength: every helper still asserts on the child's exit code and
  output after `wait_with_output`; only `BrokenPipe` is tolerated, any other
  write error still panics.
- The new test `stdin_fed_to_a_command_that_already_exited_is_not_a_test_failure`
  fails with the issue's exact panic (`Os { code: 32, kind: BrokenPipe }`)
  when the old unwrapped write is substituted, and passes with the helper.
- `scripts/ci-local.sh` passed (fmt, clippy, deny, stable and MSRV tests).

## Residual risk

A helper that feeds input larger than the pipe buffer to a child that writes
a lot of output before reading stdin could deadlock; that shape predates this
change and no current test feeds input of that size.
