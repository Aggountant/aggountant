# Assessment of the vat-zero-rated-sales-fields review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| N1 | Mis-sided 0 % tag derives a negative 309–311 | UNABLE_TO_VERIFY | RARE | NEUTRAL | NONE | MEDIUM | DISCUSS (conductor) |
| N2 | Periods filed before this change are not flagged | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | DISCUSS (conductor) |

FIX: 0   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 2   DROP: 0

- N1: the derivation follows the one sign convention the return already uses
  for 301–303; changing it for 0 % lines only would split the rule. A negative
  field is visible in the preview before approval, and Ilmoitin itself warns
  on negative 309–312 (VSRALVKV message #1009), so it is not silent at the
  authority either. Whether the live books contain such a voucher is a data
  question: the conductor can check the two companies' homes for 0 % sale tags
  on a debit line before installing.
- N2: until this change the guide told users with EU sales or exports to file
  in OmaVero and enter 309–311 by hand, so a direct filing of such a period
  without those fields is unlikely. A replacement made with `file-vat replace`
  reads the filed return and reports the difference. No issue filed: no
  observed occurrence; the conductor's pre-install check (does any filed
  period contain these tags?) settles it for the live homes.

No confirmed data-integrity finding remains open.

## Simplicity

- Reuses the existing per-voucher `domestic_sales_fields` buffer and its
  mixed-direction rule, `VatReturnFields::add_computed`, and the broker's
  existing 309–311 API mapping; no new types, no projection change.
- One owner for the rate-class → 309/310/311 rule (`zero_rated_sales_field`
  in `filing.rs`), used by both the derivation and the new record-time guard,
  beside the `301/302/303` owner.
- Left out: the EC sales list (filed as `vat-ec-sales-list`), renumbering the
  field labels to the VSRALVKV codes (filed as `vat-return-field-numbering`),
  and 0 % passenger-transport sales of 2023 (`domestic-passenger-transport`
  stays outside the fields as before).
