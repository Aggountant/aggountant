# Review: agent-surface-small

One review covers the three issues landed together in this change:
`nominally-undesirable-afterthought`, `voucher-vat-code-help` and
`tulorekisteri-profile-no-command`.

- Skill: `/llm-review`, rigor **routine**, with one focused verifier question
  on the high-risk boundary: `check_readiness` is the send-time gate in front
  of an Incomes Register filing, and a weaker gate could let a send go out
  under a profile that should have stopped it.
- Scope: commits `05701a7`, `b89bacb`, `a41dfc6` against `3fe66a0`
  (`crates/ledger/src/store/git_masterdata.rs`, `crates/domain/src/vat/`,
  `crates/cli/src/{input_help,main,tulorekisteri,vero_fi,wage}.rs`,
  `crates/broker/src/tulorekisteri.rs`, tests, guide and skills).
- Finder: `gpt-6-astra`, one call (the diff and the three issues). Verifier:
  `claude-fable-5-2`, one call, asked only whether the new gate is weaker than
  the old one for any input and whether `tulorekisteri configure` can write a
  production profile the gate then refuses. Two model calls in total.
- Risk class: authority filing gate; VAT help an agent follows when booking.

## F1 (must-fix): `configure --replace` dropped the employer_report block

- Location: `crates/cli/src/tulorekisteri.rs`, `configure` (the `replace`
  branch).
- Claim: with `--replace` the merge base was empty and the input cannot carry
  `employer_report`, so every replace deleted the block written by
  `file-wage employer-report configure`; `changes` was computed against `{}`,
  so even `--dry-run` did not show the deletion.
- Evidence: local read; the help documented the drop, but the block belongs to
  another command and losing it makes the employer's separate report fail with
  `EMPLOYER_REPORT_NOT_CONFIGURED`.
- Fix: `--replace` replaces only this command's settings and keeps a stored
  `employer_report` block that still parses; one that does not parse is
  dropped and shown under `changes`. `changes` is always computed against the
  stored file. Test: the replace step in
  `configure_merges_status_itemises_and_employer_report_survives`.

## F2 (should-fix): the vat_code help gave the wrong formula for margin codes

- Location: `crates/cli/src/input_help.rs`, `vat_code_input!`.
- Claim: the paragraph said every tagged posting's VAT is `round(base x rate)`
  and told the agent to book the VAT line at that amount; a margin code's VAT
  is extracted from within the margin, `round(margin x rate / (100 + rate))`
  (`crates/domain/src/vat/derive.rs`, `full_vat`).
- Evidence: local read of `derive.rs`; the margin tests pin the two amounts.
- Fix: the paragraph states both formulas.

## Verifier: the send-time gate

- Weaker gate: none found. The verifier tabulated the ten refusal conditions
  of the old `check_readiness` against the new `profile_blockers` /
  `credential_blockers`; each still refuses, and `Ok` is returned only with no
  blockers. Both call sites (`production_broker`, the adapter's `call`) remain.
- Configure writes production only when `evaluate_readiness` is ready, on the
  same typed profile it writes and the same credential path resolution
  (`tulorekisteri::credential_path`) that `file-wage`'s broker uses.
- Adjacent, pre-existing (not a regression): readiness checked the transport
  identity but not the record-signing key, so a bundle whose key signing cannot
  load (an EC key, or a PEM with a leading newline) read as ready and then
  every send failed before any network call. Fixed in this change because the
  new status command would otherwise claim ready: the key load moved into
  `tulorekisteri_xml::signing_key`, used by both signing and readiness
  (`TULOREKISTERI_CREDENTIAL_INVALID`). Tested in
  `readiness_is_itemised_with_owners`. The repository's own VAT mTLS test
  fixture has an EC key: the VAT transport accepts it, Tulorekisteri signing
  cannot, so the new CLI test had to switch to the RSA wage fixture. A bundle
  that works for VAT is therefore not proof that it works here.
- Noted, not a defect: with its own `credentials_file` the broker no longer
  reads the platform configuration, so a broken platform config no longer
  stops such a send. That file is not a readiness requirement.

## Dropped

None. Both finder candidates were confirmed locally.

## Coverage

Read: the whole diff, the three issues, `derive.rs`, the margin tests, the
old `check_readiness`. Tested: workspace tests, clippy, fmt and cargo-deny on
stable and 1.89 (`scripts/ci-local.sh`), the guide check, and the new CLI
tests for `voucher vat-codes`, `tulorekisteri configure|status` and the
`MASTERDATA_CORRUPT` hint. Outside scope: a real Incomes Register certificate
(the tests use the synthetic mTLS fixtures), and chart-specific mistag
refusals of a listed VAT code, which `voucher vat-codes` deliberately leaves to
`voucher validate`.
