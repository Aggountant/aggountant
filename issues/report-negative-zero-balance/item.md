---
created: 2026-11-19
updated: 2026-11-19
type: bug
status: untriaged
priority: normal
provenance: other
provenance_detail: observed while running the practice month
source_ref: taskfleet:01m7t18mhrten3hbcn2y49fb94/observation:report-negative-zero
originating_run: 01m7t18mhrten3hbcn2y49fb94
originating_run_kind: spinoff
---

# Monthly report prints a cleared liability account as −0,00

## Description

## Problem

`aggountant report statements` prints a cleared balance-sheet account as a
negative zero. In the practice month (`docs/user/practice/run-practice-month.sh`,
fictional Esimerkki Oy), the purchase invoice is recorded against 2700
Ostovelat (credit 124.00) and the bank row clears it (debit 124.00). The
report then shows:

- HTML: `2700 Ostovelat −0,00 EUR` under "Lyhytaikainen vieras pääoma"
- native JSON: `{"account":"2700","account_name":"Ostovelat","amount":"-0.00"}`

## Impact

The guide now teaches the owner to check at month end that trade payables
are zero (or only unpaid invoices). A "−0,00" in that exact place reads as a
small debit balance or an error and invites a needless investigation. The
figures and totals are correct; only the sign of zero is wrong. JSON
consumers comparing the string to "0.00" also mismatch.

## Expected

A zero balance prints as `0,00` / `"0.00"` regardless of the side the
liability sign flip comes from.

Observed while implementing `bank-row-for-recorded-receipt` (2026-11-19).
