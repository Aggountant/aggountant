---
created: 2026-11-10
updated: 2026-11-10
type: task
status: open
priority: normal
lane: chart-decisions
lane_seq: 30
collision: [crates/ledger/data/fi-general-2026.json]
related: ['@chart-account-notes']
---

# Standard chart of accounts: which model it follows

## Problem

The bundled standard chart (`crates/ledger/data/fi-general-2026.json`, 98
accounts) is not documented as derived from any known model. Finnish law does
not prescribe account numbers: kirjanpitolaki requires a chart suited to the
business and used consistently. In practice the numbering convention comes from
what bookkeeping programs offer by default, so a chart that accountants,
auditors and users recognise is a real requirement, not a preference. Decided:
use the common, familiar numbering rather than inventing one.

The standard chart also encodes VAT treatment on the sales side (`3200`
intra-Community sales of goods, `3300` exports outside the EU), the same
mechanism `chart-account-notes` describes for imported charts. Splitting sales
into domestic, EU and export has its own justification (the income statement
and the VAT return need it), but the choice should be explicit.

## To decide

1. Which known model the standard chart follows, with the source documented.
2. Coverage: a small-company minimum, or broader.
3. Sales split: keep separate accounts for EU and export sales, or one sales
   account with the VAT code carrying the split. Record why.
4. A prose description of the standard chart and per-account notes, the same
   as for imported charts (`chart-account-notes`).

Not in scope: SIE4 import (`sie4-chart-import`), role metadata for imported
charts, migration between charts.

## Acceptance criteria

- [ ] The chart's source model is documented in `crates/ledger/data/PROVENANCE.md`.
- [ ] The four questions are answered and the chart file updated accordingly.
- [ ] Existing homes keep their activated chart version unchanged.
