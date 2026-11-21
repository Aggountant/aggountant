# Assessment of the vat-replacement-return review

Assessed against `ce6c1e9`; the accepted fixes are in the following commit.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Coverage of a correction is decided by the filing event's timestamp | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | MED | FIX (Rule 5) — applied |
| F2 | `complete-manual --replacement` appends a filing on every call | CONFIRMED | OCCASIONAL | NEUTRAL | NONE | HIGH | FIX (Rule 5) — applied |
| F3 | An interrupted Run is judged by the latest filing, not the one in its slot | CONFIRMED | RARE | IMPROVES | NONE | MED | FIX (Rule 5: RARE, but the fix improves readability) — applied |
| F4 | Replacement preflight does not check that the processed answer is the return in force | UNABLE_TO_VERIFY | - | - | - | LOW | DROP (Rule 1a: unable to verify, verification infeasible) |
| F5 | Status can call a same-day pending replacement `processed_values_differ` | CONFIRMED | RARE | NEUTRAL | NONE | LOW | DROP (Rule 1b: RARE, no readability gain) |

FIX: 3   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 2

- F1: data integrity, so it is fixed whatever its likelihood. Coverage is now
  decided in `process` (`unfiled_vat_corrections` in `file_vat/status.rs`): the
  cut-off is when the Run holding the in-force request began dispatching, not
  when the filing was recorded. The Ledger only lists corrections sealed after
  a given instant (`vat_corrections_sealed_after`). Test:
  `a_correction_sealed_while_a_replacement_was_unresolved_is_still_due_after_adoption`.
- F2: `replace_vat_filing` replays a manual replacement whose channel and
  evidence reference are those of the filing in force. Tests:
  `a_manual_replacement_entered_twice_is_recorded_once` (ledger) and the
  repeated call in `a_replacement_made_in_omavero_is_recorded_with_its_evidence`.
- F3: `reconcile` looks at the filing that followed the ones the Run started
  from; only the Run's own `sent_hash` is "fiscal landed", anything else closes
  the Run as superseded. This restores the result first-return Runs had before
  the slice. Test: `an_interrupted_first_return_never_claims_a_later_replacement`.
- F4: the check is as described, but whether GetFiledVATReturn/v2 ever answers
  `Processed` with the previous return while a newer one waits is not in the
  developer portal text and cannot be observed without the production service.
  A guard built on unobserved authority behaviour is not added. If it happens,
  the Run stays unresolved and the manual route settles it; nothing is sent.
  Worth a look at the first real replacement.
- F5: needs a return submitted, processed and replaced within one Finnish date,
  and a status read while the replacement is pending. The message still ends
  "do not file this period again", nothing is written, and the next read is
  right.

No follow-up issue was filed: the three confirmed findings with real impact are
fixed in this slice, and neither dropped finding is an observed occurrence or a
self-contained problem with credible impact. Nothing here is a multi-user
finding.

## What the person judging the slice should know

- **The replacement has never met the real API.** Everything is tested against
  the scripted seam and the mock. The API facts (reason codes, receipt, what
  GetFiledVATReturn shows) were read from the developer portal on 2026-11-21
  and are recorded in `crates/broker/AGENTS.md`.
- **The closing rule for a replacement is weaker evidence than for a first
  return.** Two unchanged answers a bank day apart close the Run, but that is
  also what the authority shows while an accepted replacement waits. A re-sent
  replacement carries the same values, so the return in force ends up the
  same: the books record one replacement, and the authority may have received
  it twice.
- **A manual replacement covers everything sealed before it was recorded.**
  There is no request to compare, so the operator's evidence is the only record
  of what was filed.
- **Whether to attribute a correction to the filed period or to correct a
  minor error in the current return is the user's decision** and must be made
  before sealing. There is deliberately no way to change it afterwards.

## Simplicity

- Reused: the `file-vat` Run itself. A replacement is the same workflow, gates,
  state file, journal, `reconcile` and `status`; the differences hang on one
  optional `replacement` block in the Run state and one question, "has this
  Run's filing reached the books", which replaced the `vat_filed` checks
  (`filing_landed`). No second Run type, unlike the travel report's correction,
  because the VAT recovery machinery (bank-day rule, interrupted dispatch,
  status) would have had to be duplicated.
- Reused: the period transition chain for the filing history. No new table and
  no migration: a replacement is one more `Sealed→Sealed` event, and the three
  `last_filing_*` store queries became one `period_filings`.
- Reused: the optional, skip-when-absent field pattern of the voucher body
  (`buyer_vat_id`, `vat_expansion`) for `corrects_vat_period`, so every
  existing sealed voucher keeps its bytes and hash.
- Removed repetition: the first return and the replacement are recorded by one
  `record_filing`; the previous code had the recording in three places.
  `vat_receipt_of_run` and `filing_run_with_receipt` now match by `sent_hash`
  instead of a "several candidates with identical bytes" rule.
- One rule, one place: which return a voucher belongs to is
  `VoucherBody::in_vat_period`, used by every VAT derivation in `domain`.
- Left out on purpose: a post-seal attribution command (unsafe once the posting
  period is filed); computing or paying the difference of a replacement (the
  earlier return may be paid or not; the tax account knows); a generic
  "correction recorded in the e-service" abstraction shared with the Incomes
  Register (the two channels share the decision, not the data); storing the
  filed figures in the fiscal store.
