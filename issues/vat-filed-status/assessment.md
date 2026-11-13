# Assessment of the vat-filed-status review

| # | Finding | Classification | Outcome | Notes |
|---|---|---|---|---|
| F1 | No-activity matcher ignores remote amounts | CONFIRMED | FIX | Applied in `vero_vat_api.rs` with a unit test; makes reconcile stricter, never looser |
| F2 | Several succeeded Runs made status fail | CONFIRMED (narrowed) | FIX | Identical request bytes are no longer ambiguous; differing bytes fail closed with the receipt in the message |
| F3 | 200 without a readable receipt reported as `not_filed` | CONFIRMED | FIX + SPIN_OFF | Status answers `outcome_unknown`; broker classification filed as intake `staggeringly-aspiring-sky` |
| D1 | `Ledger::open` migration in a read-only command | INCORRECT for this change | DROP | Same convention as every other read, including `file-vat preview`; no filing state changes |
| D2 | Exact Run selection via the ledger's filing `sent_hash` | CONFIRMED as an improvement | DISCUSS | Needs a `crates/ledger` read, outside this task's boundary; current behaviour fails closed |

No confirmed data-integrity finding remains open. F3 was the one that could have
misled a user into filing twice, and it is fixed on the status side, with the
filing-path root cause in intake.

## Simplicity

- Reused: the existing `GetFiledVATReturn` adapter, `FiledVatReturn` matcher,
  the ledger's receipt and manual-evidence reads, and the Run journals. The
  status command adds no store, no new broker operation and no journal event.
- Repetition removed: the GetFiledVATReturn request construction and the
  approved-request parse are now one helper each, shared by `reconcile` and
  `status`.
- Left out on purpose: recording the fetched status as an observation, a
  `--run-id` selector beside `--period`, and use of the outbox. The outbox is
  gitignored, so a fresh clone of the books would lack it; the journals carry
  the same information.
