# Review of capture-domestic-input-eu-class

`/llm-review`, routine rigor. The surface is high-risk (the seal boundary for VAT
treatment, and stores that already hold sealed vouchers), so besides the finder
the verifier got one focused false-negative question about that boundary.

- Scope: the diff of the fix commit (`check_treatment_rate_class` and the two
  class lists in `vat/expand.rs`, the `VAT_TREATMENT_CLASS_MISMATCH` domain
  error, capture-receipt's pass-through of that code, the import and capture
  INPUT help, the error-code guide rows, the VAT treatment concept page, and the
  domain and CLI tests).
- Finder: gpt-6-astra (1 call). Verifier: claude-fable-5-2 (1 call, focused).
- Checked locally before the review: the whole workspace test suite passed
  without changes to existing tests; the only bundled examples of a treatment
  (INPUT help, `docs/user/practice/run-practice-month.sh`, CLI tests) use
  domestic classes; the legacy `purchase-zero-rated` tag on bank import is a
  separate tagged path that does not go through the expansion and still
  works (`zero_rate_legacy_tag_remains_a_supported_transport_path`).

## F1 (should-fix): the suggested partial treatment omits its deductibility

- Location: `crates/domain/src/vat/expand.rs`, `check_treatment_rate_class`.
- Claim: for `domestic-input-partial` / `reverse-charge-partial` the message
  suggested a `reverse-charge-partial` object without the required
  `deductibility` member, so copying it verbatim would fail to parse.
- Evidence: local. `VatTreatment::ReverseChargePartial` has a mandatory
  `deductibility` with no default.
- Fix: the suggestion says "plus the same deductibility member" for the partial
  types; the test pins it.

## Verifier: false-negative probe on the seal boundary

(a) New seals: none found that bypass the check. Every ledger seal entry point
reaches `verify_vat_expansion_for_seal_with_role_table`
(`crates/ledger/src/lib.rs`, before the commit), which re-runs
`vat_expand_with_role_table` and so the new check. `voucher seal` takes a
`DraftInput`, which cannot carry a `vat_expansion`. The two draft producers
(capture-receipt, bank import) hit the check before a draft exists.

(b) Already sealed vouchers: none found that read, verify, report or file
through the check. Filing uses the sealed-resolution verifier
(`vat_expand_resolved_with_roles`), which does not run it.
`a_voucher_sealed_before_the_treatment_class_check_still_verifies_and_files`
pins this.

R1 (adjacent): an idempotent replay of an entry that was sealed with such a
pair before this change now fails in preparation instead of replaying
(`import camt053 seal` re-run on the very same statement file with the same
assignments; `capture-receipt record` re-run with the same input). Checked
locally: a replay must be byte-identical, attachment included
(`validate_seal_inner` compares the request hash), so a different, overlapping
statement file never replays anyway. Only a retry of the exact same batch across
the upgrade is affected. The outcome is a loud refusal that names the right
treatment, and a changed assignment then gets `IDEMPOTENCY_KEY_REUSE`, which
tells the user to correct the sealed voucher with a new one.

## Dropped

1 out of scope: `voucher validate` accepts a bare `VoucherDraft` with a
`vat_expansion` and does not run the expansion verifier. It writes nothing and
no command seals that shape; it predates this change.

## Coverage and residual risk

Read: the diff, `vat/expand.rs`, `vat/roles.rs` (verification and filing),
`ledger/src/lib.rs` seal and validate paths, `process/src/capture_receipt.rs`,
`import/src/lib.rs`, `cli/src/bank_import.rs`. Tested: workspace suite and
`scripts/ci-local.sh`. Not reviewed: whether Finnish law has an EU acquisition
that is legitimately reported with zero self-assessed tax; the change refuses
such a case as unmodelled (the domain's closed-categories rule) rather than
deciding it.
