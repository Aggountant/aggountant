---
created: 2026-11-10
updated: 2026-11-10
type: improvement
status: open
priority: normal
lane: parkki
lane_seq: 10
---

# Multi-user considerations register (single-user now; hardening deferred)

## Purpose

aggountant is a single-user system: one trusted user or local agent on one
machine, one data home per company. That is an explicit scope decision, not an
oversight. This issue is the register of every point that would need another
look if aggountant ever moves toward several users, concurrent processes or a
less-trusted environment. It stays open and accumulates entries; nothing here is
a bug under the single-user assumption.

## How entries land here

When a review finds something that matters only under several users,
concurrency or a hostile trust boundary, record it here instead of fixing it:
one bullet stating the point, why it is safe today, what to do if the model
changes, and a one-line source (which review or slice, and when). Data
integrity is never a multi-user concern; a finding that can make single-user
bookkeeping silently wrong is a bug and gets its own issue.

## Register

- **Filing and closing authority are not fenced.** Any run may file a sealed,
  unfiled period, and the closing-authority token is process memory plus a
  persisted owning run id. Double filing is still impossible, because the
  filing compare-and-set re-checks the period state. If the trust model
  hardens, decide a fencing generation for both authorities together.
- **Broker outbox is single-writer.** `submit` loads an entry and
  `begin_attempt` inserts it in separate steps on a fresh connection per call,
  so two concurrent submits of the same effect could both pass the load check.
  Today the `file-vat` Run holds the process lease, and the primary key bounds
  any race to a loud error. If concurrency appears, fold the check into one
  transaction and share one connection.
- **Personal data at rest and pseudonymous linkage.** The employee store's
  cipher is a plaintext placeholder; its boundary is isolation plus
  `.gitignore`. The process store's `employee_id`, amounts and payload hashes
  are pseudonymous personal data. Revisit (with real envelope encryption) when
  a home stops living on one trusted machine.
- **Knowledge approvals and year-end signing have no reviewer identity.** The
  `ApproveKnowledgePR`, `ApproveStatements` and `SealYear` gates prove that a
  human approved, not which one. A firm with a preparer and a signing
  accountant would need reviewer identity and signing authority on these gates.
- **Disclosure isolation.** A co-located hostile process could read an
  unprojected source document before its disclosure projection is built, and
  projections are tenant-scoped only because there is one tenant.
- **Reads strict-decode but do not re-walk the hash chain.** `period_vouchers`
  and `year_vouchers` decode each voucher's canonical bytes but do not verify
  the chain; `year_vouchers` also reads the chart outside the voucher snapshot,
  and opening balances now fold every earlier year. A tampered store or a
  concurrent chart edit could skew derived figures. If the store becomes
  untrusted or shared, derive from a fully verified snapshot and read the chart
  atomically with it.
- **Broker `409` recovery trusts the authority's key-to-payload binding.** The
  adapter treats a `409` that echoes our idempotency key (the `natural_key`) as
  proof our request was processed. Two clients, or two effects, deriving the
  same `natural_key` with different payloads would record a false recovery. A
  multi-client deployment needs a payload digest in the echo, a remote key
  derived from `effect_key`, or an outbox that rejects a `natural_key` bound to
  more than one `effect_key`.
- **Adjustment attribution has no capability or reason.**
  `Ledger::attribute_adjustment` takes a bare run id and actor, and will move
  any sealed voucher's statement year, not just depreciation, accruals and
  corrections. VAT stays in the posting period, so attributing a VAT-coded
  voucher breaks the direct reconciliation between annual statements and VAT
  periods. If the model hardens: a close-year capability, a typed reason, a
  decision on VAT-coded vouchers, and a reconciliation report.
- **`vat_expand` does not reject account-role collisions.** A caller can pass a
  net account equal to the contra account, or to a generated VAT account, and
  the result still balances and seals. Validate account roles against the
  pinned chart if accounts ever come from a less-trusted caller.
- **Opening an existing home is check-then-act.** The existing-home preflight
  and the subsequent `Ledger::open` are not atomic against another process
  removing or swapping paths. Reconsider a true no-create open for concurrent
  operation.

(Append new entries above this line.)

## When to act

When a concrete multi-user or multi-tenant requirement appears, or when a
deployment stops being one trusted user on one machine. This register then
becomes the checklist for a hardening epic.
