---
created: 2026-11-12
updated: 2026-11-13
type: bug
status: in-progress
priority: high
provenance: other
provenance_detail: observed while implementing non-calendar-fiscal-year
source_ref: taskfleet:01m77hp9gbv26hpbdcfxjgc38m/observation:year-boundary-opening-voucher
originating_run: 01m77hp9gbv26hpbdcfxjgc38m
originating_run_kind: spinoff
lane: year-end
lane_seq: 1
collision: [crates/ledger]
preparation: ready
---

# Opening voucher dated the old system's last day blocks every report at a year-boundary start

## Description

## What happens

A calendar-year company that starts its aggountant books at the turn of a
fiscal year, and follows the user guide's setup step ("date the opening
voucher on the old system's last day"), cannot produce any report for its
first aggountant year.

Reproduced with a fresh synthetic home (Esimerkki Oy, made-up figures) on
`main` at 505aa12:

```sh
aggountant --home H init
# opening balances dated the old system's last day, 31.12.2025
echo '{"draft":{"document_date":"2025-12-31","posting_date":"2025-12-31","description":"Avaavat saldot","postings":[{"account":"1910","amount":{"amount":"100.00","currency":"EUR"},"side":"debit"},{"account":"2000","amount":{"amount":"100.00","currency":"EUR"},"side":"credit"}]},"natural_key":"open","actor":"t"}' \
  | aggountant --home H --json voucher seal
aggountant --home H --json report statements --period 2026-01 --company "Esimerkki Oy" --format native-json --output r.json
# => PERIOD_NOT_SEALED: fiscal year 2025 must be Sealed before its closing balances can open 2026
```

## Why

`Ledger::fiscal_year_data` (`crates/ledger/src/lib.rs`) treats any voucher in
an earlier fiscal year as prior activity and requires that year to be
`Sealed` before it builds the next year's opening balance. The opening
voucher is the only activity in 2025, and 2025 can only be sealed by closing
all its periods and running close-year for a year the company never kept in
aggountant. The monthly report, close-year statements and the close package
all go through `fiscal_year_data`, so all of them fail for 2026.

The guide's advice is right for a mid-year start (the voucher then falls in
the same fiscal year and carries the year's result so far). At a year
boundary it leads straight into this refusal.

## Possible directions

- At a fiscal-year boundary, date the opening voucher on the first day of the
  new year (it holds only balance-sheet accounts then), and say so in the guide
  and the `aggountant-init` skill; or
- give external opening balances a first-class path (the model already has
  `OpeningBalanceSource::ExternalCutover`, which nothing in the CLI can create).

Found while implementing `non-calendar-fiscal-year`; the July–June company
in that change avoids it because no fiscal year exists before its first
defined year, so its opening voucher must be dated the year's first day.
