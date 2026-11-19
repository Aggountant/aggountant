# Review: voucher-validate-seal-envelope

- Skill: `/llm-review`, rigor **routine**, risk class data integrity: the
  change reads the idempotency table and the period gate, and a dry-run that
  disagrees with `seal` misleads the agent about what will be booked.
- Scope: `f41cc53` against `1a1f6d0` (`crates/cli/src/main.rs`,
  `crates/ledger/src/lib.rs`, `crates/ledger/src/store/{mod,sqlite_fiscal}.rs`,
  tests, guide, skill), then the fixes in `aca8a35` and the working tree.
- Finder: `gpt-6-astra`, one call on the first commit's diff plus the issue
  and `sqlite_fiscal.rs`. Verifier: `claude-fable-5-2`, one focused
  false-negative call on the fixed diff (commit refactor unchanged? dry-run
  parity with `seal`?). Two model calls in total.

## R1 (must-fix): the envelope dry-run skipped the period gate

- Location: `crates/ledger/src/lib.rs`, `Ledger::validate_seal` (unused key
  returned straight after `validate`); `crates/cli/src/main.rs`
  `VOUCHER_VALIDATE_INPUT_HELP` claimed a closed period was invisible to a
  dry-run.
- Claim: `seal` refuses a voucher in a Closing or Sealed period, or outside
  the fiscal calendar, in `SqliteFiscalStore::commit`; the dry-run did not
  read that state although it is plain database state.
- Evidence: local read of `commit` (calendar, sealed year, month and quarter
  rows after the idempotency check). Realistic: a late receipt dated in an
  already closed month validates `ok: true` and seal refuses it.
- Fix: the gate moved into `posting_gate_refusal(conn, gate)`, used by
  `commit` (which still logs the refusal) and by a new read-only
  `FiscalStore::posting_gate_refusal`. `validate_seal` checks it after the
  idempotency check, as `commit` orders them, so a replay into a period closed
  since still replays. Help text corrected. Test
  `validate_seal_sees_the_period_gate_and_replays_like_seal`
  (`crates/ledger/tests/period.rs`). Verifier confirmed the refactor leaves
  `commit`'s codes, messages, rejection targets, ordering and transaction
  handling unchanged.

## R2 (should-fix): misleading conflict message for a source-bound key

- Location: `crates/ledger/src/lib.rs`, `Ledger::validate_seal`, conflict
  branch.
- Claim: when the importer sealed the key with a source fingerprint and the
  envelope has identical content, the dry-run said "different content" with
  two equal hashes and advised a new natural_key, which would book a
  duplicate of a voucher already in the books. `seal` itself says the
  projected content is unchanged.
- Evidence: verifier, checked locally against the `bank:event-1` test case.
- Fix: a separate message ("already in the books … do not seal it again
  under a new natural_key"); the test asserts it.

## Also fixed

- A doc comment of `evaluate_period_gate` had been carried onto the new
  helper by the refactor (verifier); moved back.

## Dropped

None. The finder returned one candidate; the verifier one finding plus the
doc slip.

## Coverage and residual risk

Read: the diff, `commit` and its gate, `seal_with_provenance`'s pre-commit
checks, `build_draft`, `VoucherDraft` deserialisation. Verifier traced that
`validate`'s extra VAT-warning work cannot fail where seal succeeds and that
`build_draft` never attaches a VAT expansion, so seal's expansion checks
cannot fire on this path. Not covered: the race between a dry-run and a later
seal (single-user v1). The CLI envelope-in-closed-period path is tested at the
ledger level only.
