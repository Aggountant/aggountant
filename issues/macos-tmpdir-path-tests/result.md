# Result

The CLI still reports the canonical home path, because that is the path it
actually opens. `crates/cli/tests/home_required_cli.rs` now builds the expected
`accounting` and `.aggountant` paths from `std::fs::canonicalize` of the
tempdir, so the expectations are canonical on every platform.

Verification on Linux: with `TMPDIR` pointing at a symlink to a real
directory, the old tests fail with the same two panics CI shows on macOS
(lines 173 and 270), and the fixed tests pass. `scripts/ci-local.sh` passes.
