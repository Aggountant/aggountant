---
created: 2026-11-10
updated: 2026-11-10
type: bug
status: open
priority: high
lane: cli-canon
lane_seq: 10
collision: [crates/cli/src/main.rs]
---

# Read-only commands create a stray empty home

## Problem

A read-only command creates a new, empty company home when home resolution
lands on a directory that is not a home.

```sh
cd "$(mktemp -d)"
aggountant --json employee list     # fails with MASTERDATA_NOT_INITIALIZED
find .                              # ./accounting/fiscal.sqlite and ./accounting/artifacts now exist
```

Run from inside a home directory, the `./accounting` fallback creates a stray
`accounting/accounting` home. The next `run show --run-id …` then answers
`RUN_NOT_FOUND` because it reads the empty stray home, which is easy to mistake
for a lost run.

Some read paths already use `open_existing_ledger` (`crates/cli/src/main.rs`);
many still call `Ledger::open`, which creates the store.

## Expected

Only explicit bootstrap (`init`) creates a home. Every other command fails
without writing anything when the resolved path holds no initialized home. The
error names the resolved path and how it was chosen (flag, environment,
configuration, marker, fallback), so a wrong working directory is obvious.

## Acceptance criteria

- [ ] Read commands (`employee list/show`, `run show/list`, `period show`, `voucher verify/show`, `file-vat preview`, `report …`) never create `fiscal.sqlite`, `artifacts/` or other files in a non-home directory.
- [ ] Their error states the resolved home path and its resolution source.
- [ ] A regression test runs a read command in an empty temporary directory and asserts the directory stays empty.
