# Assessment of the review findings

| ID | Finding | Classification | Likelihood | Outcome | Notes |
|---|---|---|---|---|---|
| F1 | Backdated voucher changes a sealed year's carried position | CONFIRMED | OCCASIONAL (a forgotten opening balance found after the first close) | FIX (done, `b492dec`) | Data integrity. This blocked the merge until fixed. It also closes the same pre-existing hole for any never-closed year before a sealed one. |
| F2 | Reopened period erases the "kept here" signal | CONFIRMED | RARE | FIX (done) | `head_seq > 0` |
| F3 | Late period close makes a sealed first year irreproducible | CONFIRMED | RARE | FIX (done) | Same rule as F1, applied to period transitions |
| F4 | Attribution to an earlier year disqualifies the cut-over | CONFIRMED | RARE (library-only) | FIX (done) | Guard widened to `[effective_year, ∞)` |

Simplicity: the change reuses `OpeningBalanceSource::ExternalCutover` and the
existing `first_sealed_year_in` guard rather than adding a new model variant or
an opening-balance store. The F1/F3/F4 fixes state one rule in three write paths:
nothing enters a year at or before a Sealed fiscal year. That rule is now written
in `crates/ledger/AGENTS.md`. Left out on purpose: a CLI verb for external
opening balances and arbitrary chart mappings. Nothing needs them yet.
