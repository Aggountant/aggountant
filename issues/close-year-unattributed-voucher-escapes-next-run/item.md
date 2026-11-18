---
created: 2026-11-18
updated: 2026-11-18
type: bug
status: in-progress
priority: high
provenance: ai-review
source_ref: taskfleet:01m7q5ksn5752w7p5s6j0s4qss/review-finding:C1-unattributed-closeyear-voucher
review_source: ai-review
originating_run: 01m7q5ksn5752w7p5s6j0s4qss
originating_run_kind: spinoff
labels:
- ai-review-model:gpt-6-astra
- ai-review-model:claude-fable-5-2
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_confidence: high
review_severity: must-fix
review_target: 5fb11fb..HEAD:crates/process/src/close_year.rs
lane: close-year
lane_seq: 7
collision: [crates/process/src/close_year.rs]
preparation: ready
---

# close-year: a voucher sealed but never attributed by an abandoned run escapes the next run

## Description

# close-year: a voucher sealed but never attributed by an abandoned run escapes the next run

## Description

A close-year Run seals each year-end voucher into the next year's open period
and then attributes it to the closing year, in two separate ledger commits
(`seal_attributed` in `crates/process/src/close_year.rs`). If the process stops
between the two and that Run is abandoned rather than resumed, the voucher
stays in the books with no attribution.

A later Run for the same year whose input does not contain that entry does not
see it: `assert_run_vouchers_attributed` covers only the Run's own plan, and
`assert_no_unplanned_attributions` only vouchers already attributed to the
year. `Ledger::fiscal_year_data` counts an unattributed voucher in its posting
year, and `Ledger::seal_year` has no check for pending close-year vouchers. The
later Run seals the year without the entry; afterwards the voucher can no longer
be attributed (the year is sealed), so it stays in the next year's result.

Sequence (Esimerkki Oy, made-up figures): Run A for 2026 (asset kone-1, 2,000.00
depreciation) is approved; the process stops after the depreciation voucher is
sealed on 2027-01-01 but before it is attributed. Run A is abandoned. Run B for
2026 is started without kone-1 and completes. 2026 is sealed without the
2,000.00 depreciation, and 2027's result carries it.

The window is two consecutive ledger commits, and it also needs the Run to be
abandoned and the entry left out of the next Run. If Run B includes kone-1 with
the same figures, its seal replays and attributes the voucher, so the books are
right (`an_adjustment_sealed_but_not_attributed_is_attributed_when_the_run_continues`
covers the same-Run case).

Fix sketch: treat a sealed voucher whose natural key starts with
`closeyear:<year>:` and has no attribution like an unplanned attribution:
refuse to start, approve `ApproveAdjustments` or bind the statements until the
Run's input includes it. That needs a way to find such vouchers (the next
year's periods, or a ledger lookup by natural-key prefix).

Found by the review of close-year-seal-after-year-sealed,
close-year-correction-on-rejected-run and
close-year-liitetiedot-miss-booked-depreciation (finder gpt-6-astra, confirmed by
the verifier claude-fable-5-2 by reading the code). Predates those changes.
