# Simplicity review: camt-event-identity

## Reused

- **The natural key is the event key.** The parser's
  `bank-import:holvi:<iban>:<ref>` already identified a transaction across
  exports, and the fiscal store's `idempotency` table already makes it unique.
  No second key was introduced, and that uniqueness is also what guarantees a
  voucher the previous import sealed is never sealed again.
- **One seal path.** `commit` and the batch both run `seal_in_tx` (idempotency,
  period gate, number allocation, chain CAS), so the batch inherits the gapless
  numbering instead of restating it. The content checks the seal made before
  its transaction moved into `check_seal_content`, which the single seal and
  the batch share.
- **One resolution for preview and seal.** `commit_bank_batch` runs the same
  code with `write = false` and rolls back, so the preview cannot drift from
  the seal. The CLI resolves once without writing before it stores the
  statement, which is what keeps a refused batch from leaving an artifact.
- **The existing fingerprint, preparation hash and CLI output.**
  `BankTransaction::source_fingerprint` (v1) and `import_preparation_hash` are
  unchanged; the seal output keeps `created_count`, `replayed_count` and
  `sealed`, with the outcome added to each row.

## Repetition removed or left

- `validate_seal_with_pending_artifact` lost its only caller and is gone, with
  the pending-artifact parameter of `validate`.
- Left: `Ledger::validate` and `check_seal_content` both check accounts, the
  mistag rule and the attachment. `validate` has no policy set and serves
  `voucher validate`; merging them would thread an optional policy set through
  the primitive for no gain.
- Left: a batch-sealed voucher carries its fingerprint both in
  `vouchers.source_fingerprint` (the existing provenance column, which makes a
  later fingerprint-less `voucher seal` of the key a conflict) and in
  `bank_event` (which must also hold backfilled fingerprints, and sealed rows
  must not be updated).

## Left out on purpose

- No generic "source event" abstraction for other importers: there is one.
- No re-import with an optional assignment for a booked transaction, and no
  acknowledgement input to pass a changed bank fact: both are product decisions
  (see the review's discussion item).
- No separate backfill command or eager migration step: the fiscal migration is
  pure DDL because the ledger cannot parse camt (`import` sits beside
  `ledger`); the backfill runs inside the next import's own transaction.
- No reversal handling and no automatic assignment (out of the issue's scope).
