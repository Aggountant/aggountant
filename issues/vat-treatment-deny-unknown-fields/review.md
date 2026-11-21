# Review of vat-treatment-deny-unknown-fields

`/llm-review`, routine rigor. The surface is high-risk (stdin that seals VAT
treatment into the books), so a clean finder result got one focused
false-negative question to the verifier.

- Scope: the diff of the fix commit (`VatTreatment` deny_unknown_fields,
  `deny_members` on `NoVat`, the INPUT help sentence, domain and CLI tests).
- Finder: gpt-6-astra (1 call). Verifier: claude-fable-5-2 (1 call, focused).
- Before the change, confirmed locally that `VatTreatment` is not deserialized
  from stored data. Its only `Deserialize` route is `Assignment.vat_treatment`
  inside `ImportSealInput` on `import camt053 seal` stdin. Sealed vouchers store
  `VatExpansionTreatment` (`crates/model/src/vat_expansion.rs`), and
  reconstruction builds `VatTreatment` in code (`domain_treatment` in
  `vat/roles.rs`). `capture_receipt` also builds it from `ReceiptVatProposal`
  in code.

## Finder result

No findings. The finder independently confirmed the stored-data claim with the
same call sites.

## Verifier (false-negative probe on VatTreatment)

The verifier ran about 80 inputs against the real types (serde 1.0.229, the
workspace lock): foreign members with the tag in any position, members on
`no-vat`, null-valued members, duplicate tags and members, escaped keys, nested
objects and arrays. No input that drops a member was found. Two things depend
on serde internals and are not drops:

- the array form (`["domestic-input","domestic-standard"]`) and one-key-map
  enum spellings for `scope` are accepted, but positionally complete;
- duplicate keys are refused on `from_str` (the production path), not through
  `serde_json::Value`, which no production path uses.

## F1 (should-fix, adjacent, outside this slice): ReceiptVatProposal::NoVat drops members

- Location: `crates/process/src/capture_receipt.rs`, `ReceiptVatProposal`.
- Claim: the same serde gap. `{"type":"no-vat","rate_class":"domestic-standard","scope":"eu-goods"}`
  is accepted on capture-receipt stdin and seals with no VAT leg.
- Evidence: the verifier ran it against the real type. Locally the mechanism is
  the one this slice's own test hit first: before `deny_members`,
  `VatTreatment::NoVat` accepted a `deductibility` member.
- Fix: the same variant-level guard with one shared helper.

Dropped: none.
