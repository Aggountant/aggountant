---
created: 2026-11-27
updated: 2026-11-27
type: feature
status: untriaged
priority: normal
provenance: other
provenance_detail: agent finding while implementing vat-zero-rated-sales-fields
source_ref: taskfleet:01m8f29v4frryht7478xt5rc4y/finding:ec-sales-list
originating_run: 01m8f29v4frryht7478xt5rc4y
originating_run_kind: spinoff
---

# Derive the EC sales list (yhteenvetoilmoitus) from the sealed EU sales

## Description

## Problem

A company with intra-Community sales of goods or services must file the EC
sales list (yhteenvetoilmoitus) for the same period as its VAT return.
aggountant now derives VAT return fields 310 and 311 from sealed
`sales-eu-goods` / `sales-eu-services` vouchers and stores the validated buyer
VAT-ID on each such voucher, but it does not produce the EC sales list, so the
user still enters it by hand in OmaVero, and nothing checks that it agrees
with fields 310 and 311.

## Expected

Derive the EC sales list per buyer VAT-ID (goods, services, triangulation
where modelled) from the same sealed vouchers that feed 310 and 311, and show
it (or file it through the Vero API `File VAT EC sales list` operation) so its
totals equal the return's fields.

## Sources

- Vero API interfaces list *File VAT EC sales list* under self-assessed taxes:
  https://www.vero.fi/en/About-us/it_developer/vero-api/what-are-the-vero-api-interfaces/
- Ilmoitin VSRALVYV data-file specification (self-assessed taxes):
  https://www.vero.fi/en/About-us/it_developer/data-format-specifications/specifications__return_for_selfassessed/
