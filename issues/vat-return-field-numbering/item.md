---
created: 2026-11-27
updated: 2026-11-27
type: bug
status: untriaged
priority: normal
provenance: other
provenance_detail: agent finding while implementing vat-zero-rated-sales-fields
source_ref: taskfleet:01m8f29v4frryht7478xt5rc4y/finding:vat-field-numbering
originating_run: 01m8f29v4frryht7478xt5rc4y
originating_run_kind: spinoff
---

# VAT return field labels 310-317 are not the Tax Administration's VSRALVKV codes

## Description

## Problem

`VatReturnField` (crates/domain/src/vat/filing.rs) says it identifies fields by
"Verohallinto's stable field number", and the offline preview tells the user to
enter "these numbered fields in OmaVero (or send an Ilmoitin.fi file you
prepared yourself)". From 310 upwards the numbers are not the Tax
Administration's data-element codes. The VSRALVKV data-file specification
(A86/200/2016, version 1.10, 27 February 2026) numbers the return as:

| aggountant | meaning | VSRALVKV code |
|---|---|---|
| 310 | sales of goods to other EU countries | 311 |
| 311 | sales of services to other EU countries | 312 |
| 312 | purchases of goods from other EU countries | 313 |
| 313 | purchases of services from other EU countries | 314 |
| 314 | imports of goods from outside the EU | 310 |
| 315 | VAT on imports of goods from outside the EU | 304 |
| 316 | sales of construction services / scrap metal | 319 |
| 317 | purchases of construction services / scrap metal | 320 |

301–303 and 305–309 and 318 agree. The direct Vero API path files by element
name and is unaffected. A user who builds an Ilmoitin file, or who reads the
numbers against Vero's documentation, would put amounts in the wrong fields
(for example the EU goods sales of a synthetic Esimerkki Oy into "imports
from outside the EU").

## Expected

Either renumber the labels to the VSRALVKV codes (the serialized keys appear
in run state and `--json` output, so this needs a compatibility decision) or
stop calling them Verohallinto's numbers and show the OmaVero labels in the
manual-filing view.

## Source

https://www.vero.fi/en/About-us/it_developer/data-format-specifications/specifications__return_for_selfassessed/
(VSRALVKV PDF, section 7 "List of the data content").
