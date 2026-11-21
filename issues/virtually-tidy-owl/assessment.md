# Assessment of the virtually-tidy-owl review

Assessed against the branch head after the review fixes.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | The close required the previous fiscal year to be sealed | CONFIRMED | COMMON | NEUTRAL | MINOR | HIGH | FIX — applied |
| F2 | A resumed start skipped the pre-commit content check | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX (one line, keeps one invariant on both paths) — applied |
| F3 | `ready` did not cover the ledger's coverage rules | CONFIRMED | RARE | NEUTRAL | NONE | HIGH | FIX (documentation of the field) — applied |
| D1 | Resume seals without rechecking content | INCORRECT as a defect | RARE | — | — | MEDIUM | DROP |

FIX: 3   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 1

- F1 was a regression the slice introduced, and the worst kind for this
  feature: every company keeping its previous year in aggountant could not
  close January until the year-end close, which blocks the January VAT return.
  Fixed with a fallback that builds the content from the year's own vouchers
  and says the opening position is missing (`balance_sheet.opening_note`).
  Tested.
- F2: the window is a crash between journalling the transition and the fiscal
  commit, but the fix costs one call and keeps "content failure leaves the
  period Open" true on both start paths.
- F3: a start refused by the ledger after `ready: true` changes nothing and
  says why (`PERIOD_TRANSITION_CONFLICT`); the states that trigger it (months
  and quarters in one year, a quarter in a non-calendar fiscal year) are rare
  and already documented as limitations. Reproducing the ledger's checks in the
  preview would state the same rule twice; a read-only ledger check is the
  right shape if it is ever needed, and nothing asks for it now.
- D1: dropped with the reason in `review.md`: the approved act (sealing the
  period's own vouchers, frozen by Closing) is what gets sealed, and refusing
  after an answered gate would strand the Run.

No confirmed data-integrity finding remains open. Nothing was filed.

## Simplicity

- Reused: the ledger's `fiscal_year_data` projection, `compute_statements` and
  `tase_erittelyt` (the same calls as the monthly report, so the figures
  agree and no second statement rule exists); the existing gate snapshot hash,
  extended by one field instead of a second binding; the obligation store's
  `open_for_period`; the `RunView` serialised unchanged with fields added by
  `#[serde(flatten)]`.
- Repetition: the content is built in one function used by preview, start,
  resume, gate display and approval. Per-account balances come from
  `tase_erittelyt` rather than a new sum. The legacy hash is the same function
  with the content field absent.
- New module `close_period.rs` rather than growing `lib.rs`, which is the
  collision hotspot for this lane.
- Left out on purpose: persisting the content in the Run (Closing freezes the
  period, so rebuilding is equal unless the books changed, and then the
  approval must refuse anyway); a workflow version bump (in-flight Runs keep
  closing under their old hash); content on answered gates; a VAT summary in
  the content (`file-vat preview` is the VAT approval's own content); voucher
  number lists (count and range).
