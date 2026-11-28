# Review: camt-creditor-reference

- Skill: `/llm-review`, rigor **routine**. The change touches the bank-import
  parser whose output becomes sealed vouchers, but it adds no write path, no
  stored field and no store migration; the identity invariants were tested
  directly (see Coverage).
- Scope: the slice's working diff against `eb4a86b` (`crates/import/src/camt.rs`,
  `crates/import/src/lib.rs`, `crates/process/src/tax_payments.rs`,
  `crates/cli/src/bank_import.rs`, `crates/cli/src/main.rs`, tests and the
  new fixture, guide pages, the `capture-receipt` skill, CHANGELOG), with the
  post-change `camt.rs`, `bank_import.rs`, `payment_reference.rs`,
  `tax_payments.rs` and the issue text attached.
- Finder: `gpt-6-astra`, one call. Verifier: not called. The single candidate
  was confirmed against the code (it was the open question of the
  implementation) and fixed; a verifier verdict could not change the outcome.
  One model call in total.
- Risk class: bookkeeping source data (an entry's identity; evidence the agent
  proposes accounts from).

## F1 (must-fix): unreadable tax payment details blocked parse and preview

- Location: `crates/cli/src/bank_import.rs`, `parse_camt_file` and the
  `--dry-run` branch of `seal_camt`.
- Claim: both paths propagated `read_tax_payments` errors with `?`, so a
  hand-broken `masterdata/tax-payments.json` (`TAX_PAYMENT_DETAILS_CORRUPT`)
  made `import camt parse` and `import camt seal --dry-run` fail for any
  statement, including one without references, while the seal itself does
  not read the file.
- Evidence: local read; `read_tax_payments` refuses a file whose reference
  check digit fails or whose JSON is malformed.
- Impact: the preview-and-approve workflow of a bank import stops on unrelated
  optional master data; preview and seal would disagree about refusal.
- Fix: the comparison loads the details through `tax_details`, which turns a
  read error into a top-level `warnings` entry (`code`, `message`) and marks
  nothing. `unreadable_tax_payment_details_do_not_block_reading_the_statement`
  covers parse and preview.

## Dropped candidates

None; the finder returned one candidate.

## Coverage

Read and tested:

- `creditor_reference.rs`: golden output of a fixture with a domestic, an RF,
  a zero-padded, a wrong-check-digit and an `other` reference; the same file
  without `Strd` gives the same keys, fingerprints and descriptions; the
  `main` build's fingerprints and descriptions pinned.
- The existing `a_file_without_the_elements_parses_exactly_as_before` golden
  test is unchanged and passes.
- CLI: re-import over entries sealed without the reference is five duplicate
  observations; tax recognition with and without stored details (also that
  the preparation hash does not change and that a money-in entry carrying a
  stored reference is not marked); parse without a home.
- By hand: the fixture sealed with the `main` build (eb4a86b) into a fresh
  home, then previewed, re-imported and re-exported with this build:
  `duplicate_observation_count` 5, no new vouchers.

Outside scope: other banks' placement of the reference (for example in
`Ustrd` only) is not read; the issue asks for `Strd`.
