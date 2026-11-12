---
created: 2026-11-12
updated: 2026-11-12
type: bug
status: untriaged
priority: normal
provenance: other
provenance_detail: agent observation while verifying the user-docs design against the binary
source_ref: taskfleet:01m779qnvqysm7q5z9wcs3sgma/observation:bank-row-already-recorded-receipt
originating_run: 01m779qnvqysm7q5z9wcs3sgma
originating_run_kind: spinoff
---

# Guide says a bank row for an already recorded receipt is not recorded again, but the import requires every row

## Description

## Problem

The user guide (`docs/user/fi/04-kuukausikierto.md`, "Pankin tiliote", and the
English counterpart) says that a bank-statement row which pays an
already-recorded receipt "is not recorded again". `aggountant import camt053
seal` refuses a batch in which any booked entry lacks an assignment
(`MISSING_ASSIGNMENT`; `crates/import/src/error.rs`), so the row cannot be left
out. Neither the guide nor the `capture-receipt` skill says what such a row is
assigned to, and the skill's example records a receipt directly against the bank
account (contra 1910).

Observed while checking the user-docs design against the 0.1.0 binary with the
fictional Esimerkki Oy: every booked row of the statement must be assigned for
the seal to succeed.

## Impact

A new user or agent following the guide can book the same purchase twice:
once from the receipt against 1910 and again from the bank row against an
expense account. The month-end check of the bank balance against the statement
catches it, so it is not silent, but it costs a correction voucher in sealed
books.

## Expected

The guide and the `capture-receipt` skill state one convention for a receipt
that also appears on the bank statement (for example: record the receipt against
a liability such as 2700 Ostovelat, or 2740 when the owner paid, and assign the
bank row to clear it), or the import gains an explicit way to mark a row as
settled by an existing voucher. Which of the two is a product decision.
