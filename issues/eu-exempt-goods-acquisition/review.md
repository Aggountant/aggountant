# Review: eu-exempt-goods-acquisition + vat-expansion-no-vat-members

- Skill: `/llm-review`, rigor **critical**: the change decides what a VAT
  return reports and what a sealed voucher may hold; a wrong answer is frozen
  into the books and filed with the authority.
- Scope: the two commits of this change before the review fixes ("vat: book
  an exempt intra-Community goods acquisition with zero tax" and "model:
  refuse members on a stored no-vat expansion treatment"): `crates/domain/src/vat/expand.rs`,
  `crates/model/src/vat_expansion.rs`, INPUT help, the VAT treatment page and
  the error-code guide (fi/en), tests in domain, model, ledger and CLI.
- Finder: `gpt-6-astra`, one call over the diff with `expand.rs`, `roles.rs`,
  `vat_expansion.rs`, `wire/src/lib.rs`, both issues and `crates/domain/AGENTS.md`.
  Answer: no findings.
- Verifier: `claude-fable-5-2`, one focused false-negative call (the finder was
  clean and the surface is high-risk): can admitting the pair produce a wrong
  or refused return or a wrong sealed voucher through any caller (filing
  projection, record-time and historical verifiers, `draft_deductible_vat`,
  the Vero payload and its cross-field check, capture-receipt, bank import,
  other readers of the treatment or `resolved_percent`)? It answered path by
  path with file and line evidence and ran the focused tests: no defect.
- Two model calls in total. Risk class: VAT filing figures and sealed-voucher
  readability.

## Confirmed findings

None.

## Notes from the verifier (not defects)

- R1. The doc comment of `EXEMPT_EU_GOODS_ACQUISITION` said fields 305 and
  307 "do not move"; 305 changes from absent to a reported `0.00` (which the
  Vero cross-field check needs, since 312 is then present). Wording only;
  corrected.
- R2. The admitted input has the same shape as the mistake the class check was
  written to stop: a taxable EU goods purchase given `zero-rated` because the
  invoice shows 0 %. Field 312 is still right, but 305 and 307 are understated
  by the tax, and for a buyer without full deduction the payable tax too. The
  Domain cannot tell the two apart from its inputs; this is the cost of the
  issue's decision, not a code defect.
- R3. A chart with no `eu-goods` reverse-charge target refuses the exempt
  booking although it posts to no VAT account. The refusal is the same one a
  taxable EU goods purchase gets on that chart and names the upgrade.
- Not shown in the repository: acceptance by the real Vero API of a return
  with 305 = 0.00 beside 312 > 0. The mock does not validate amounts; the
  payload shape is not new (307 is always sent, explicit zeros included).

## Dropped candidates

None: neither model proposed a candidate.

## Coverage

Read: the whole diff; `check_treatment_rate_class` and the expansion paths;
`sum_vat_account_figures_with_role_table`, `derive_filing_figures_with_role_table`,
the verifiers; `crates/broker/src/vero_vat_api.rs` field mapping and
cross-field check; `wire::decode_voucher_body`; every non-test reader of
`VoucherBody`/`VatExpansion` deserialization. Tested: domain, model, ledger and
CLI suites (see the results). Outside scope: real-API acceptance, real
companies' stores (the conductor scans them read-only after landing).
