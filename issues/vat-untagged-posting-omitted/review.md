# Review: vat-untagged-posting-omitted (period VAT reconciliation)

- Finder: gpt-6-astra (`--task review`, diff HEAD~1 of 239f6ba over domain/process/ledger/cli sources)
- Verifier: claude-fable-5-2, one focused false-negative question on the reconciliation boundary (high-risk surface, finder candidates were already confirmed locally and by a failing test)
- Model calls: 2
- Rigor: routine; risk class high (VAT authority filing, silently wrong bookkeeping)
- Scope: `crates/domain/src/vat/reconcile.rs`, `crates/process/src/vat_reconciliation.rs`, the `file_vat.rs` wiring, `ledger` validate warning, CLI input/output

## F1 — must-fix: Runs persisted before the change bypass the refusal

Location: `crates/process/src/file_vat.rs` `assert_current_filing_proposal` (dispatch guard).

Claim: `require_reconciled` ran only in `start_file_vat`. A Run awaiting `ApproveFiling` from an earlier version carries no acknowledgement and its fingerprint/figures are unchanged, so approving it after upgrade dispatched an under-reported return.

Evidence: local reading confirmed; the pre-dispatch guard only compared fingerprint and figures. Regression test `a_run_started_before_the_check_cannot_dispatch_an_unreconciled_filing` fails with the guard disabled and passes with it.

Fix (applied): require reconciliation in the shared pre-dispatch guard for a first dispatch only (submit effect still `Planned`), so an attempt already made still recovers.

## F2 — should-fix: preview message recommended the remedy the design rejects

Location: `crates/process/src/vat_reconciliation.rs` `attention_items`.

Claim: the blocking item said "record the VAT with a VAT code in a correcting voucher", but a closed month cannot be reopened, so that puts the VAT into a later return or reports it twice.

Evidence: confirmed against `result.md` decision 5 and the user guide.

Fix (applied): message now says to check against the source, file VAT that belongs in this return in OmaVero with corrected figures without re-recording it later, and acknowledge only postings that do not belong in a return.

## F3 — must-fix (verifier): a coded line shields uncoded VAT legs in the same voucher

Location: `crates/domain/src/vat/reconcile.rs` `classify_vat_account_postings` (all VAT legs of a voucher with any `vat_code` were `Tagged`).

Claim: a two-line sales invoice with the code missing on one line (1700 Dr 251,00; 3000 Cr 100,00 coded; 3000 Cr 100,00 uncoded; 2939 Cr 51,00) derives 25,50 € while 2939 holds 51,00 €, yet the period was `reconciled`. Same for a purchase credit note, and for a payout where a purchase code sits next to an uncoded sale leg.

Evidence: traced by the verifier; reproduced locally as tests `a_partly_tagged_invoice_does_not_hide_the_uncoded_lines_vat` and `a_code_of_the_other_direction_does_not_shield_an_uncoded_sale`.

Fix (applied): a coded voucher's VAT legs are compared per direction with that voucher's own `derive_vat_figures`; legs differing by more than a cent per coded line are `untagged`. Codes with no VAT legs stay owned by the codes (`coded_lines_without_vat_legs_stay_owned_by_their_codes`).

## Unresolved uncertainty

- Rounding tolerance sums over the period's coded lines (verifier note); accepted and recorded in `result.md` decision 3.

## Dropped

0 candidates dropped. The verifier's "checked, not cases" (generated legs, unknown direction/currency, settlement mirror, aggregate-equal tagged/untagged) match accepted gaps or are not under-reporting.

## Coverage

Read: the full diff, `roles.rs`, `derive.rs`, `file_vat.rs` dispatch paths. Tested: domain, process and CLI suites plus the new regression tests. Outside scope: imported-chart role tables in real homes (direction fallback by account kind is unit-tested only).
