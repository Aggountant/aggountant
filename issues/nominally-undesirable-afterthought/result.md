# Result: nominally-undesirable-afterthought

A home written by an older `init` that lacks a bundled masterdata file now gets
a `MASTERDATA_CORRUPT` that names the remedy:

```
MASTERDATA_CORRUPT: active chart has no readable VAT role metadata at
<home>/masterdata/reference/vat-account-roles.json: No such file or directory
— the home lacks bundled masterdata an older `init` did not write
(vat-account-roles.json); run `aggountant home upgrade --dry-run` to see the
plan, then `aggountant home upgrade --apply`; do not restore or write the file
by hand
```

- `GitMasterDataRefStore::open` (`crates/ledger/src/store/git_masterdata.rs`)
  asks the upgrade planner (the same dry-run `home upgrade` shows) when it
  fails with `MASTERDATA_CORRUPT`. The hint is added only when the plan would
  create a chart snapshot or the VAT role file, the two files `open` needs; a
  file the upgrade would refuse (`unavailable`), a broken file that is present,
  or a hand-edited chart gets no hint.
- `skills/aggountant-init/SKILL.md` no longer names `MASTERDATA_CORRUPT` as a
  trigger for `home upgrade`.
- Tests: `missing_bundled_vat_roles_name_home_upgrade_but_a_hand_edit_does_not`
  (ledger) and the `chart list` step of `home_upgrade_plans_applies_and_is_idempotent`
  (CLI).
