---
created: 2026-11-10
updated: 2026-11-10
type: feature
status: open
priority: normal
lane: chart-decisions
lane_seq: 20
collision: [crates/ledger]
related: ['@chart-account-notes']
---

# Import a company's own chart of accounts from SIE4

## Problem

A company moving to aggountant from another bookkeeping program has its own
chart of accounts, and its earlier books use that chart's numbers. The bundled
standard chart (`fi-general-2026`) numbers many accounts differently or lacks
them. A synthetic example:

| Purpose | Company's chart | Bundled standard chart |
|---|---|---|
| Input VAT receivable | `1763 ALV-saamiset` | (missing) |
| Shareholder loan | `2941 Velat osakkaille` | (missing) |
| Software and IT | `7660 Atk-ohjelmistot` | `7690 Atk-laite- ja ohjelmistokulut` |
| Bank charges | `8560 Rahaliikenteen kulut` | `8350 Muut rahoituskulut` |

To continue the books the way the company keeps them, or to reproduce earlier
years for comparison, aggountant must use the company's own chart. Finnish
bookkeeping programs export the chart (`#KONTO`) and the postings (`#VER`,
`#TRANS`) in SIE4. Chart adoption itself exists (a company can activate a chart
other than the standard one); what is missing is reading it from SIE4.

## Expected

`aggountant` can create a chart version from an SIE4 file's `#KONTO` records
(account number, name, and the account type where the file gives it), for
activation through the existing chart-adoption path. Postings are out of scope
here (`historical-import`).

## Acceptance criteria

- [ ] A synthetic SIE4 file produces a chart version with every `#KONTO` account.
- [ ] Encoding (SIE4 uses PC8/CP437) and account types are handled; unknown records are reported, not dropped silently.
- [ ] The imported chart can be activated and used to seal a voucher.
