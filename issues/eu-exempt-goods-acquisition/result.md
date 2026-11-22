# Result

An intra-Community acquisition of goods that is exempt in Finland is booked as
`{"type":"reverse-charge","rate_class":"zero-rated","scope":"eu-goods"}`.
Its value reaches field 312 ("Tavaraostot muista EU-maista") with 305 reported
as `0.00`; 307 and 308 and every other field are unchanged.

- Primary source, confirmed: Verohallinto, *Arvonlisäverotus
  EU-tavarakaupassa* (VH/5995/00.01.00/2024, in force from 1.1.2025), ch. 6.6
  ("Arvonlisäverottomat yhteisöhankinnat ilmoitetaan arvonlisäveroilmoituksella,
  mutta niistä ei suoriteta eikä vähennetä arvonlisäveroa") and ch. 12
  ("yhteisöhankinnan arvo ilmoitetaan ainoastaan arvonlisäveroilmoituksen
  kohdassa 'Tavaraostot muista EU-maista'"); the return instructions for the
  box also say "Ilmoita myös verottomat yhteisöhankinnat".
- Design: the return derivation needed nothing new. The full reverse charge
  with scope `eu-goods` already files its source amount in 312 and its tax in
  305, and at the 0 % of `zero-rated` no VAT leg is generated. So the change
  is one admitted pair in `check_treatment_rate_class`
  (`crates/domain/src/vat/expand.rs`); the partial variant (nothing to deduct),
  the `eu-goods`/`eu-services` classes and the service scopes stay refused,
  and every goods refusal names the booking. Exempt services stay `no-vat`.
- Stored data: no new variant, no migration. A voucher of this shape sealed
  before the class check (possible in a store from that time) is
  byte-identical to a new one, verifies, files, and now also re-seals (test
  `a_voucher_sealed_before_the_treatment_class_check_still_verifies_and_files`).
- Text: INPUT help (capture-receipt, bank import), the VAT treatment concept
  page and the error-code guide (fi/en) describe the booking and reserve it
  for a confirmed exemption, since the program cannot tell it from a taxable
  acquisition mistyped as `zero-rated`.
- Tests: domain (`exempt_eu_goods_acquisition_is_one_documented_treatment_with_no_vat_legs`,
  `exempt_eu_goods_acquisition_files_its_value_in_field_312_and_moves_nothing_else`,
  refusal messages for every other zero-rated purchase pair) and CLI
  (`exempt_eu_goods_acquisition_reaches_field_312_with_zero_tax_from_bank_and_receipt`:
  bank import and capture-receipt seal it, `voucher verify` passes,
  `file-vat preview` shows exactly 305 0.00, 307 0.00, 308 0.00, 312 480.00
  and a reconciled VAT account check).

Review and assessment: `review.md` and `assessment.md` (shared with
`vat-expansion-no-vat-members`, done in the same change).
