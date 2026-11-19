# Assessment of the filing-receipt-in-output review

Assessed against the fix commit that follows `01f77ac`.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | VAT receipt attributed to a Run that never sent it | CONFIRMED | RARE | NEUTRAL | MINOR | HIGH | FIX (data integrity: wrong filing evidence) — applied |
| D1 | `show` opens the Ledger for a filed `file-vat` Run | CONFIRMED | NEVER (no realistic trigger) | NEUTRAL | NONE | MEDIUM | DROP |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 1

- F1: rare (a crash between approval and dispatch, then a new Run instead of a
  resume), but the result would be a receipt shown on a Run whose request it
  does not acknowledge, which an agent copies into the company's notes. Fixed
  by reusing `file-vat status`'s "which Run holds the receipt" rule rather
  than adding a second one. The pre-existing adoption without a byte
  comparison is not widened; no issue filed, as it is the follow-up the code
  already names and needs the filed `sent_hash` the Ledger does not expose.
- D1: see the review; no home reaches a filed VAT Run without an openable
  Ledger.

## Simplicity

- Reused: the journal notes the broker already writes for Incomes Register
  effects, `EmployerReportReceipt` and its parser for both travel-expense and
  employer reports, the Ledger's stored VAT receipt, `filing_run_with_receipt`
  for attribution, `filing_outcome_may_be_accepted` for "unknown vs rejected",
  and `run show` as the offline read of a `file-wage` receipt instead of a new
  `file-wage show` command.
- Repetition removed: `file-wage` names its unknown-outcome code as a constant
  instead of a literal. One small adaptation remains: a travel-expense manual
  note has no version, so `incomes_register_receipt` appends `/-` before
  parsing rather than loosening the employer parser that `classify` relies on.
- Left out: no receipt for payment or payroll effects, no new command, no
  network read, no change to journal events or stored state.
