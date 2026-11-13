# Result

A company that starts its aggountant books at the turn of its fiscal year can
date its opening voucher on the old system's last day (31.12.2025) and get
reports, close-year statements and the close package for 2026. The issue's
reproduction now produces statements with an `external_cutover` opening source.

## Decisions for later review

- **Code fix, not guide-only advice** (pre-authorized). The alternative was to tell
  users to date the voucher 1.1.2026. That puts the opening position inside 2026's
  activity and leaves anyone who already followed the old guide stuck with a sealed
  31.12. voucher they cannot remove.
- **Reuse `OpeningBalanceSource::ExternalCutover`** with an identity account mapping
  (sha256 of the sorted `[account, account]` pairs touched), the opening vouchers'
  chart as the source chart, the active chart as the target, the chain hash of the
  last opening voucher as `source_head_hash`, and the new year's first month as
  `live_period`. The alternative was a new variant (for example
  `OpeningVouchers`). It would be more explicit but would change the model and
  domain for no behavioural gain.
- **Strict eligibility**: every prior voucher posts balance-sheet accounts only, on
  the prior year's last day, without attribution, with one chart version, and no
  period up to that day was ever transitioned or VAT-filed here. A looser test
  (any date in the prior year, or result accounts rolled into retained earnings)
  would let a real year's activity reach the next year without being frozen. A
  user who dated the voucher on another day still gets `PERIOD_NOT_SEALED`, and the
  message names the required date.
- **No ordering requirement** between the opening vouchers and the first 2026
  vouchers. Requiring the opening to be sealed first would trap users who record
  January before the opening, and it would not protect anything that the
  sealed-year rule below does not already protect.
- **Nothing enters a year at or before a Sealed fiscal year** (review F1, F3, F4).
  The seal, period transitions and attribution all refuse it. This is stricter
  than before for homes that sealed a year while an earlier year was never closed
  here. A voucher backdated into such a year used to change the next opening
  silently and is now refused with `PERIOD_SEALED`.
- **Mid-year starts are unchanged**: the voucher falls in the live year and carries
  the year's result so far.

## Gates

- `scripts/ci-local.sh`: green (fmt, clippy, cargo-deny, build and test on stable
  and 1.89, practice month).
- `bash scripts/check-user-guide.sh`: practice month ok.
- Review: `review.md` and `assessment.md` beside this file. All four findings were
  confirmed and fixed.
