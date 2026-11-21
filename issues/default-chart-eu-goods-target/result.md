# Result: default-chart-eu-goods-target

An EU goods purchase can now be recorded on the chart `aggountant init`
creates, and its VAT return figures come out in the EU goods fields.

## What was built

- `crates/domain/data/vat-account-roles.json`: the `fi-general-2026` roles gain
  `{"scope":"eu-goods","input_account":"1763","output_account":"2939"}`, the
  same domestic pair the two service scopes use. `capture-receipt record` and
  `import camt053 seal` with `scope: eu-goods` now seal; the return projection,
  which reads the sealed treatment rather than accounts, puts the base in 312,
  the self-assessed tax in 305 and the deduction in 307.
- `home upgrade` brings the target to existing homes. It replaces
  `vat-account-roles.json` only while the file is byte for byte what an earlier
  release's `init` wrote (`crates/ledger/data/retired-vat-account-roles/`) and
  the bundled table only adds reverse-charge targets to it
  (`VatAccountRoleTable::only_adds_reverse_charge_targets_to`, checked at run
  time and by a unit test). The replacement is temp-file-then-rename after
  re-checking the bytes. An edited or company-imported role file is still
  `skipped` / `differs` and left untouched. Dry-run reports
  `{"action":"update","resource":"vat_account_roles",...}` under `would`;
  apply lists it under a new `updated` key (`created` unchanged).
- The refusal on a chart without a target now names the scope in its written
  form (`eu-goods`) and says what to do (`home upgrade --dry-run` on the
  default chart; the company's own chart import otherwise).
- Guide (`tasks/upgrade.md`, fi and en), `home upgrade --help`, the
  capture-receipt skill (the `eu-goods` refusal bullet is gone) and the ledger
  rules in `crates/ledger/AGENTS.md` (the one sanctioned in-place role change
  and how to retire a bundled role file).

## Verified

- A home created by the pre-change binary: `home upgrade --dry-run` plans
  exactly one item, an update of `vat-account-roles.json`; after `--apply`,
  `git diff` of the home shows only the added `eu-goods` target, the EU goods
  receipt records, `voucher verify` is `chain ok`, and a second run changes
  nothing.
- Tests: `masterdata_upgrade.rs` (unedited older file gains only the target,
  everything else byte-identical; edited file left untouched),
  `home_upgrade_cli.rs` (refusal names the remedy, upgrade, then the receipt
  projects), `structured_receipt.rs` (default-chart EU goods: legs, recompute,
  312/305/307), domain unit test for the extension predicate.
- `scripts/ci-local.sh` green.

## Review

`review.md` (`/llm-review`; `gpt-6-astra` finder, `claude-fable-5-2` focused
verifier on the replacement boundary) and `assessment.md`: one confirmed
should-fix (stale doc comment), fixed. No follow-up issue.
