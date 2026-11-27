# Result: vat-zero-rated-sales-fields

The VAT return now takes fields 309, 310 and 311 from the books. A tagged sale
posting reports its signed value without VAT:

| VAT code | Field | Vero API element |
|---|---|---|
| `sales-zero-rated` (exports, other 0 % sales) | 309 | `SalesPurchasesImports.ZeroVATRateTurnover` |
| `sales-eu-goods` | 310 | `SalesPurchasesImports.SalesOfGoodsToEUMemberStates` |
| `sales-eu-services` | 311 | `SalesPurchasesImports.SalesOfServicesToEUMemberStates` |

A credit note (the tag on the debit side) reduces the same field.

## What was built

- `crates/domain/src/vat/filing.rs`: `zero_rated_sales_field`, the one owner
  of the rate class → 309/310/311 rule, with the field definitions cited from
  the Tax Administration's *Näin ilmoitat arvonlisäveron* / *Instructions for
  completing the VAT return* and the Vero API element names.
- `crates/domain/src/vat/derive.rs`: the tagged derivation adds each such
  posting's signed, cent-rounded base to its field through the same
  per-voucher buffer as 301–303. A voucher that also carries a `purchase-`
  line keeps its prior behaviour (no sales fields), because legacy
  self-assessed acquisitions were tagged `sales-eu-*` + `purchase-eu-*`.
- `crates/domain/src/vat/guards.rs`, `lib.rs`: `check_zero_rated_sale_not_mixed`
  runs in `build_draft` and refuses (`VAT_STRUCTURE`) a new voucher that puts
  such a sale together with a `purchase-` line, so a new sale cannot be left
  out of the return silently.
- Guide: `reference/status.md` (fi, en) drops the "enter 309–311 yourself"
  limitation, keeps the EC sales list limitation (now `vat-ec-sales-list`)
  and names the new refusal.

## Acceptance criteria

- Synthetic EU goods, EU services and export sales in one month produce 310,
  311 and 309 with their bases:
  `zero_rated_eu_and_export_sales_report_their_bases_in_309_310_311`
  (domain) and `zero_rated_and_eu_sales_are_filed_in_309_310_311` (process:
  offline preview and the API request bytes).
- Credit notes reduce the same fields:
  `credit_notes_reduce_the_zero_rated_sales_fields`, and the process test's
  goods credit note.
- Primary-source citation: the doc comment of `zero_rated_sales_field`.
- Unchanged returns without such sales:
  `a_period_without_zero_rated_sales_derives_the_same_return_as_before` pins
  the full field JSON and totals of a period with every tagged and VAT-A
  shape the return already reads; its expected values were recorded by
  running it against the pre-change `derive.rs`. The whole existing suite
  passes unchanged.

## Store and compatibility

No store migration; nothing sealed or filed changes. A `file-vat` Run started
before the upgrade in a period that has such sales will refuse dispatch with
`VAT_FILING_SOURCE_CHANGED` (its approved fields lack 309–311) and needs a new
preview.

## Review

`/llm-review` (finder `gpt-6-astra`: no findings; verifier `claude-fable-5-2`
on the filing boundary: no such path) and its assessment are in `review.md`
and `assessment.md`. Two DISCUSS notes for the conductor are there.

## Filed during this slice

- `vat-ec-sales-list`: the EC sales list (yhteenvetoilmoitus) is not derived.
- `vat-return-field-numbering`: from 310 upwards the `VatReturnField` labels
  are not the Tax Administration's VSRALVKV data-element codes (aggountant's
  310/311 are VSRALVKV 311/312).
