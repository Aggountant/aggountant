# Assessment of the camt-creditor-reference review

Assessed against the slice's final diff (the fix is in the same commit).

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Unreadable tax payment details blocked parse and preview | CONFIRMED | RARE | NEUTRAL | MINOR | HIGH | FIX (confirmed; optional evidence must not block an import) — applied |

FIX: 1   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: confirmed by reading the two call sites. A broken file needs a hand
  edit, so RARE, but the cost was a blocked import preview for an unrelated
  reason. Fixed with a warning instead of an error; tested.

No data-integrity finding remains open. No follow-up issue was filed.

## Simplicity

- Reused: `PaymentReference` for kind-specific validation and the canonical
  form (no second check-digit implementation); the existing `RmtInf` lookup
  in `parse_entry`; `read_tax_payments` and `TaxPaymentDetails` for the stored
  references; the existing passthrough pattern (optional field, outside
  identity, unreadable left out).
- Repetition: the comparison is one method, `TaxPaymentDetails::reference_kind`,
  and one CLI function, `tax_reference_match`, used by both parse and preview.
  `home_is_initialized` was extracted from `existing_home` so parse checks the
  same condition rather than restating it.
- Left out on purpose: any stored reference or schema change (the issue says
  no migration); matching references against invoices or open items (a later
  issue); reading a reference from `Ustrd` text; identifying the tax period
  (the reference cannot).
