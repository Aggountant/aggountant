---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: open
priority: normal
provenance: other
provenance_detail: found while rebuilding the month skill against the built binary
source_ref: taskfleet:01m7yfttyc2h090xfx2xtre7aa/gap:voucher-vat-code-help
originating_run: 01m7yfttyc2h090xfx2xtre7aa
originating_run_kind: spinoff
lane: agent-surface
lane_seq: 6
preparation: ready
---

# Help does not say which vat_code values a manual voucher may carry

## Description

## What goes wrong for the user

A manually prepared voucher (a sales invoice, a correction) reaches the VAT
return only through the `vat_code` on its base line, but the program does not
say which codes exist. The agent has to guess a code and read the refusal.

## Observed (binary built 2026-11-21)

`voucher seal --help` and `voucher draft --help` say only: "Each posting may
also carry optional vat_code." The codes are built from a direction and a rate
class of the dated rate table (`sales-domestic-standard`,
`purchase-domestic-reduced-1`, `sales-eu-goods`, a `-deduct50` suffix, a
margin-scheme form), and some combinations are refused (`purchase-eu-goods`,
a partial deduction on a sales code). None of that is reachable from the
binary without a failed attempt, and there is no command that lists the rate
classes valid on a date.

## Impact

Every company with sales records sales invoices this way each month. A wrong
or missing code is silently wrong VAT until the return's reconciliation, which
runs only after the period is locked.

## Direction

An INPUT section for `vat_code` in `voucher seal --help` (grammar, where the
code goes: the base line, not the VAT account line, and what the ledger does
with it), and a read-only way to list the rate classes valid on a date.

When this lands, `skills/capture-receipt/SKILL.md` drops the sentence that
places the code ("Merkintä kuuluu veron perusteen riville… laskisi veroa
verosta") down to its reason.
