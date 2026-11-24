# Review: camt-empty-report-and-rename

- Skill: `/llm-review`, rigor **routine**. The empty-report change adds
  output and refuses nothing; the rename keeps the bank event identity
  (`bank-import:holvi:<iban>:<ref>`) and the store schema untouched, so no
  sealed data or authority filing is at stake.
- Scope: the slice's first commit against `f5c72a9` (`crates/import/src/camt.rs`,
  `error.rs`, `lib.rs`, `crates/cli/src/bank_import.rs`, `main.rs`,
  `input_help.rs`, tests, guide, skills, CHANGELOG).
- Finder: `gpt-6-astra`, one call with the diff. Verifier: none. The one
  candidate was confirmed by reading the code and is now pinned by a
  regression test, so a second model could not change the outcome. One model
  call in total.
- Risk class: import output contract (what an agent sees before it approves
  a statement with no entries).

## F1 (should-fix): pending entries without a reference were not counted

- Location: `crates/import/src/camt.rs`, `parse_camt`, the entry loop and
  `EmptyReport::new(…, unbooked.len())`.
- Claim: `empty_report.unbooked_count` took `unbooked.len()`, but `unbooked`
  keeps only non-booked entries with a usable reference
  (`ParsedEntry::Unbooked(None)` is dropped). A pending-only report whose
  entries have no reference showed `unbooked_count: 0` and no pending note.
- Evidence: local read of `parse_entry` and the loop; the test
  `an_unbooked_entry_is_listed_unless_the_document_also_books_it` parses one `PDNG` entry without
  `AcctSvcrRef` and now asserts `unbooked_count == 1`.
- Impact: the agent would lose the hint that the account had activity
  awaiting booking, and could more easily read the report as a quiet month.
- Fix: count both `Unbooked` variants while collecting entries; `unbooked`
  itself is unchanged.

## Dropped candidates

None. The finder proposed one candidate.

## Coverage

Read: the whole diff, `parse_entry`, `summarize_report`, the seal and preview
paths in `bank_import.rs`, the clap tree and the human renderer. Tested:
balance-only camt.052 and camt.053 through parse, dry-run and seal; the
human line; the `camt053` and `camt052` aliases (parse output identical,
seal works); `INVALID_CAMT` for a malformed camt.052 with no `053` in the
message; pending-only reports with and without references.

Outside scope: what the ledger records for a statement sealed with no
entries beyond the retained artifact (unchanged by this slice).
