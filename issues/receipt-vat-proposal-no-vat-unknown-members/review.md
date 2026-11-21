# Review of receipt-vat-proposal-no-vat-unknown-members

`/llm-review`, routine rigor. The surface is high-risk (stdin that seals a
VAT treatment into the books), so the clean finder result got one focused
false-negative question to the verifier.

- Scope: the fix commit (`aggountant_model::de::deny_members`, its use on
  `VatTreatment::NoVat` and `ReceiptVatProposal::NoVat`, the capture-receipt
  INPUT help line, process and CLI tests).
- Finder: gpt-6-astra (1 call). Verifier: claude-fable-5-2 (1 call, focused).
- Locally, before the review: both new receipt tests were run with the
  `ReceiptVatProposal::NoVat` attribute removed and failed
  (`vat_proposal_refuses_members_that_do_not_belong_to_its_type`,
  `no_vat_with_a_rate_class_or_scope_is_refused_on_dry_run_and_record`). The
  CLI record case uses the approval hash of the clean `no-vat` receipt, so
  without the fix the contradictory input would have matched it and sealed.
- Searched `crates/process` and `crates/import` for other stdin-facing VAT
  enums. `import camt053` uses `VatTreatment` (already guarded); `ExpenseNature`,
  `ReverseChargeScope` and `VatCode` are string forms with no members; the
  other internally tagged enums in `process` are not VAT input.

## Finder result

No findings. It confirmed that no stored Run state re-reads
`ReceiptVatProposal` (`CaptureState` uses other types; structured receipt
preparation creates no Run), that serialization round-trips, and that no
other matching VAT enum exists in `process` or `import`.

## Verifier (false-negative probe on ReceiptVatProposal::NoVat)

None found. Checked against a replica pinned to the lockfile's serde and the
built binary: a null-valued member, the tag in any position, duplicate `type`
keys, odd keys and the array form (`["no-vat", ...]` with any remainder) are
all refused; `["no-vat"]` is accepted but carries no member. Through
`serde_json::Value` a duplicate tag loses its refusal, but a surplus member
is still refused, and the CLI parses stdin with `from_str`.

Dropped: none.
