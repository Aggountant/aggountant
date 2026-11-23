# Result: camt-import-manual-bank-vouchers

`import camt053 seal` no longer books a second voucher for a bank entry that a
voucher sealed outside the import (generic `voucher seal`) already books. It
stops instead, and the user binds the entry to that voucher or rules the
voucher out, in the same hash-bound preview and approval.

## Behaviour

- Before a new entry is sealed, the batch searches every voucher not sealed
  under the import's own key prefix. A voucher is a candidate when its natural
  key or description names the entry's bank reference as a whole token
  (evidence `bank_reference`), or when it posts the entry's amount on the
  entry's side of the assignment's bank account, with its posting or document
  date within 7 days of the booking or value date (`account_amount_date`). Any
  candidate stops the whole batch with `BANK_ENTRY_POSSIBLY_BOOKED`;
  `details.conflicts` names each voucher and its evidence. Nothing is written.
  An entry already booked (its event key in the books) is not searched.
- `existing_vouchers: [{bank_ref, bank_account, voucher_number}]` binds an
  entry to an existing voucher instead of an assignment. The voucher's bank
  leg must pass the same match, it must not be bound already and must not be
  an import voucher of another transaction, else `BANK_VOUCHER_MISMATCH`. The
  binding writes a `bank_event` and an `idempotency` row for the event key
  pointing at the voucher, plus the observation of the statement file; the
  sealed voucher, its hash and `voucher verify` are unchanged. Outcome
  `bound_existing_voucher`, counter `bound_count`. A later export with the
  same binding is a duplicate observation.
- `not_booked_by: [N...]` on an assignment rules out exactly those vouchers
  for that entry.
- A bound voucher stays a candidate for other entries, because one hand
  voucher can book several payments. A voucher is bound to one bank
  transaction only; for the other payments of such a voucher the way out is a
  correction voucher that reverses that part, then `not_booked_by`.
- Preparation hash: bindings and `not_booked_by` are bound by named fields
  written only when present, so an assignment-only input hashes as before.
- No schema change (fiscal stays v9).

## Docs

`--help` (input help), the bank-import guide page (en/fi) with the new section
"When bank transactions were recorded by hand before", the error-code tables
(en/fi), the capture-receipt skill, and `crates/ledger/AGENTS.md`.

## Tests

`crates/ledger/tests/bank_event.rs`: candidates by reference and by
account/amount/date, whole-token matching, `not_booked_by`, binding without
changing the voucher (repeat binding, later assignment, generic seal under the
key), refused bindings, a voucher booking two payments (one batch, two
batches, overlapping export), a transfer bound on one account.
`crates/cli/tests/import_camt053.rs`: the issue's reproduction end to end
(stop in preview and seal, bind, approve, overlapping export), ruling out a
voucher, inconsistent binding input.

## Review

`/llm-review` (finder gpt-6-astra, verifier claude-fable-5-2) and assessment:
`review.md`, `assessment.md`. Two confirmed findings fixed. A pre-existing
double-booking path for own-account transfers when both accounts' statements
are imported is filed as `camt-transfer-between-imported-accounts`.
