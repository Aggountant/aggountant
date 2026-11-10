---
created: 2026-11-10
updated: 2026-11-10
type: improvement
status: open
priority: normal
lane: bank-import
lane_seq: 30
collision: [crates/import, crates/cli/src/main.rs]
---

# camt import: report empty statements and stop naming camt.052 as camt.053

## Problem

Two problems in the camt import contract:

1. **Empty report succeeds silently.** A camt.052 or camt.053 report that
   carries balances but no entries imports as a success with zero transactions
   and no explanation, so a wrong or truncated export looks like a quiet month.
2. **Names say camt.053 only.** The parser accepts both camt.052 (interim
   report) and camt.053 (statement), but the module is `camt053.rs`, the
   function `parse_camt053`, the error code `INVALID_CAMT053` and the CLI verb
   `import camt053`. A malformed camt.052 file reports a camt.053 error.

Both touch the same public `ImportError` contract and CLI verb, so they are
done together.

## Expected

- An empty report returns an explicit result (or error) that says the report
  had no entries, with the report's period and account.
- Rename to `camt.rs` / `parse_camt` / `ImportError::Camt` / `INVALID_CAMT`, with
  `import camt` as the verb. Whether `import camt053` stays as an alias is
  decided in the plan; the user guide changes in the same commit.

Not in scope: copy-report (`CpyDplctInd`), `INFO`-status entries, reference-only
`NtryRef` entries and multi-currency `Rpt` blocks. No export we have seen
produces them; reopen when one does.

## Acceptance criteria

- [ ] A synthetic balance-only report yields an explicit empty-report result.
- [ ] Error codes and names no longer claim camt.053 for camt.052 input.
- [ ] `docs/user/` updated with the verb.
