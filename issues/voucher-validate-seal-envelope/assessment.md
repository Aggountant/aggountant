# Assessment of the voucher-validate-seal-envelope review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| R1 | Envelope dry-run skipped the period gate | CONFIRMED | COMMON | NEUTRAL | MINOR | HIGH | FIX — applied |
| R2 | Misleading conflict message for a source-bound key | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX — applied |

FIX: 2   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- R1: late receipts for a closed month are a normal occurrence, and the
  point of the slice is "validate the file you seal". Fixed by sharing one
  gate function between `commit` and the dry-run instead of writing a second
  copy. Bare-draft validation keeps its old scope (no period state) so its
  existing callers see no change; the help says so.
- R2: the wrong advice could lead to a duplicate voucher, which is
  data integrity. Fixed with a message that matches what happened.

No follow-up issue was filed.

## Simplicity

- Reused: `SealInput` and `build_draft` exactly as `voucher seal` uses them,
  `Ledger::validate` for the pre-commit checks, `draft_request_hash` for the
  idempotency identity, the store's `load_voucher`, and the gate code moved
  out of `commit` rather than restated.
- Repetition: the idempotency comparison (request hash, source-bound key) is
  now stated in `commit` and in `validate_seal`. Sharing it would mean moving
  the comparison out of the store's transaction into a helper both call; at
  two short conditions it was left as is, with a comment pointing at `commit`.
- Left out: period gating for bare drafts (would change an existing command's
  behaviour, not asked for), a separate `validate --seal` flag (shape
  detection is unambiguous because a bare draft's fields are closed), and a
  closing-authority dry-run for close runs.
- `UntaggedVatWarning` became `ValidationWarning` with optional
  `posting_index`/`account`/`voucher_number`; the serialised VAT warning is
  byte-identical to before.
