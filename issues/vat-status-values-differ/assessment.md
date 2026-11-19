# Assessment of the vat-status-values-differ review

Assessed against the working tree on `f46a5b0`; both fixes are in the slice's
commit.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Non-zero VAT relief ignored by the comparison | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX (Rule 5: data integrity is never filtered out) — applied |
| F2 | Amounts under unread top-level keys unseen | CONFIRMED | RARE | NEUTRAL | NONE | MED | FIX (Rule 5: data integrity; also makes the code match its stated rule) — applied |
| D1 | Net tax not compared | CONFIRMED | COSMIC_RAY | WORSENS | MINOR | HIGH | DROP (Rule 1d: restates the 308 rule in the broker) |
| D2 | >28-decimal fractions round | CONFIRMED | COSMIC_RAY | NEUTRAL | NONE | HIGH | DROP (Rule 1b) |
| D3 | Duplicate JSON keys | CONFIRMED | COSMIC_RAY | NEUTRAL | NONE | HIGH | DROP (Rule 1b) |
| D4 | `SubmittedDate` compared lexically | CONFIRMED | RARE | NEUTRAL | NONE | MED | DROP (Rule 1b; pre-existing, untouched) |

FIX: 2   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 4

- F1 and F2 are rare (a 2024 period with relief filed some other way after an
  uncertain API filing; an answer outside the published schema), but a false
  match would record another return as ours, which the repository treats as
  always real. Both fixes are a few lines inside the same function and are
  tested.
- No follow-up issue was filed: the confirmed findings are fixed in this
  slice and the drops do not pass the filing bar.

## Simplicity

- Reused: `api_field` and `VatReturnField::ALL` name the fields and their
  return numbers, so the comparison has no second field table; the existing
  `FiledVatReturn` type, `judge` as the one rule for status and reconcile,
  and the mock's existing GetFiledVATReturn fixture.
- Repetition removed: `semantically_matches_attempt` is now
  `processed_differences(..).is_empty()`, so "matches" and "which values
  differ" cannot drift apart. `normalize_numbers` and `has_nonzero_amount`
  are gone; one `amount` parser serves every value.
- Left out on purpose: rate-row arrays and order-independent row matching
  (the published schema has no arrays; the rates are named keys, so an array
  is reported as a difference instead of being interpreted), comparing the
  net tax, and any change to how the receipt or the submission date is judged.
