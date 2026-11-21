---
created: 2026-11-21
updated: 2026-11-21
type: bug
status: untriaged
priority: normal
---

# VatExpansionTreatment no-vat drops members: check stored vouchers, then refuse

## Description

Proposed by the worker of `receipt-vat-proposal-no-vat-unknown-members`.
`VatExpansionTreatment::NoVat` (`crates/model/src/vat_expansion.rs`) is an
internally tagged unit variant, so serde's `deny_unknown_fields` does not
reach it and members on `no-vat` are dropped silently. It is read from sealed
vouchers, so it cannot be tightened blindly.

## Wanted

Check that no stored voucher carries members on a `no-vat` expansion
treatment (a read-only scan of a fiscal store, or proof from the write path
that none can exist), then apply `aggountant_model::de::deny_members` to the
variant with a regression test. If a stored record can carry such members,
stop and describe it instead.
