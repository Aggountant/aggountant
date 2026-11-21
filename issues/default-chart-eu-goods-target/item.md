---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: open
priority: normal
provenance: other
provenance_detail: found while rebuilding the month skill against the built binary
source_ref: taskfleet:01m7yfttyc2h090xfx2xtre7aa/gap:default-chart-eu-goods-target
originating_run: 01m7yfttyc2h090xfx2xtre7aa
originating_run_kind: spinoff
lane: vat-roles
lane_seq: 0
---

# EU goods purchases cannot be recorded on the default chart

## Description

## What goes wrong for the user

A company that buys goods from another EU country cannot record the purchase
with correct VAT on the chart `aggountant init` creates. The receipt path
refuses, and no other path puts the acquisition into the right fields of the
VAT return.

## Observed (synthetic home from `init`, binary built 2026-11-21)

`capture-receipt record --dry-run` with
`"vat": {"type":"reverse-charge","rate_class":"domestic-standard","scope":"eu-goods"}`:

```
{"error_code":"INVALID_ANSWER","error_kind":"user","message":"INVALID_ANSWER: VAT_STRUCTURE: VAT_STRUCTURE: chart_version \"fi-general-2026@…\" has no reverse-charge role target for EuGoods"}
```

The home's `masterdata/reference/vat-account-roles.json` lists
`reverse_charge_targets` for `eu-services` and `non-eu-services` only. The
same scope is offered by `import camt053 seal` and refuses there too.

The manual route is closed as well, correctly: `voucher validate` refuses a
`purchase-eu-goods` code ("an intra-Community acquisition must be
reverse-charged"), and tagging the self-assessed tax with domestic codes would
report it in the domestic fields instead of the EU acquisition fields.

## Impact

EU goods purchases are ordinary for a small company (equipment, materials,
stock). Today the agent can only leave the invoice open and tell the user the
program's VAT return lacks it, so the user files that period by hand with
corrected figures.

The error is also hard to read: the code is `INVALID_ANSWER` on a path that has
no answer, the `VAT_STRUCTURE:` prefix is doubled, and nothing says what the
user can do.

## Direction

Give the default chart a reverse-charge target for `eu-goods` (and a
`home upgrade` step for existing homes), or, if goods acquisitions are
deliberately unsupported for now, say so in the refusal and in the guide's
`reference/status.md`.

When this lands, `skills/capture-receipt/SKILL.md` drops the bullet that starts
"EU-tavaraosto (`eu-goods`) kieltäytyy oletustilikartalla".
