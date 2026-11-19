# Assessment of the report-negative-zero-balance review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Committed `.pyc` with a home path | CONFIRMED | ALWAYS | NEUTRAL | NONE | HIGH | FIX — applied (amended before push, `.gitignore`) |
| F2 | Close-year statement bytes change for cleared rows | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | DROP as code change; documented in `result.md` |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 1

- F1: publication rule; the file never reached origin.
- F2: the change is the fix itself. Only a close-year Run parked across the
  upgrade with a zero-summing credit-normal row is affected, and it fails
  closed with a refusal rather than sealing wrong figures. Rejecting and
  restarting the Run recovers. No follow-up issue filed.

No data-integrity finding remains open.

## Simplicity

- Reuses: the domain's existing statement formatter; the ledger report now
  calls it instead of keeping its own `decimal_string`, so the two-decimal
  rule and the zero-sign rule live in one function.
- Removed repetition: `decimal_string` (ledger) and `eur_str` (domain) merged
  into `statement_amount_str`.
- Left out: no change to `Money::amount_str` (canonical voucher bytes are
  hashed and parsing already rejects `-0.00`), and no change to the Rust HTML
  `fi_amount`, which only renders freshly computed reports. The Python PDF
  renderer got a one-line guard because it reads saved JSON that may predate
  the fix.
- Remaining repetition: `account_normal` in `reports.rs` duplicates the
  domain's `account_balance`; out of this slice's scope.
