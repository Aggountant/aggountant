# Assessment of the vat-settlement-shape-hides-untagged-reductions review

| # | Finding | Classification | Outcome | Notes |
|---|---|---|---|---|
| F1 | No roles: bank leg of a net payment passed as refund interest, input leg settled | CONFIRMED (failing test) | FIX | Data integrity: fail-open where the spec demands fail-closed. `settles_vat` returns no settlement for any non-VAT leg when roles are absent |
| F1b | No roles: untagged payment nets an untagged sale to a `reconciled` period | CONFIRMED (failing test) | FIX | Data integrity. Status forced `unreconciled` while `settlement_roles_missing` is non-empty; matches the issue ("the period is unreconciled") |
| F2 | Account kinds from active chart, not voucher snapshot | CONFIRMED but pre-existing | DROP | Not introduced here; fails toward blocking, not omission; needs a chart edition that drops a used account |

No confirmed data-integrity finding remains open after the fixes.

## Simplicity

The fixes add one early return and one status condition; no new type, field or
path. The settlement-role table, classifier call path, upgrade machinery and
existing acknowledgement are reused as the implementation's `result.md`
describes.
