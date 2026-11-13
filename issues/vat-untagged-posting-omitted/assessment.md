# Assessment: vat-untagged-reconciliation

Source review: `history/review-vat-untagged-reconciliation.md` (head 239f6ba83a)

| ID | Finding | Confirmation | Likelihood | Architecture | Recommendation | Outcome |
|---|---|---|---|---|---|---|
| F1 | Runs persisted before the change bypass the reconciliation refusal (`crates/process/src/file_vat.rs` `assert_current_filing_proposal`) | CONFIRMED | OCCASIONAL | NONE | FIX | applied in this slice |
| F2 | Preview blocking message recommended the rejected remedy (`crates/process/src/vat_reconciliation.rs` `attention_items`) | CONFIRMED | REGULAR | NONE | FIX | applied in this slice |
| F3 | A coded line shields uncoded VAT legs in the same voucher (`crates/domain/src/vat/reconcile.rs` `classify_vat_account_postings`) | CONFIRMED | OCCASIONAL | MINOR | FIX | applied in this slice |

All three findings were confirmed against the code and fixed before merge; none is a data-integrity residual. No finding was dropped, none is spun off, and no issue is filed from this review.

## Simplicity check

- Reused: `DerivedVatFigures`, chart role table, ledger chart list; the per-voucher check reuses `derive_vat_figures` on one voucher rather than restating `base × rate`.
- One classifier serves the period check and the `voucher validate` warning.
- Left out: a settlement voucher marker (model change) and any automatic correction.

## F1: Runs persisted before the change bypass the reconciliation refusal

- Claim: require_reconciled ran only at start; an awaiting Run from an earlier version could dispatch an under-reported return on approval.
- Evidence: Guard missing in dispatch path; regression test fails with the guard disabled, passes with it.
- Fix: Require reconciliation in the pre-dispatch guard for a first dispatch.

## F2: Preview blocking message recommended the rejected remedy

- Claim: Message told the agent to record the VAT with a code in a correcting voucher, which moves VAT to a later return or double-reports it.
- Evidence: Message text contradicted result.md decision 5 and the guide; replaced.
- Fix: Point to filing corrected figures in OmaVero; acknowledge only non-return postings.

## F3: A coded line shields uncoded VAT legs in the same voucher

- Claim: Every VAT leg of a voucher with any vat_code was Tagged, so a partly coded invoice under-reported while the period reconciled.
- Evidence: Reproduced by two new domain tests that failed before the change; fixed and green.
- Fix: Compare a coded voucher's legs per direction with its own derive_vat_figures; demote unexplained legs to untagged.
