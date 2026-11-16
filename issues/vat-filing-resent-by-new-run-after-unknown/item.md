---
created: 2026-11-13
updated: 2026-11-16
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
preparation: ready
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

## Decision (product owner, 2026-11-16)

A new filing for a period whose earlier filing is unresolved is refused until
the Tax Administration has been asked and has answered "not found" twice, the
second answer at least one **bank day** after the first. Bank days, not
calendar days: Monday to Friday excluding Finnish bank holidays (New Year's
Day, Epiphany, Good Friday, Easter Monday, 1 May, Ascension Day, Midsummer
Eve, Independence Day, Christmas Eve, Christmas Day, St Stephen's Day). The
codebase has no bank-day calendar yet; add one in the domain with tests for
the moving holidays.

- Both checks are read-only `GetFiledVATReturn` reads and are recorded as
  evidence on the unresolved Run (timestamp, answer).
- "Found and matching" adopts the filing (as `file-vat reconcile` does);
  nothing is sent.
- A failed or ambiguous read records nothing that counts toward the two
  answers.
- After the second qualifying "not found", the unresolved Run is closed as not
  filed and a new `file-vat` Run may file the period from the beginning.
- The guard sits on the single dispatch path (`run_submit_filing`), so no
  route around it exists.

This also covers an interrupted Run (`perfectly-uppity-knife`): the same check
decides between adopting and starting over.

## Acceptance criteria

- [ ] Run 1 ends `OUTCOME_UNKNOWN`; Run 2 for the same period is refused before any FileVATReturn request.
- [ ] One "not found" is not enough; a second on the same bank day (or over a weekend/holiday with no bank day between) is not enough.
- [ ] Two "not found" answers one bank day apart close Run 1 and Run 2 files once.
- [ ] "Found" adopts the receipt and sends nothing.
- [ ] Bank-day calendar tests include Easter-dependent holidays and Midsummer Eve.
- [ ] The user guide (fi, en) explains the wait in the VAT filing section.
