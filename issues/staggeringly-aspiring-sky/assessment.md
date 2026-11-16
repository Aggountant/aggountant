# Assessment of the staggeringly-aspiring-sky review

| # | Finding | Classification | Outcome | Notes |
|---|---|---|---|---|
| V1 | 200 with a two-element JSON array body read as a receipt | CONFIRMED (pre-existing, no known source) | FIX | Cheap and inside the function this change owns: the body is parsed once to a `Value` and only an object can be a receipt, which also removed one of two parse sites. Test case added to both adapters' receiptless list |
| V2 | Predicate comment claims `MALFORMED` appears only in older journals | CONFIRMED | FIX | Comment now says which current paths still journal `MALFORMED` and why treating them as possibly accepted is safe; recorded under decisions in `result.md` |

No confirmed data-integrity finding remains open. Neither finding could make a
delivered filing look failed or an uncertain one look settled after the fixes.

## Simplicity

- Reused: the existing `AdapterError::AmbiguousDelivery` → broker `Unknown` →
  journal `OUTCOME_UNKNOWN` path that timeouts and 5xx already take. No new
  error variant, journal event or outcome state.
- Repetition removed: the two adapters had their own copies of the 200-arm
  parse and completeness check (and disagreed on `trim`); they now share
  `file_vat_return_success`, and `FileVatReceipt::decode` uses the same
  `is_complete`. `status.rs` and the `complete-manual` guard share
  `filing_outcome_may_be_accepted` instead of two rules. `accepted_body` lost
  its only caller and was removed.
- Left out on purpose: refusing a new Run while a filing is unknown (the next
  issue in the lane, which can call the same predicate), a distinct journal code
  for "2xx without receipt" (the outbox attempt keeps `HTTP_<status>_WITHOUT_RECEIPT`
  as the cause), and a 401/403 arm in the scripted adapter (the issue says error
  statuses keep their current meaning).
