---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: untriaged
priority: normal
provenance: other
provenance_detail: found while rebuilding the month skill against the built binary
source_ref: taskfleet:01m7yfttyc2h090xfx2xtre7aa/gap:vat-reconciliation-before-close
originating_run: 01m7yfttyc2h090xfx2xtre7aa
originating_run_kind: spinoff
---

# VAT reconciliation is only available after the period is locked

## Description

## What goes wrong for the user

The check that tells whether a period's VAT accounts agree with the VAT return
runs only after the period is locked. A voucher with an untagged VAT posting is
therefore found when it can no longer be corrected in that period, and the
user has to file the return by hand in OmaVero with corrected figures.

## Observed (synthetic home, binary built 2026-11-21)

```
$ aggountant --json file-vat preview --period 2026-07
{"error_code":"PERIOD_NOT_SEALED","error_kind":"user","message":"PERIOD_NOT_SEALED: period 2026-07 is Open — file-vat requires a Sealed period"}
```

On a sealed period the same command returns `vat_reconciliation` with
`status`, `untagged_postings` and the per-direction differences. Before the
close the only signal is the `VAT_POSTING_UNTAGGED` warning of
`voucher validate`, which does not block sealing and is seen once, by whoever
prepared that voucher.

## Impact

Sales invoices are special entries whose VAT the agent tags by hand every
month, so this is the ordinary route by which a return ends up wrong. The
month close is the natural point to catch it and the program has the answer,
but only shows it one step too late.

## Direction

Let the read-only reconciliation (and the draft figures) run on an open
period, clearly marked as not fileable, or include the reconciliation status in
what the month close shows before approval (`virtually-tidy-owl` gives that
gate its content).

When this lands, `skills/capture-receipt/SKILL.md` replaces the last paragraph
of "Erikoiskirjaukset ja niiden ALV-merkintä" with a pointer to the check in
the reconciliation section.
