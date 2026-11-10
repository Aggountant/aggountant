---
created: 2026-11-10
updated: 2026-11-10
type: improvement
status: open
priority: normal
lane: vat-roles
lane_seq: 30
collision: [crates/domain/src/vat/roles.rs, crates/domain/src/vat/guards.rs, crates/domain/src/vat/expand.rs]
---

# Reverse charge: validate scheme and output/input pairing at seal time

## Problem

The record-time guard `check_voucher_vat_structure` detects a malformed reverse
charge by arithmetic (`gross > 0 && net == 0` on the VAT base per direction).
That is necessary but not sufficient. It cannot detect:

- a cancellation masked by an unrelated same-direction line (net ≠ 0);
- an unequal-base mistag that leaves a small rounding residual;
- a pair split across two vouchers.

Each leaves a non-zero figure that the `file-vat` cross-check surfaces, so none
silently zeroes the return, but the voucher is still structurally wrong.

The root cause is that a `VatCode` reduces to `(direction, rate_class)`. The
scheme (EU acquisition, domestic reverse charge, import) and the pairing of the
output and input components are discarded, so no arithmetic predicate can tell
a mistagged reverse charge from a legitimate same-voucher reversal.

## Expected

The VAT tag carries the scheme, and each reverse-charge group is validated at
seal time: output component present with the correct side, deductible input
component present with the correct side, matching base and rate. The
structural pairing check then replaces the arithmetic heuristic.

## Acceptance criteria

- [ ] The three cases above are rejected at seal time with a structural error.
- [ ] A legitimate same-voucher reversal still seals.
- [ ] `check_voucher_vat_structure`'s heuristic is removed, not kept beside the new check.
