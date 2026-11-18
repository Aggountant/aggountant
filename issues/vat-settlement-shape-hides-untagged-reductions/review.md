# Review: vat-settlement-shape-hides-untagged-reductions

`/llm-review`, rigor routine on a high-risk surface (VAT reconciliation that
gates authority filing; masterdata upgrade of live homes).

- Scope: `git diff main` of the implementation commit (`crates/` and
  `crates/ledger/AGENTS.md`), read against the issue and `result.md`.
- Finder: gpt-6-astra (1 call). Verifier: claude-fable-5-2 (1 call, a focused
  false-negative question on the no-roles fail-closed boundary after the fix).
- The first review attempt by the worker that wrote the implementation produced
  no output (the finder call failed on provider quota); this review was run in
  full by the harvesting worker.

## F1 (must-fix): without settlement roles a net VAT payment settled its input-VAT leg

Location: `crates/domain/src/vat/reconcile.rs`, `settles_vat`.

Claim: with no settlement roles for the voucher's chart, a bank leg fell through
to the interest-line test. Beside an input-VAT reduction an asset credit counts
as refund interest, so in a net payment `2939 D 255,00 / 1763 C 200,00 / 1910 C
55,00` the bank credit (55,00) was smaller than the reduced VAT (455,00) and
the 1763 leg became a `Settlement`. That contradicts the specified fail-closed
behaviour ("without roles no payment or refund is a settlement").

Evidence: new test
`a_net_vat_payment_in_a_chart_without_settlement_roles_settles_no_leg` fails on
the implementation commit and passes after the fix.

Fix: without roles, any posting off the VAT-role accounts makes the reduction
untagged; a clearing purely between VAT accounts still settles.

## F1b (must-fix, found while checking F1): a fail-closed payment could net away an untagged sale

Location: `reconcile_vat_accounts`, status.

Claim: the status is `unreconciled` only when untagged postings exist and the
difference exceeds rounding. Without roles a payment is counted as untagged,
so in a period where it exactly equals an untagged sale's VAT the output
accounts agree with the return, the period read `reconciled`, and the sale's
VAT would drop out with no attention item (the roles-missing item is emitted
only for an unreconciled period).

Evidence: new test
`without_settlement_roles_a_payment_never_reconciles_by_netting_an_untagged_sale`.

Fix: a non-empty `settlement_roles_missing` keeps the period `unreconciled`.

Verifier on the fixed code: "No." — no other no-roles shape settles a
reduction, and the status cannot be `reconciled` while roles are missing.

## Dropped

1. F2 (gpt-6-astra, should-fix): the account-kind lookup in
   `process/src/vat_reconciliation.rs` comes from the active chart, not each
   voucher's snapshot. Pre-existing (the `kinds` map predates this change and is
   not touched by it); it only bites when a later chart edition removes or
   re-kinds an account old vouchers used, and then fails toward `Untagged`
   (blocking), not toward a silent omission. Out of scope; not filed.

## Coverage and residual risk

Read: the full diff of `reconcile.rs`, `git_masterdata.rs` (open, init,
upgrade, validation), `ledger/src/lib.rs` (validate warning),
`process/src/vat_reconciliation.rs`, and their tests. Workspace tests pass on
stable and 1.89 after the fix. Not separately reviewed: guide and skill prose
beyond consistency with the behaviour. The accepted known gaps in `result.md`
remain.
