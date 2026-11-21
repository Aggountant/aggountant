# Result: camt-import-preview-and-balances

The bank statement import now has a preview bound to the approval, and
`parse` shows what the bank says the statement's period and balances were.

```
aggountant import camt053 seal --file S.xml --dry-run < assignments.json
aggountant import camt053 seal --file S.xml --approved-preparation-hash H < assignments.json
```

## What was built

- `seal --dry-run` runs the same preparation as the seal and writes nothing:
  every entry is projected (VAT expansion included) and dry-run through
  `Ledger::validate_seal_with_pending_artifact`, so the preview refuses what
  the seal would refuse: missing or duplicate assignment, a split that does
  not sum, an unsafe VAT tag, an unknown account, a closed period, a natural
  key reused with different content. It returns every voucher draft with its
  natural key, source fingerprint, Ledger warnings and `would_replay`, the
  statement summaries, the statement's content address, the chart version,
  the policy set, the actor and the `preparation_hash`.
- `preparation_hash` (`import_preparation_hash` in the import crate) binds the
  actor, chart version, policy set, the statement bytes' content address and,
  in order, each voucher's natural key, source fingerprint and Ledger draft
  request hash.
- `--approved-preparation-hash` refuses a seal whose preparation differs with
  `IMPORT_PREPARATION_CHANGED` before anything is written. It is optional so
  existing callers keep working; the skill and the guide always pass it. A
  real seal now also returns `dry_run: false`, `preparation_hash` and
  `statements` (additive).
- The whole-batch preflight moved from `Ledger::validate` to
  `validate_seal`, so a closed period or a conflicting natural key now
  refuses the batch before the first seal instead of after the earlier
  entries were sealed. The statement is attached only after preparation
  succeeds, so a refused batch no longer leaves an orphan artifact.
- `Ledger::artifact_address` gives the content address without storing the
  bytes (the artifact store's own function, now named `content_address`).
- `parse` (and both seal outputs) carry `statements`: per `Stmt`/`Rpt` block
  the `id`, `period` (`FrToDt`), `opening_balance` (`OPBD`, else `PRCD`),
  `closing_balance` (`CLBD`), every `Bal` (signed; a debit balance is
  negative), `entry_count`, `entries_net` and `balances_reconcile`
  (`opening + entries_net == closing` in one currency; `null` without both
  balances). A malformed `Bal` is refused as `INVALID_CAMT053`.
- The seal flow moved from `main.rs` into `crates/cli/src/bank_import.rs`.
- Guide (fi/en): import task, sealing-and-approval, status, error codes,
  practice month (and its script, which now previews, approves by hash and
  checks the closing balance). Skill `capture-receipt`: the two sentences the
  issue named are replaced by the preview-and-approve flow and the
  reconciliation flag.

## Not done

- `report statements` does not compare the bank account with the imported
  closing balance. That needs the statement balance stored somewhere the
  report can read; the issue offers it as a later possibility.
- Empty-statement reporting and the camt.052 naming are
  `camt-empty-report-and-rename`, next in this lane.

## Review

`review.md` and `assessment.md`: one finding fixed (mixed-currency entries
could reconcile `true`), one dropped as the intended trade-off (preflight
refusals are not in the rejection log, because the batch must be
all-or-nothing).
