---
created: 2026-11-23
updated: 2026-11-23
type: bug
status: open
priority: normal
---

# cargo-deny: url 2.5.8 hits RUSTSEC-2026-0592

## Problem

The `cargo-deny` job fails on every CI run on `main` since 23.11.2026 (e.g. runs 45194119778 and 45196192116):

```
error[vulnerability]: Stack overflow in `Url::origin()` on nested `blob:` URLs
    ├ ID: RUSTSEC-2026-0592
    ├ Advisory: https://rustsec.org/advisories/RUSTSEC-2026-0592
```

## Cause

`Cargo.lock` pins `url` 2.5.8. The advisory affects `url` ≤ 2.5.8; 2.5.9 is patched.

## Fix

`cargo update -p url` (to ≥ 2.5.9), confirm `cargo deny check` passes, commit the lockfile.
