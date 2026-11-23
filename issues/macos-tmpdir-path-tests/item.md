---
created: 2026-11-23
updated: 2026-11-23
type: bug
status: open
priority: normal
lane: unlaned
---

# macOS CI: home_required_cli tests fail on /private/var symlink

## Problem

CI on `main` has been red since 22.11.2026 07:29 UTC (run 44998457583, commit 9224481) on both macOS test jobs (`test (macos-latest, stable)` and `test (macos-latest, 1.89.0)`). Linux jobs pass. Latest real failure: run 45194119778 (commit 8770f2e, 23.11.2026).

```
---- a_bare_marker_is_named_and_left_alone stdout ----
thread 'a_bare_marker_is_named_and_left_alone' panicked at crates/cli/tests/home_required_cli.rs:270:5:
assertion `left == right` failed
  left: String("/private/var/folders/q2/.../T/.tmpbdPrgZ/.aggountant")
 right: "/var/folders/q2/.../T/.tmpbdPrgZ/.aggountant"

---- commands_in_an_empty_directory_leave_it_empty_and_name_the_fallback stdout ----
thread '...' panicked at crates/cli/tests/home_required_cli.rs:173:9:
assertion `left == right` failed: ["employee", "list"]
  left: String("/private/var/folders/q2/.../T/.tmp8YktdS/accounting")
 right: "/var/folders/q2/.../T/.tmp8YktdS/accounting"
```

## Cause

On macOS `$TMPDIR` lives under `/var/folders/…`, and `/var` is a symlink to `/private/var`. The CLI reports the canonicalized home path, while the tests build their expectation with `std::path::absolute(tempdir.path()…)`, which does not resolve symlinks. The tests came with b22ab4e ("only init creates a company home; other commands refuse a path without one").

## Fix

In `crates/cli/tests/home_required_cli.rs`, build expected paths from a canonicalized tempdir (`std::fs::canonicalize(cwd.path())` before joining `accounting` / `.aggountant`), or compare canonicalized forms. Decide whether the CLI should report the canonical path at all; if the user-facing message should echo the path as given, change the CLI instead of the test.
