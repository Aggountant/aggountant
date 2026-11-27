# Assessment of the camt-rich-fields review

Assessed against the slice's final diff (the fix is in the same commit).

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | A reversal's counterparty became the account owner | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | HIGH | FIX (confirmed, real impact on a voucher description) — applied |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: confirmed by reading the selection; a bank listing both parties on a
  reversal is the documented camt.05x usage, so OCCASIONAL. Fixed by reading
  the opposite role first for a reversal only, with the direction's role as
  fallback; both conventions are tested.

No data-integrity finding remains open. No follow-up issue was filed.

## Simplicity

- Reused: the existing per-entry reduction in `parse_entry`, the `child` /
  `children` / `text` helpers, `Money` and `Direction` for amounts and charge
  direction, the existing duplicate-key check for a reversal that reuses the
  original's reference, and the Ledger's existing `SOURCE_FACTS_CHANGED` stop
  across files. No new error code, store field or projection path.
- Repetition: one `read_money` reads every passthrough amount; one
  `read_transaction_code` reads both `BkTxCd` and `RtrInf/OrgnlBkTxCd`; the two
  `Chrgs` layouts (001.02 and 001.03+) read into one shape in one function.
- Left out on purpose: a fingerprint v2 or any identity change (the brief and
  the stored vouchers forbid it); exchange rate and `TxAmt`/`CntrValAmt`
  (the issue asks for the instructed amount only); a reversal-specific event
  key (a reversal has its own reference in the files seen; a reused reference
  stops loudly); automatic account choice for reversals or charges (agent
  judgment, stated in the skill).
