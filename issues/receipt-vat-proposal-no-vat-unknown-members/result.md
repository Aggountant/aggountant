# Result

`capture-receipt record` (both `--dry-run` and with an approval hash) now
refuses a `no-vat` proposal that carries any other member, such as
`{"type":"no-vat","rate_class":"domestic-standard","scope":"eu-goods"}`, with
INVALID_INPUT naming the member. Nothing is sealed.

- `crates/model/src/de.rs`: `deny_members` moved here from `domain` as the
  one owner. Serde's container-level `deny_unknown_fields` does not reach a
  unit variant of an internally tagged enum; the helper is put on such a
  variant with `deserialize_with`.
- `crates/process/src/capture_receipt.rs`: `ReceiptVatProposal::NoVat` uses
  it. `crates/domain/src/vat/expand.rs`: `VatTreatment::NoVat` uses the shared
  helper instead of its private copy. Behaviour of `VatTreatment` is unchanged.
- `crates/cli/src/input_help.rs`: the capture-receipt INPUT help says `no-vat`
  takes no other member.
- Tests: `vat_proposal_refuses_members_that_do_not_belong_to_its_type`
  (process; every variant parses, round-trips and refuses foreign and unknown
  members); `no_vat_with_a_rate_class_or_scope_is_refused_on_dry_run_and_record`
  (CLI; INVALID_INPUT on dry-run and on record with the clean receipt's
  approval hash, zero vouchers afterwards); and
  `no_vat_with_a_rate_class_or_scope_is_refused_before_sealing` in
  `import_camt053` pinning the already-closed import path end to end. Both
  receipt tests fail without the fix.
- Other stdin-facing VAT inputs in `process` and `import` were checked; none
  had the gap (import uses the guarded `VatTreatment`).
- Stored data is unaffected: `ReceiptVatProposal` is read only from stdin, and
  `VatExpansionTreatment` (sealed vouchers) is unchanged. A saved receipt file
  with surplus members on `no-vat` is now refused when resubmitted, which is the
  intended behaviour.

Review and assessment: `review.md` and `assessment.md`.
