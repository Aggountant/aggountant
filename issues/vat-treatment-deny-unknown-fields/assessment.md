# Assessment of the vat-treatment-deny-unknown-fields review

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | ReceiptVatProposal::NoVat drops members on capture-receipt stdin | CONFIRMED | RARE | NEUTRAL | MINOR | HIGH | SPIN-OFF — filed as `receipt-vat-proposal-no-vat-unknown-members` (untriaged, unlaned) |

FIX: 0   FIX (with care): 0   SPIN-OFF: 1   DISCUSS: 0   DROP: 0

- F1 is real silently wrong bookkeeping in principle. The likelihood is lower
  than in the original issue because the `type` says `no-vat` outright. It is
  outside this worker's file set (`capture_receipt.rs`), so it is a spin-off,
  not a fix here. The issue body stands alone.
- The verifier's notes on the array form and on duplicates through
  `serde_json::Value` are not drops and reach no production path. Not filed.

## Simplicity

- Reuses serde's `deny_unknown_fields`. The only added code is the
  `deny_members` helper, which closes serde's unit-variant gap without changing
  the Rust enum (`NoVat` stays a unit variant, so no call site changes).
- No wire-format mirror enum. A parallel `Deserialize` type would repeat the
  variant list.
- The helper stays private in `domain` while it has one user. F1 would be its
  second user, so that issue says to give it one owner instead of copying it.
- Deliberately left out: tightening `VatExpansionTreatment`. It is read from
  stored vouchers, and refusing members there is a stored-data change.
