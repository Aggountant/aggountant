# crates/broker — rules the code does not show

Read `crates/AGENTS.md` first. The broker is the only route to an outside
system: adapters, the outbox, per-attempt provenance and the credential vault.
A filing to a tax authority has legal effect and cannot be taken back, so the
rules here lean towards stopping and asking rather than retrying.

## Every adapter

- The intent is written to the outbox before the network call, and every
  attempt keeps its own provenance, including the hash of the bytes sent.
- Do not pretend the remote side is idempotent. An adapter may rely on remote
  idempotency or a "duplicate" status only when the outside contract documents
  it; otherwise a timeout, connection loss or server error is `OutcomeUnknown`
  and is resolved by safe status queries or by a person.
- Secrets never reach the process journal, a payload, provenance or git. They
  stay behind `CredentialHandle`, which deliberately does not implement
  `Serialize`.
- Consult advice never becomes a ledger input.

## Vero API VAT filing

The VAT channel is the direct Vero API (`GetVATPeriods/v1`,
`FileVATReturn/v2`, `GetFiledVATReturn/v2`) under the user organisation's own
software registration, software key and mTLS certificate. Acting for another
organisation adds a Suomi.fi authorisation token (`Vero-AuthorizationToken`);
the adapter passes one when configured, but nothing fetches it.

- **The approval binds the final API request**, not the internal numbered field
  map. The internal field 308 (net VAT) is not sent; the authority computes it.
- **Only HTTP 200 with a receipt is an accepted filing.** `UniqueIdentifier` and
  `AcceptedTimestamp` prove receipt, not final processing. A 400 is a final,
  correctable error shown to the user.
- **No automatic resend.** A 500, timeout or dropped connection is
  `OutcomeUnknown`. Recovery reads `GetVATPeriods` and `GetFiledVATReturn` and
  accepts the remote return as ours only when its values match the approved
  request unambiguously. `Missing` right after sending does not prove the
  return was lost, and a delay before OmaVero shows the return is not a reason
  to send again: the Run stays in `NeedsAttention` with the deadline and the
  manual options.
- **After an accepted return, every later filing for the period is a
  replacement return** with all values and a reason, and needs a new explicit
  approval. A blind second "first" filing would silently become an unintended
  replacement.
- **Filing is not paying.** `ApproveFiling` never approves a payment; a payment
  is a separate approval, effect and receipt.
- **The manual fallback (OmaVero, Ilmoitin.fi) is never started silently or by
  browser automation**, and not before an uncertain API outcome has been
  resolved.
- The three portal verification endpoints are fixed constants used only by
  `vero-fi verify-portal`; `file-vat` cannot select them, and no command takes a
  URL, target or mode for them.
