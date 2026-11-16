# Result: VAT settlements are recognised from a bank and tax-account role

## What changed

- **New masterdata file** `masterdata/reference/settlement-account-roles.json`,
  bundled from `crates/domain/data/settlement-account-roles.json`: per exact
  `chart_version`, the accounts VAT is paid and refunded through. The bundled
  `fi-general-2026@dd5d3f33…` gives the role to 1910 and 1920 (not 1900, not 1730).
  `domain::SettlementAccountRoleTable` parses it (canonical chart version, no
  duplicate chart or account, valid account ids, no empty entry, no unknown
  field). The masterdata (`validate_chart_settlement_roles`) refuses to open when
  a role account is missing from its chart snapshot, is not an asset or liability
  account, or carries a VAT role in that chart; the message names the account.
- **`init`** writes the file. **`home upgrade`** adds it as resource
  `settlement_account_roles` when it is missing and the bundled table has an entry
  for the home's active chart (and validates against the home's snapshots and VAT
  roles); otherwise `unavailable`. A differing file is `differs` and left alone. A
  home without the file **opens**; its table is empty.
- **Classifier** (`classify_vat_account_postings`, `settles_vat`): a reduction-side
  untagged VAT posting is a settlement only when every other posting is on a
  VAT-role account, a settlement-role account of the voucher's own chart, or an
  interest or penalty line (expense or liability debits beside output VAT, income or
  asset credits beside input VAT) whose total is smaller than the reduced VAT. Any
  other balance-sheet leg or a money account without the role makes it `Untagged`.
  With no settlement roles for the chart, no payment or refund is a settlement
  (fails closed); a clearing between VAT accounts still is.
- **Reconciliation** carries `settlement_roles_missing` (voucher, chart version) for
  vouchers with an untagged reduction under a chart without roles. `file-vat
  preview` then adds the blocking attention item `VAT_SETTLEMENT_ROLES_MISSING`
  beside `VAT_ACCOUNTS_UNRECONCILED`, naming the chart and `aggountant home upgrade`;
  the `file-vat start` refusal message says the same. The existing acknowledgement
  lets the filing continue.
- **`voucher validate`** under a chart without roles: a reduction-side warning tells
  about the missing roles and `home upgrade` instead of advising a VAT code.
- `crates/ledger/AGENTS.md`: the frozen-roles rule states its scope (filed figures
  and treatment) and why the settlement file was added to the existing edition; the
  upgrade rule says new masterdata goes into a new file.
- Guide fi + en: `tasks/file-vat-omavero.md` (recognition by bank or tax-account
  leg, the three corrections that belong in the return, the new attention code, the
  known gap under Rajoitukset / Limitations), `tasks/upgrade.md` (the new file),
  `reference/error-codes.md` (`VAT_SETTLEMENT_ROLES_MISSING`). Skills
  `vero-fi`, `capture-receipt`, `aggountant-init`; `home upgrade --help`.

Tests: `tests_reconcile.rs` (the three issue examples plus the asset-sale credit
note, a refund of an asset return to the bank, cash refund through 1900, payment from
1920, payment and refund without roles, clearing without roles, a sale not blamed
on missing roles, both known gaps pinned, table shape refusals, bundled roles), every
earlier test unchanged in outcome; `ledger/tests/masterdata.rs` (refusals by
account, a liability account accepted, a home without the file opens);
`masterdata_upgrade.rs` and `cli/tests/home_upgrade_cli.rs` (create, apply, no-op,
differs, unavailable, a recent home gets only the new file and nothing else
changes); `untagged_vat_warning.rs` (three drafts warn, payment does not; without
roles the warning names `home upgrade`); `process/tests/vat_reconciliation.rs`
(period with the three corrections blocks, payment reconciles, without roles the
preview names the roles and an acknowledgement lets start continue).

## Simplicity

- Reuses the existing classifier, its one call path for validate and period check,
  the upgrade's `plan_file` / `create_new_file`, and the existing acknowledgement.
  No new CLI surface; the attention item is one function in
  `process/src/vat_reconciliation.rs`. `reduces_vat` now states the reduction-side
  rule once (it was written in the classifier and is needed by the reconciliation
  and the validate warning).
- The role file is a plain list of account ids per chart version: no role tag, no
  bank / tax-account distinction, because detection treats them the same.
- Deliberately left out: an interest/penalty-account role or a voucher marker (gap
  below), any change to `vat-account-roles.json`, and any fallback to the old shape
  rule for a home without the file.

## Decisions for later review

1. **Accrued interest stays a settlement, so liability debits (output) and asset
   credits (input) count as interest lines.** The issue's rule says any other
   balance-sheet posting makes the reduction untagged, but also that every existing
   test keeps its outcome; the reproduction test of `vat-settlement-with-interest-flagged`
   (a real-world shape) pays 2,00 € of interest accrued on 2740 together with the
   VAT. Under the literal rule it would be flagged again, reopening that bug. I let
   those two balance-sheet kinds count toward the "smaller than the reduced VAT"
   amount test, like expense and income lines. All three issue examples and the
   asset-sale credit note are still flagged (their balance-sheet leg is at least the
   VAT, or of the other side, or equity). Cost: a smaller advance refund sharing a
   voucher with a larger VAT payment passes (pinned as a known gap). Alternative:
   the literal rule, flagging every payment that settles accrued interest.
2. **`VAT_SETTLEMENT_ROLES_MISSING` is `blocking` and appears only with
   `VAT_ACCOUNTS_UNRECONCILED`.** It explains the same block rather than adding a
   separate one; `require_reconciled` still decides on the status. It lists only
   vouchers with an untagged *reduction* under a chart without roles, so an
   untagged sale is not blamed on missing roles.
3. **A home without the file opens; an unreadable or invalid file refuses.** Missing
   is the normal state of every existing home before `home upgrade`; a file that
   exists but is wrong is corrupt masterdata like any other.
4. **Upgrade validates the bundled file against the home's VAT roles when they
   load**, and against chart snapshots only when they do not, so a home that also
   lacks VAT roles still gets both files in one run.

## Known gaps (not closed by this)

When a larger untagged VAT payment or refund shares the voucher, an untagged
output-VAT reduction whose net is an expense or liability debit (credit loss,
customer refund booked on an expense such as 2939 D 500,00 / 7780 D 100,00 / 2939
D 25,50 / 1910 C 625,50, or an advance refund on 2620) or an input-VAT reduction
whose net is an income or asset credit counts as a settlement. Pinned by
`known_gap_an_expense_booked_customer_refund_beside_a_larger_payment_passes` and
`known_gap_an_advance_refund_beside_a_larger_payment_passes_as_accrued_interest`.
Closing them needs a role for interest and penalty accounts or a voucher marker.
