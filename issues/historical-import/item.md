---
created: 2026-11-10
updated: 2026-11-10
type: task
status: open
priority: normal
lane: chart-decisions
lane_seq: 25
related: ['@sie4-chart-import']
---

# Import a company's earlier bookkeeping as is, without guard rejections

## Problem

A company that starts using aggountant has years of earlier bookkeeping: bank
statements covering many years, and a general ledger from its previous
program. It wants that history inside aggountant from day one, for analysis
(trends, spend per vendor and category, prior-year comparatives) and for
opening balances, and to browse its old data read-only.

This is different from the monthly cycle. The history was already kept and
closed in another system, so it is a fact, not a proposal: aggountant does not
answer for its correctness, and its own record-time guards must not refuse to
import it. Today the import path seals through the same ledger path as new
vouchers, so it meets guards designed to stop new mistakes, for example a
rejected unknown or inactive chart, or an untagged movement on a VAT account
that requires a VAT tag.

## Decided

- Imported history is a mirror of books kept elsewhere; aggountant does not
  validate it into correctness.
- Problems found in it do not block the import. They are recorded as the
  import's durable findings, shown to the user, and listable later per import.
  Findings have at least two classes: "aggountant would book this differently"
  and "this is an error". The classes are refined against real imports, not
  designed exhaustively up front.
- A period of imported history can be closed in simulation; the result is
  marked as mirrored or simulated.
- Mirrored data never reaches a VAT return.

## Open: structure

Choose between:

- **Shared ledger with a flag**: imported history lives in the same structures
  as aggountant's own vouchers, marked as mirrored. One query surface, but every
  reader has to respect the flag, and one that forgets mixes mirrored data into
  aggountant's own results silently.
- **Separate mirror structure**: imported history lives apart from the
  authoritative ledger. Contamination is structurally impossible, but anything
  that crosses the boundary needs a bridge. Opening balances are already such a
  bridge.

Also open: how a multi-year bank file maps onto fiscal years and periods, and
where the earliest opening balances come from.

## Acceptance criteria

- [ ] An ADR records the structure choice and the reasons, citing kirjanpitolaki where it matters.
- [ ] A synthetic multi-year import (bank statements plus an SIE4 ledger) imports without guard rejections and produces a findings report.
- [ ] Imported history can be browsed read-only and is excluded from VAT filing by construction or by test.
