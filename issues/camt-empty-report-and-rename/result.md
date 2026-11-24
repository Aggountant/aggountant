# Result: camt-empty-report-and-rename

## Empty statements

A camt.053 or camt.052 report with balances and no booked entries still
imports: a quiet month is previewed and sealed with `{"assignments":[]}`,
creates no vouchers and retains the statement file. What changed is that it
no longer looks like a plain success.

- `parse_camt` sets `ParsedStatement.empty_report` when the document has no
  booked entries: `account_iban`, `periods` (each block's `FrToDt`),
  `unbooked_count` (every non-booked entry, with or without a reference) and
  a `message` naming the account and the period. When a block's opening and
  closing balances differ, the message says the file is missing entries and
  not to import it as a quiet month.
- `import camt parse`, `seal --dry-run` and `seal` carry `empty_report`; the
  human output adds an `empty report: …` line.
- `--help` (`EMPTY REPORT`), the `capture-receipt` skill and the guide page
  "Import the bank statement" (new section "When the statement has no
  transactions", fi and en) tell the agent to confirm the account and period
  with the user before sealing an empty statement. The "empty statement is
  not mentioned" limitation is removed from the status page.

## Rename

- `crates/import/src/camt.rs`, `parse_camt`, `ImportError::Camt`,
  `INVALID_CAMT`, CLI verb `import camt`.
- `import camt053` and `import camt052` stay as visible aliases.
- `INVALID_CAMT053` is renamed without an alias (breaking, in the CHANGELOG
  `[Unreleased]`); exit code and `error_kind` are unchanged. The guide's
  error-code table gains an `INVALID_CAMT` row, so `explain INVALID_CAMT`
  and the error `hint` now have text.
- Bank event identity (`bank-import:holvi:<iban>:<ref>`), source
  fingerprints and the store schema are unchanged; no migration.

## Tests

- `crates/import/tests/parse.rs`: balance-only camt.052 and camt.053,
  balances that differ, no period, a report with entries has no
  `empty_report`, pending-only reports with and without references, and a
  malformed camt.052 that reports `INVALID_CAMT` without `053`.
- `crates/cli/tests/import_camt.rs`: a balance-only statement in both
  containers through parse, dry-run and an approved seal; the human line;
  the `camt053`/`camt052` aliases (identical parse output, seal works); the
  CLI error for a malformed camt.052.
- `crates/cli/tests/exit_codes.rs`: help succeeds on `import camt` and both
  aliases.

## Review

`/llm-review` (one finder, `gpt-6-astra`) plus `/assess-findings`, in
`review.md` and `assessment.md`. One finding, fixed.
