---
created: 2026-11-10
updated: 2026-11-10
type: task
status: open
priority: normal
lane: vat-filing
lane_seq: 30
collision: [crates/process/tests/file_vat.rs]
---

# Lock in the per-direction VAT cross-check in file-vat with a test

## Problem

`file-vat` cross-checks the supplied VAT figures against the sealed ledger per
direction (sales VAT and purchase VAT separately). A reverse-charge voucher
tagged with swapped codes keeps the net at zero while flipping the
per-direction totals. The existing test
`file_vat_refuses_figures_that_disagree_with_the_sealed_ledger`
(`crates/process/tests/file_vat.rs`) uses input that also differs in net, so a
refactor to a net-only cross-check would let the swapped-code case through with
every test still green.

## Expected

A test that seals a correctly tagged reverse-charge voucher (for example 10 000.00
base, 2 550.00 output and 2 550.00 input VAT), supplies the correct figures, feeds a
ledger whose codes are swapped, and asserts `VAT_FIGURES_MISMATCH`: net equal,
per-direction different.

## Acceptance criteria

- [ ] The new test fails against a net-only cross-check and passes on the current per-direction check.
