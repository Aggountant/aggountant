# Assessment of the camt-empty-report-and-rename review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Pending entries without a reference were not counted in `empty_report` | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX — applied |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: confirmed by reading `parse_entry`; fixed with a separate counter in
  the entry loop and a regression test. No follow-up issue.

## Decisions taken in the slice

- **Result, not error.** An empty report is `ParsedStatement.empty_report`
  (account, periods, unbooked count, message), carried into parse, the
  `--dry-run` preview and the seal, and printed on the human line. An error
  code would have needed a flag to import a genuinely quiet month; the
  preview-then-approve flow already gives the agent and the user a point to
  stop, and the preview they approve now says the report was empty. When the
  opening and closing balances differ, the message says the file is missing
  entries and not to import it, matching the existing `balances_reconcile:
  false` rule.
- **Old verb kept, old error code not.** `import camt053` and `import
  camt052` are visible clap aliases of `import camt`; production agents and
  scripts keep running. `INVALID_CAMT053` is renamed to `INVALID_CAMT`
  without an alias, because one `error_code` field cannot carry both and a
  permanent duplicate would keep the misleading name. Exit code and
  `error_kind` are unchanged, so a caller that stops on failure is not
  affected; the rename is listed as breaking in the CHANGELOG. The guide's
  error-code table gains an `INVALID_CAMT` row, which `explain` and the
  error JSON's `hint` now read (the old code had none).

## Simplicity

- Reused: the existing per-block `StatementSummary` (periods and
  `balances_reconcile`) to compose the empty report; no second reconciliation.
  The CLI passes the parser's struct through; preview and seal share one
  `add_empty_report` helper and the human renderer one `with_empty_report`.
- Repetition: the empty-state rule lives once, in `parse_camt`
  (`transactions.is_empty()`); every output reads the same field.
- Left out: a refusal or `--allow-empty` flag, a per-block empty flag (the
  per-block `entry_count` already shows it), and any change to the bank
  event identity or store schema.
