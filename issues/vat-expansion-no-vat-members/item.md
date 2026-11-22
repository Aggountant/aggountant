---
created: 2026-11-21
updated: 2026-11-22
type: bug
status: fixed
priority: normal
lane: vat-goods
collision: [crates/model/src/vat_expansion.rs, crates/domain/src/vat/expand.rs]
lane_seq: 2
closed: 2026-11-22
closed_by: worker
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

## Decisions

### 2026-11-22T04:29:46Z · @conductor

Accepted by the product owner (2026-11-22); done together in one change.

## Resolution

### 2026-11-22T04:45:41Z · @worker

Proved no stored no-vat expansion can carry members (writer emits the tag alone; decode_voucher_body refuses non-canonical bytes); deny_members applied. See result.md.
