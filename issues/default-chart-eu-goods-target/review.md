# Review: default-chart-eu-goods-target

- Skill: `/llm-review`, rigor **routine**, surface treated as high-risk
  (in-place rewrite of company masterdata that sealed vouchers resolve against).
- Scope: commit `ec9e5f9` against `3fe66a0` (bundled VAT roles,
  `crates/domain/src/vat/{roles,expand}.rs`,
  `crates/ledger/src/store/git_masterdata.rs`, the retired role file,
  `crates/cli/src/home.rs`, tests, upgrade guide, capture-receipt skill,
  `crates/ledger/AGENTS.md`).
- Finder: `gpt-6-astra`, one call (diff plus the issue and the ledger rules).
  Answer: "No findings."; it confirmed the retired bytes equal the existing CLI
  fixture and that the bundled table changes only by the `eu-goods` target.
- Verifier: `claude-fable-5-2`, one focused false-negative call on the
  replacement boundary only: can `home upgrade --apply` (a) overwrite a file the
  company edited, (b) change how a sealed voucher verifies or a filed period's
  figures, (c) leave a half-written or unopenable home. It ran the upgrade
  tests and answered "cannot happen" for all three with code citations
  (byte-equality gate re-checked before the rename; targets are read only
  through `reverse_charge_target` / `domestic_input_target`; every other field
  required equal; temp-file-then-rename; creates before the replacement, and
  every intermediate state opens).
- Two model calls in total.

## Confirmed findings

### R1 (should-fix): stale doc comment on `Ledger::upgrade_masterdata`

- Location: `crates/ledger/src/lib.rs`, doc comment of `upgrade_masterdata`.
- Claim: it still said "Never rewrites an existing file", which the slice
  makes false for an unedited earlier-release VAT role file.
- Evidence: verifier read; local read confirmed.
- Fix: comment states the one sanctioned rewrite.

## Dropped (2)

- Apply proceeds even when the planned masterdata check fails, so a home that
  already did not open for an unrelated reason still gets the role file
  replaced: pre-existing behaviour for creates, reported through
  `masterdata_check`, and the replacement is itself validated against the
  active chart. Not a regression.
- A crash between the temp write and the rename leaves a hidden temp file in
  `reference/`: same as the existing `create_new_file`; nothing lists that
  directory, and a re-run converges.

## Coverage

Read and tested: the upgrade plan/apply paths, `only_adds_reverse_charge_targets_to`,
the refusal message, CLI envelope, the default-chart EU goods receipt through
seal, recompute and filing figures (312/305/307). Workspace tests and clippy
green. Not tested: real crash injection during the rename.
