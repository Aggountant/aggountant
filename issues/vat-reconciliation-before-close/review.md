# Review: vat-reconciliation-before-close

- Finder: gpt-6-astra (`--task review`, diff of the uncommitted slice)
- Verifier: claude-fable-5-2 (one batched kill attempt on the two surviving candidates)
- Model calls: 2. Rigor: routine (read-only preview; the filing paths were
  checked to still require a sealed period).
- Scope: `crates/process/src/file_vat.rs`, `crates/process/src/vat_reconciliation.rs`,
  `crates/cli/src/main.rs` (help and human line), the two test files,
  `skills/capture-receipt`, `skills/vero-fi`, and the user guide pages changed.

## F1 (should-fix): `fileable: true` beside known-incomplete figures

- Location: `crates/process/src/file_vat.rs` (`fileable = period_state == Sealed`).
- Claim: a sealed, unreconciled period reports `fileable: true` although
  `file-vat start` refuses it without an acknowledgement.
- Local check: a sealed unreconciled period has two legitimate routes, the
  acknowledgement (`require_reconciled` accepts it) and the manual OmaVero
  filing; the preview cannot see an acknowledgement, so `sealed && reconciled`
  would call an acknowledgeable period unfileable.
- Verifier: refuted as a defect; the doc comment "whether these figures can be
  filed" overstated the field.
- Outcome: doc comment reworded to "whether the period is in a fileable state;
  whether the figures need action is in `attention`".

## F2 (should-fix): Closing preview prescribes refused operations

- Location: `crates/process/src/file_vat.rs`, the non-sealed `filing_instruction`.
- Claim: Open and Closing shared one instruction ("correct with a voucher in
  this period, close it with `run start close-period`"). In Closing a bare seal
  is refused (`PERIOD_CLOSING`, `crates/ledger/src/store/sqlite_fiscal.rs`) and
  `begin_close` only transitions from Open; the executable act is answering
  `ApproveClose`.
- Local check: confirmed by reading both refusals.
- Verifier: survives.
- Fix: a separate Closing instruction (reject the close to reopen before a
  correction, otherwise approve it), asserted in the CLI test.

## Dropped

None beyond F1's reclassification.

## Coverage and residual risk

Read: the diff, `derive_verified_sealed_filing` callers (start and the Run's
re-derivation both still go through the sealed check), `reconcile_period`,
`period_vouchers`. Tested: process and CLI reconciliation tests for open,
closing and sealed states, read-only property of the open preview, and that
`file-vat start` still refuses an open period. Not covered: the month-close
approval gate content, which `virtually-tidy-owl` owns.
