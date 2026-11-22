# Result

`VatExpansionTreatment::NoVat` refuses members through
`aggountant_model::de::deny_members`
(`crates/model/src/vat_expansion.rs`).

Why no stored voucher can carry one, so the tightening reinterprets nothing:

- Write path: `VatExpansion` has private fields and is built only by
  `VatExpansion::new`, whose only production caller is the domain's
  expansion; the derived `Serialize` writes a unit variant of an internally
  tagged enum as its tag alone, `{"type":"no-vat"}`.
- Read path: every stored body is read through
  `aggountant_wire::decode_voucher_body`, which re-encodes the parsed body and
  refuses bytes that differ. A `no-vat` with a member would re-encode without
  it, so before this change it was already refused as `NON_CANONICAL_WIRE`
  (checked by hand with the attribute removed); it is now refused while
  decoding (`WIRE_DECODE`). No other non-test code deserializes a
  `VoucherBody` or `VatExpansion`.
- Synthetic store scan: ledger test
  `a_stored_no_vat_expansion_carries_no_members_and_a_member_never_decoded`
  seals a `no-vat` expansion, scans every stored body's
  `vat_expansion.treatment` of type `no-vat` and finds only
  `{"type":"no-vat"}`, then shows an injected member does not decode.
- Model test `no_vat_treatment_refuses_members_instead_of_dropping_them`.

Review and assessment: `../eu-exempt-goods-acquisition/review.md` and
`assessment.md`.
