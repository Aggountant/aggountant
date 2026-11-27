# Assessment of the review

| Finding | Classification | Outcome | Why |
|---|---|---|---|
| F1 payee IBAN letters | CONFIRMED | FIX | A mistyped account would be shown as the payment destination; the fix is one condition. |
| F2 superseded details at approval | CONFIRMED | FIX | Real window (start, notice the wrong reference, correct it, approve). The Run's proposal is what the user pays from after filing, so a stale reference is the misallocation the feature exists to prevent. Refusing before the send costs a restart. |
| F3 leading zeros | CONFIRMED | FIX | Low likelihood, but refusing leading zeros is the standard's canonical form and makes the duplicate guard exact at no cost. |
| RF/domestic same digits | CONFIRMED | DROP | No source gives both spellings; the user would type one reference twice in two forms. |
| `require_payment_route` wording | CONFIRMED | DROP | Reached only by Runs from earlier builds, which carry no reference. |

## Simplicity

- Reused: `model::Iban` for the checksum, the existing `payment_body` JSON and
  its approval hash (new keys only when present, so older Runs hash and read
  as before), the existing `ProcessError` code-plus-message pattern, the
  atomic write pattern of the other process files.
- One rule, one place: which reference a VAT payment uses is
  `TaxPaymentDetails::vat_reference`, read by the start, the preview and the
  approval check; the account and reference are resolved only in the engine
  (`resolve_vat_payee`). The CLI module only parses and prints.
- Left out: hard-coded Tax Administration accounts (vero.fi does not publish
  them, and one changes on 1 December 2026), a payment-method switch (the
  general-or-tax-specific rule is enforced by refusing a mix instead), a
  transfer-tax reference, and any use of the prepayment or arrangement
  references beyond storing them.
- No store migration: a new plaintext file under `masterdata/`; a home
  without it behaves as before except that a first VAT return asks for the
  reference.
