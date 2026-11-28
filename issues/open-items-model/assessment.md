# Assessment of the ADR-12 review (`review.md`)

`/assess-findings` over the two findings of `review.md`. Both are DESIGN
findings on `docs/site/adr-12-open-items.md`.

| ID | Finding | Confirmation | Likelihood | Readability | Architecture | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Partial-payment guidance (question 5) leaves the paid part offered again on the pay-now list | CONFIRMED | OCCASIONAL | IMPROVES | MINOR | HIGH | FIX (applied) |
| F2 | KILA 5.7 quoted without its "unless they carry required information" condition | CONFIRMED | RARE | IMPROVES | NONE | HIGH | FIX (applied; rule 1b does not drop it, the fix strictly improves accuracy) |

## Evidence

- **F1.** Outstanding is derived only from origin and settlement rows (ADR
  sections 1–2), and the pay-now list is "Approved, unpaid" with no live
  schedule (section 4). A receivable booked outside the claim changes
  neither, so the paid part stays offered. Partial payments are an ordinary
  slip (a mistyped amount), so OCCASIONAL. The fix reuses the
  `claimant_receivable` item question 6 already proposed and adds no table
  or mechanism: the claim's displayed amount is net of open receivable items
  naming it. The owner is `expense-claim-payment-matching`, whose note says
  so.
- **F2.** The condition is in KILA 5.7 verbatim. No decision rests on the
  unconditional reading, since section 4 argues from "it books nothing", but
  a citation should not drop its source's condition.

## Simplicity

The fixes add no structure. F1 merges the partial-payment and overpayment
receivables into one item kind instead of describing two mechanisms.
