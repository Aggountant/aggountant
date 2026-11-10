---
created: 2026-11-10
updated: 2026-11-10
type: improvement
status: open
priority: normal
lane: vat-roles
lane_seq: 20
collision: [crates/domain/src/vat/roles.rs, crates/domain/src/vat/guards.rs, crates/domain/src/vat/code.rs]
---

# Derive VAT return fields 309-311 for zero-rated and EU sales

## Problem

EU sales are modelled as 0 % rate classes (`eu-goods`, `eu-services`), which
yields the correct tax (zero), but the ledger derivation does not fill the
numbered return fields that report their value: field 309 (zero-rated turnover
other than EU supplies, including exports), 310 (intra-Community supplies of
goods) and 311 (intra-Community supplies of services). The field map exists in
`crates/domain/src/vat/filing.rs`, but no derivation writes to these three
fields, so a company with EU or export sales has to enter them by hand and they
are not checked against the ledger.

These fields also have to agree with the EC sales list (yhteenvetoilmoitus).

## Expected

Sales tagged with the EU goods, EU services and export rate classes contribute
their taxable base to fields 310, 311 and 309 respectively, through the same
`VatReturnFields` path as the domestic fields.

## Acceptance criteria

- [ ] Synthetic EU goods, EU services and export sales in one month produce 310, 311 and 309 with their bases.
- [ ] Credit notes reduce the same fields.
- [ ] Primary-source citation for the field definitions.
