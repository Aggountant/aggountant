---
created: 2026-11-11
updated: 2026-11-11
type: feature
reporter: jari
status: open
priority: normal
preparation: ready
lane: capture
lane_seq: 2
collision: [crates/process, skills/capture-receipt]
---

# Capture a receipt with the invoice's own VAT figures

## Description

aggountant computes VAT from the net amount and rate. A supplier invoice can
differ by a cent because of line-level rounding, for example a VAT total of
25.51 instead of 25.50, and then the voucher no longer matches the invoice.

Let capture accept either invoice lines with rates or the invoice's own net /
VAT / gross totals, validate that they add up within permitted rounding, and
seal one voucher with the invoice's figures without splitting the invoice.

## Acceptance Criteria

- [ ] A receipt with source totals that differ from recomputed VAT within rounding seals with the source figures.
- [ ] Totals outside the permitted rounding are rejected with the difference shown.
