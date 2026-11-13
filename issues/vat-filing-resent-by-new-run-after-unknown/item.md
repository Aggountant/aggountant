---
created: 2026-11-13
updated: 2026-11-13
type: bug
status: open
priority: high
provenance: other
provenance_detail: found while verifying an issue rewrite
source_ref: taskfleet:01m7a125k88v6kskced061b93a/cross-run-unknown-resend
originating_run: 01m7a125k88v6kskced061b93a
originating_run_kind: spinoff
lane: vat-filing
lane_seq: 6
collision: [crates/process/src/file_vat.rs, crates/broker/src/vero_vat_api.rs]
---

# A new file-vat Run re-sends a filing whose outcome is still unknown

## Description

## Problem

After a VAT filing ends in `OUTCOME_UNKNOWN` (a timeout or a 5xx from
FileVATReturn), nothing stops a new `file-vat` Run for the same period from
sending the return again.

Inside one Run the filing is protected: `retry_file_vat_submit` refuses
`submit-filing`, and `resume_file_vat` leaves a parked effect alone. Across
Runs, though, the broker does not short-circuit an `Unknown` attempt by design
(`Broker::submit`, `crates/broker/src/broker.rs`: "`Unknown` is intentionally
NOT short-circuited"). The outbox key `submit-filing:<period>` is shared, and
the bytes stay the same while the sealed vouchers do not change, so the
idempotency check passes. Neither `start_file_vat` nor `answer_file_vat`
(`crates/process/src/file_vat.rs`) looks at an earlier Run's unresolved filing.
`period_has_unresolved_unknown_filing` exists, but only `complete-manual` uses it.

Reproduced with the scripted adapter in the `crates/process/tests/file_vat.rs`
harness: Run 1 approve → Timeout → `NeedsAttention`. Run 2 for the same period
starts, its `ApproveFiling` is approved, the second scripted FileVATReturn step
(a 200 receipt) is consumed, and the period becomes `vat_filed`. Against the
real authority that would be a second filing of a return that may already have
been accepted.

## Open decision

Blocking the second Run needs an answer to when re-filing becomes allowed.
`file-vat reconcile` adopts the filing only when GetFiledVATReturn matches the
approved bytes. Otherwise it leaves the Run parked, and a "Missing" answer can
also mean "not visible yet; can take days". So a guard needs a defined way out
for a filing that really never arrived, for example an explicit operator
action that records the authority's answer as evidence. The guard belongs on
the one dispatch path (`run_submit_filing` already says it "owns every path
that can dispatch the filing").

Related: `staggeringly-aspiring-sky` makes a receiptless 2xx an
`OUTCOME_UNKNOWN`, so after it lands this gap covers that case too.
