# Review: camt-import-preview-and-balances

- Skill: `/llm-review`, rigor **routine** (the surface seals vouchers, so it
  was treated as high-risk for data integrity, but it is no authority filing,
  migration or destructive operation).
- Scope: commit `6f42e85` against `3fe66a0`
  (`crates/cli/src/bank_import.rs`, `crates/cli/src/main.rs`,
  `crates/cli/src/input_help.rs`, `crates/import/src/camt053.rs`,
  `crates/import/src/lib.rs`, `crates/ledger/src/lib.rs`,
  `crates/ledger/src/store/fs_artifact.rs`, tests, guide, skill).
- Finder: `gpt-6-astra`, one call (diff plus the issue, `crates/AGENTS.md`
  and `crates/ledger/AGENTS.md`). Verifier: not called. F1 was settled by a
  local regression test that failed before the fix; F2 is a judgement about
  intended behaviour, not a factual claim a verifier could refute. One model
  call in total.
- Risk class: sealing permanent vouchers (silently wrong bookkeeping).

## F1 (must-fix): reconciliation added different currencies as one

- Location: `crates/import/src/camt053.rs`, `summarize_report` (the
  `entries_net` sum and the `balances_reconcile` comparison).
- Claim: without `Acct/Ccy` the parser accepts entries in several currencies.
  The summary labelled the sum with the first entry's currency and added every
  amount, so a statement with a EUR debit and a USD credit could report
  `balances_reconcile: true` against EUR balances. The skill tells the agent
  that `true` means the file was read correctly.
- Evidence: local read confirmed `parse_entry` checks the entry currency only
  when the account currency is declared; the finder reproduced it on the
  fixture with `Acct/Ccy` removed and one entry in USD.
- Fix: `entries_net` is `None` unless every entry is in the summary currency,
  and `balances_reconcile` is then `false` when balances exist. Test
  `entries_in_several_currencies_never_reconcile`.

## F2 (should-fix, contested): a real seal's preflight refusal is not in the rejection log

- Location: `crates/cli/src/bank_import.rs`, `prepare` (the
  `validate_seal_with_pending_artifact` call, used by both the preview and the
  real seal).
- Claim: before this change a closed-period entry or a natural key reused with
  different content reached `ledger.seal`, which records the refusal in the
  hash-chained rejection log. Now the whole-batch preflight refuses first, and
  `validate_seal` writes no rejection record.
- Evidence: local read confirms the call path. It also shows the old flow
  sealed every entry before the refused one, leaving a partial batch, and that
  the old preflight (`Ledger::validate`) already refused unknown accounts and
  missing attachments without a rejection record.
- Assessed as not a defect; see `assessment.md`.

## Dropped

None beyond F2's assessment. The finder raised two candidates.

## Coverage

Read: the whole diff, `Ledger::validate_seal` and `commit`'s order of checks,
`capture_receipt.rs`'s preparation hash (the pattern followed), the import
crate's projection path. Tested: import crate (49 tests), CLI import tests
(14), the practice month script, `scripts/ci-local.sh` (stable and MSRV).
Not covered: banks other than the synthetic Holvi-shaped fixtures; `Bal`
shapes beyond `Cd`/`Prtry`, `Dt`/`DtTm`.
