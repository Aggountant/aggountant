---
created: 2026-11-21
updated: 2026-11-21
type: feature
status: open
priority: high
provenance: other
provenance_detail: follow-up filed by the taskfleet worker implementing filing-correction-unrecordable
source_ref: taskfleet:01m7ysp1v4571vxqbtzzgxj77x/follow-up:vat-replacement-return
originating_run: 01m7ysp1v4571vxqbtzzgxj77x
originating_run_kind: spinoff
lane: vat-filing
lane_seq: 1
preparation: ready
---

# Replacement VAT return through file-vat for an already filed period

## Description

## Problem

aggountant cannot correct a VAT return it has filed. `file-vat` sends only a
first return for a period, and once the period is `vat_filed` every path
refuses: `file-vat start` and `file-vat complete-manual` both stop in
`Ledger::acquire_filing_authority` with `PERIOD_TRANSITION_CONFLICT` ("already
filed — filing is irreversible"). The refusal is loud, so nothing is recorded
wrongly, but the user has to make the replacement return in OmaVero and the
books keep the first return's receipt and figures as the period's filing.

The decision recorded in `filing-correction-unrecordable` (2026-11-21) is that a
correction goes through aggountant like the original filing: previewed,
approved, sent through the API, receipted and recorded. That issue landed the
Incomes Register travel report (replacement and cancellation). The VAT channel
is this issue, because it needs a design the travel report did not.

## When it happens

Example (Esimerkki Oy, fictional): the October 2026 return is filed through
`file-vat` with 24 % sales VAT of 480.00 €. In December a sales invoice of
October turns out to be missing; the correction voucher is sealed in the next
open period. The filed October return is now 24.00 € short and must be
replaced. Today the skill sends the user to OmaVero and tells the agent to
archive the receipt and note it in the home.

## What the API supports (facts from the code and its rules)

- `FileVATReturn/v2` carries `ReplacementReturn` (boolean) and
  `ReplacementReason` (string). The adapter already has both fields
  (`FileVatReturn` in `crates/broker/src/vero_vat_api.rs`) and always sends
  `ReplacementReturn: false` with no reason (`project_file_vat_return`).
- `crates/broker/AGENTS.md`: after an accepted return, every later filing for
  the period is a replacement return with all values and a reason, and needs a
  new explicit approval. A replacement gives the whole return again, not a
  difference.
- `GetFiledVATReturn/v2` and `GetVATPeriods/v1` are already used to reconcile
  an unknown outcome by comparing the filed values with the approved request.

Not verified here, and to be read from the Vero API developer portal
(`api-developer.vero.fi`, SAT / VAT API) before building: the allowed values or
length of `ReplacementReason`; whether a replacement answers with a new
`UniqueIdentifier`; and what `GetFiledVATReturn/v2` returns when several
versions exist (the recovery comparison must match the replacement, not the
first return).

## Why it is its own slice

1. **Where the figures come from.** A return's figures are derived from the
   period's sealed vouchers (`VAT_FIGURES_MISMATCH` guards that), and a sealed,
   filed period cannot be reopened: a correction voucher goes to the next open
   period (`crates/ledger/AGENTS.md`). A replacement return for October
   therefore needs figures that include vouchers sealed in a later period but
   belonging to October. Nothing attributes a correction voucher to the VAT
   period it corrects today, and without that the later period's own return
   would count the same amount again.
2. **Ledger state.** `vat_filed` is a one-way flag with one filing event. A
   replacement needs a second audited filing event for the same period (which
   receipt is in force, which figures), and `acquire_filing_authority` needs a
   capability for "replace the filed return" that is distinct from a first
   filing, so a blind second "first" filing stays impossible.
3. **Recovery.** `file-vat reconcile` adopts a remote return as ours when its
   values match the approved request. With a replacement the remote side holds
   the first return until the replacement is processed, so `Missing` and
   "values differ" mean something else than for a first filing.
4. **The e-service fallback.** `file-vat complete-manual` should record a
   replacement made in OmaVero with evidence instead of refusing, the way
   `file-wage correction complete-manual` does for the travel report.

## Direction

Decide the attribution of correction vouchers to a filed VAT period first (it
is also what keeps the later period's return right). Then: `file-vat` gets a
replacement as its own previewed and approved action with a reason, its own
outbox entry and the same "never resend on a guess" recovery; the Ledger
records each filing of a period; `complete-manual` records an OmaVero
replacement. The skill `skills/vero-fi/SKILL.md` and the guide
(`docs/user/*/reference/status.md`, "Replacement returns are not made") change
with it.
