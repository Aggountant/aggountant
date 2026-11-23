---
created: 2026-11-23
updated: 2026-11-23
type: bug
reporter: jari
status: untriaged
priority: normal
provenance: agent:homebase-sihteeri
source_ref: agent:homebase-sihteeri/reporter:jari/id:sihteeri-agg-explain-20261123
---

# CI: explain_cli test pins the old ALREADY_INITIALIZED wording

## Description

CI: explain_cli test pins the old ALREADY_INITIALIZED wording

## Problem

Since 192c8b7 ("docs/user: error-codes cells say who does what …", 23.11.2026) the Linux test jobs fail on `main` as well (run on cb46fad):

```
---- error_json_carries_the_guides_hint stdout ----
thread 'error_json_carries_the_guides_hint' panicked at crates/cli/tests/explain_cli.rs:32:5:
assertion `left == right` failed
  left: String("ei mitään, koska koti on jo olemassa eikä siihen koskettu")
 right: "ei mitään; koti on jo olemassa eikä siihen koskettu"
```

## Cause

192c8b7 reworded the `ALREADY_INITIALIZED` "what is done" cell in the user guide's error-code table, which `aggountant explain` and the error JSON `hint` print. `crates/cli/tests/explain_cli.rs:34` still expects the old text.

## Fix

Update the expectation in `crates/cli/tests/explain_cli.rs:34` to the new wording (or assert against the table row read at test time so prose edits do not break the test). Docs-only commits under `docs/user/` that touch the error-code table need `cargo test -p aggountant --test explain_cli`.

Related open bugs on the same red main: `@macos-tmpdir-path-tests`, `@url-rustsec-2026-0592`.

Reported by jari via agent:homebase-sihteeri.
