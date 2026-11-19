# Result: vat-status-values-differ

`file-vat status` and `file-vat reconcile` now compare the Tax
Administration's processed return with the approved request amount by amount,
and name every amount that differs:

```json
"differences": [
  {"field": "VATOnDomesticSalesByTaxRate.HighVATRate", "return_field": 301,
   "approved": 255, "authority": 250}
]
```

The human line (the `message`) carries the same list:
`Differs: 301 VATOnDomesticSalesByTaxRate.HighVATRate: approved 255, authority 250.`

## Why the old comparison failed, structurally

Checked against the published schemas (Vero API developer portal,
SelfAssessedTaxes: `FileVATReturnv2Request`, `GetFiledVATReturnv2Response200`):

- **Every GetFiledVATReturn/v2 group lists all of its amounts, and all are
  `required`.** `VATOnDomesticSales` always has `HighVATRate`, `MediumVATRate`
  and `LowVATRate`; `VATOnPurchasesAndImports` always has its four amounts;
  `SalesPurchasesImports` always has its eight. The request that aggountant
  sends carries only the fields that were reported. The old comparison
  required the two JSON objects to be equal, so a return with only 25.5 %
  sales (`{"HighVATRate": 255.00}` sent, `{"HighVATRate": 255.0,
  "MediumVATRate": 0.0, "LowVATRate": 0.0}` returned) could never match.
- **Groups the request omits are present in the answer**, all zero:
  a month without EU purchases or zero-rate sales sends no
  `VATOnPurchasesAndImports` or `SalesPurchasesImports`, yet the answer has
  both, and an omitted `DeductibleVAT` comes back as `0`.

Either one alone makes every ordinary return `processed_values_differ`. The
October-shaped example in the issue (25.5 % sales plus one deduction) trips
both, which fits two consecutive production periods failing the same way.
The tests never caught it because the mock answered with the request's own
shape; it now answers in the schema's shape.

The issue also listed rate-row arrays and their order. The published schema
has no arrays: rates are named keys, so key order cannot matter. An array
where a group belongs is reported as a difference rather than interpreted.
Numeric strings, which the old `normalize_numbers` left alone, now compare
as decimals.

## What was built

- `crates/broker/src/vero_vat_api.rs`: `FiledVatReturn::value_differences`
  and `processed_differences`, and the public `VatValueDifference`. A missing
  or null amount is zero; numbers, numeric strings and exponent forms compare
  as decimals. Anything the request never carries (unknown keys inside a
  group, VAT relief, payment-basis accounting, amounts under a top-level key
  the comparison does not read) is a difference unless it holds nothing, so a
  different return cannot pass as ours. `semantically_matches_attempt` is
  now "processed, on or after the attempt, and no differences".
- `crates/process/src/file_vat/status.rs`: `FileVatStatus.differences`
  (always present, empty unless the processed return differs), filled for
  `processed_values_differ` and for an interrupted filing whose processed
  return differs.
- `crates/process/src/file_vat/recovery.rs`: `judge` carries the
  differences in `Finding::Other`; `FileVatReconcile.differences` and the
  reconcile message name them.
- `crates/vero-vat-api-mock`: GetFiledVATReturn answers in the schema's
  shape, so every end-to-end filing test now exercises the comparison as
  production does.
- Docs: `docs/user/{fi,en}/troubleshooting.md`, `skills/vero-fi/SKILL.md`,
  and the comparison rule in `crates/broker/AGENTS.md`.

## Tests

- Broker unit tests: the schema-shaped answer matches; omitted and null
  groups and keys, a request zero omitted from the answer, numeric strings
  at other scales, extra zero keys in another order, and exponent notation
  all match; a genuine amount difference and a deduction difference are
  named with both values; unknown non-zero keys, relief, cash basis, a
  renamed group, a non-numeric amount and an array-shaped group never match.
- `crates/process/tests/file_vat_status.rs`: `processed_values_differ` lists
  `differences` in JSON and the message; a schema-shaped answer with numeric
  strings, null groups and relief keys at zero is `processed`.
- `crates/process/tests/file_vat.rs`: reconcile names a differing amount and
  stays unresolved, then adopts the schema-shaped matching answer.
- `scripts/ci-local.sh` passes (fmt, clippy, cargo-deny, stable and MSRV
  tests).

## For the conductor

Run `aggountant file-vat status --period <period> --json` for the two
production periods. If the structural cause above was the whole story, both
now answer `processed`. If either still answers `processed_values_differ`,
`differences` names the amounts to compare in OmaVero.

Review and assessment: `review.md` and `assessment.md` in this directory.
