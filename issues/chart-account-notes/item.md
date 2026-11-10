---
created: 2026-11-10
updated: 2026-11-10
type: task
status: open
priority: normal
lane: chart-decisions
lane_seq: 22
related: ['@sie4-chart-import']
---

# Chart description and per-account notes for charts that encode VAT in account numbers

## Problem

Some charts of accounts encode VAT treatment in the account number: the same
expense is split into several accounts by where it was bought. A synthetic
example:

| Account | Name |
|---|---|
| `4000` | Ostot |
| `4010` | Tavaraostot EU |
| `4130` | Ostot EU:n ulkopuolelta |
| `7660` | Atk-ohjelmistot |
| `7665` | Atk-ohjelmistot, yhteisöhankinta |

`7660` and `7665` are the same expense; the only difference is whether it was an
intra-Community purchase. In aggountant the kind of expense decides the account,
and the VAT treatment comes from the posting's VAT code. A chart like this,
imported faithfully (`sie4-chart-import`), is redundant with the VAT code and
splits expense reporting: "how much went on software" needs two accounts.

Importing such a chart is correct; the question is how an agent knows what the
accounts mean. Decided: a chart is not just a list of numbers. It gets a prose
description of what kind of chart it is, and individual accounts can have
written notes (for example "`7665` is `7660` for intra-Community purchases;
the VAT code already carries the difference"). There is no typed duplicate flag
and no programmatic enforcement.

## Open questions

1. Should a domestic VAT code posted to an account documented as
   "intra-Community" be reported by the mistag check?
2. When a company later moves to the standard chart, are such duplicates merged
   at that point?

## Expected

A place for a chart-level description and per-account notes, guidance (in the
relevant skill) on writing them, and the capture and import skills reading them
before choosing an account.

## Acceptance criteria

- [ ] Chart and account notes can be written, versioned and shown with the chart.
- [ ] Skill guidance tells the agent to read them before assigning an account.
- [ ] The two open questions are answered in the plan.
