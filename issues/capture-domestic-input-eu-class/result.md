# Result

A purchase treatment with a zero-rated sale class is refused before the seal.
`capture-receipt record` and `import camt053 seal` (also `--dry-run`) answer
`VAT_TREATMENT_CLASS_MISMATCH` for `domestic-input` with `eu-goods`,
`eu-services` or `zero-rated`, and for the partial and reverse-charge types with
those classes. The message names the treatment that fits, for example
`{"type":"reverse-charge","rate_class":"domestic-standard","scope":"eu-goods"}`
for an EU goods purchase and `{"type":"no-vat"}` for a purchase without Finnish
VAT, and the error JSON carries the guide's hint for the code.

- Rule (`crates/domain/src/vat/expand.rs`, `check_treatment_rate_class`): a
  purchase treatment takes its rate from a domestic supply class
  (`PURCHASE_RATE_CLASSES`): the seller's VAT for `domestic-input`, the Finnish
  VAT the buyer self-assesses for `reverse-charge`, where EU or non-EU is the
  `scope`. `zero-rated`, `eu-goods` and `eu-services` name zero-rated sales
  (`ZERO_RATED_SALE_RATE_CLASSES`) and no purchase admits them. An acquisition
  exempt from Finnish VAT is refused as unmodelled. The decision is by class,
  never by percentage, so domestic passenger transport at its 2023 0 % still
  expands. A unit test pins that every class in the dated table is in exactly
  one list.
- The check runs in the shared step of `vat_expand` and
  `vat_expand_with_role_table`, so the ledger's seal-time re-expansion enforces
  it too. Historical verification and filing re-run only the sealed arithmetic
  and skip it, so a store holding such a voucher sealed earlier still opens,
  verifies, reports and files (test
  `a_voucher_sealed_before_the_treatment_class_check_still_verifies_and_files`).
  The one effect on old data: a retry of the exact same batch that already sealed
  such an entry is refused instead of replaying.
- `capture_receipt.rs` passes this domain code through; other domain errors
  there stay `INVALID_ANSWER`. `CAPTURE_VAT_CLASS_UNKNOWN` now lists the classes a
  purchase can use instead of every known class.
- INPUT help for capture-receipt and the bank import, the error-code guide
  (fi/en) and the VAT treatment concept page (fi/en) describe the rule.
- Tests: domain (every purchase type x sale class, both entry points, the
  messages, domestic classes still valid, sealed vouchers still verify and file)
  and CLI (the issue's reproduction on capture-receipt, dry run and record, with
  the hint; the same on bank import, dry run and seal, with no fiscal vouchers).

Review and assessment: `review.md` and `assessment.md`.
