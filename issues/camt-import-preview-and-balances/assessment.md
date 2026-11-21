# Assessment of the camt-import-preview-and-balances review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Reconciliation added different currencies as one | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX (data integrity: a false `true` is the exact signal the skill trusts) — applied |
| F2 | Real seal's preflight refusal is not in the rejection log | CONFIRMED (call path) | OCCASIONAL | NEUTRAL | MINOR | MEDIUM | DROP (intended trade-off) |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 1

- F1: a camt file without `Acct/Ccy` and with mixed entry currencies is rare
  (Holvi declares the currency), but the flag exists to tell the agent the
  file was read correctly, so a false positive is never acceptable. Fixed; a
  statement whose entries cannot be netted in the balances' currency now
  reconciles `false`.
- F2: the rejection log records commits the Ledger refused. The import's
  preflight refuses the batch before attempting any commit, exactly as it
  already did for an unknown account; the alternative is the old behaviour,
  where the entries before a closed-period entry were sealed and only the
  refused one was logged, a partial batch that the user did not approve as
  such. The preview has to refuse whatever the seal would refuse, and the
  seal has to be all-or-nothing, so one preflight serves both. The refusal is
  still returned to the caller with its code (`PERIOD_SEALED`, `PERIOD_CLOSING`,
  `IDEMPOTENCY_KEY_REUSE`, ...). No issue filed: it is not a defect, and the
  rejection log has no user-facing contract that promises preflight refusals.

No confirmed data-integrity finding remains open.

## Simplicity

- Reused: `Ledger::validate_seal` (the existing dry run of the seal's own
  checks) instead of a second validator; the preparation-hash shape and
  framing of `capture-receipt record` and `BankTransaction::source_fingerprint`;
  `draft_request_hash` as the voucher identity; the artifact store's own
  content address (`content_address`, now shared by `put`, `get` and the new
  `Ledger::artifact_address`).
- Repetition removed: preview and seal run one `prepare` function, so the
  checks and the hashed vouchers cannot drift apart; the seal flow moved out
  of `main.rs` into `bank_import.rs` rather than being duplicated for the
  dry run.
- Left out: making the approval hash mandatory (it would break every existing
  caller; the skill and guide always pass it); a `report statements`
  comparison with the imported closing balance (the issue names it as a
  possible follow-up; it would need the statement balance stored, which is a
  separate design); empty-statement reporting and the camt.052 naming, which
  are `camt-empty-report-and-rename`, next in this lane.
