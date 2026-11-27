# Review: vat-zero-rated-sales-fields

- Skill: `/llm-review`, rigor **routine** with a high-risk surface: the change
  alters authority-facing VAT return content (fields 309–311 in the preview,
  the approval hash and the Vero API request).
- Scope: the staged diff of this slice (`crates/domain/src/vat/derive.rs`,
  `filing.rs`, `guards.rs`, `lib.rs`, tests in `tests_derive.rs`,
  `tests_guards.rs`, `tests_roles.rs`, `crates/process/tests/file_vat.rs`,
  `docs/user/*/reference/status.md`, `CHANGELOG.md`), read with `derive.rs`,
  `filing.rs`, `guards.rs`, `roles.rs`, `reconcile.rs`,
  `crates/process/src/file_vat.rs` and `crates/broker/src/vero_vat_api.rs`.
- Finder: `gpt-6-astra`, one call. Result: "No findings."
- Verifier: `claude-fable-5-2`, one focused false-negative call on the
  high-risk boundary: can a period without the three tags file anything
  different, and can such a sale be counted twice, with the wrong sign or
  omitted (account-sum path, `draft_deductible_vat`, reconcile, replacement
  read-back, API projection)? Result: "No such path", with line references.
- Two model calls in total.

## Confirmed findings

None.

## Notes outside the question (from the verifier)

### N1: a mis-sided zero-rated tag in existing books derives a negative value

- Location: `crates/domain/src/vat/derive.rs`, the zero-rated block in the
  `VatDirection::Sales` arm.
- Claim: a sealed voucher whose `sales-eu-*` / `sales-zero-rated` tag sits on
  the debit (receivable) line instead of the revenue line now derives a
  negative 309–311 value; before, a 0 % tag on either side had no effect.
- Evidence: the derivation uses the same `signed_by_side` convention as
  301–303, where a debit-side sale tag is a credit note. A mis-sided domestic
  tag would already have misstated output VAT, so the convention is in use;
  whether any real book has a mis-sided 0 % tag cannot be seen from the code.

### N2: periods filed before this change show no attention for 309–311

- Location: `crates/process/src/file_vat.rs`, `VAT_REPLACEMENT_DUE` is raised
  only for correction vouchers sealed after the filing.
- Claim: a period filed through the API before this change, containing these
  tags, now previews 309–311 but is not flagged as differing from the filed
  return. `file-vat replace` does report the difference.

## Dropped candidates

None proposed.

## Coverage

Tests run: `scripts/ci-local.sh` (fmt, clippy, deny, stable and MSRV build
and test, user-guide gate, practice month). The pinned-return test
`a_period_without_zero_rated_sales_derives_the_same_return_as_before` was
run against the pre-change `derive.rs` to record its expected values. Not
checked: the real GetFiledVATReturn answer's key names (no vendored schema;
a mismatch fails closed as a reported difference).
