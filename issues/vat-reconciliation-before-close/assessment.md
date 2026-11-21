# Assessment of the vat-reconciliation-before-close review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | `fileable: true` beside unreconciled figures | INCORRECT (as a defect) | COMMON | IMPROVES | NONE | HIGH | FIX (doc comment only) — applied |
| F2 | Closing preview prescribes refused operations | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX — applied |

FIX: 2   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: `fileable` describes the period, not the figures. A sealed unreconciled
  period is filed either with an acknowledgement or manually with corrected
  figures, and the blocking `VAT_ACCOUNTS_UNRECONCILED` item already says which
  action the figures need. Tying `fileable` to reconciliation would mislabel an
  acknowledgeable period. The field's doc comment now says this.
- F2: an agent following the shared instruction on a closing period would hit
  `PERIOD_CLOSING`, or try to start a second close. Closing now has its own
  instruction; the CLI test asserts it.

No data-integrity finding remains open; no follow-up issue filed.

## Simplicity

- Reused: the existing preview, `reconcile_period` and `attention_items`; the
  sealed-period check stays in one place (`derive_verified_sealed_filing`),
  now a thin wrapper over the shared derivation the preview calls directly.
  No second preview command, no separate draft type.
- Repetition removed: none needed; the state-dependent remedy is one `match`
  in `attention_items`, and the instructions one `match` in the preview.
- Left out: gate content for the month close (`virtually-tidy-owl`), and any
  change to the sealed `filing_instruction` wording
  (`vat-preview-filing-instruction` owns it).
