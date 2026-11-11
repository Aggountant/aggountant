---
created: 2026-11-11
updated: 2026-11-11
type: feature
reporter: jari
status: open
priority: high
preparation: ready
lane: vat-filing
lane_seq: 6
collision: [crates/process/src/file_vat.rs, crates/broker/src/vero_vat_api.rs]
---

# Read the status of an accepted VAT return

## Description

After a direct filing the Tax Administration returns a receipt. The receipt
proves intake, not processing, and the return can take days to appear in
OmaVero. There is no safe way to ask aggountant what happened to an accepted
return: `file-vat reconcile` only handles a filing whose outcome is unknown.
The temptation is to file again "to be sure", which cannot be undone.

Add a read-only status command for a completed filing that uses
`GetFiledVATReturn` with the stored receipt and outbox data and reports
received, pending authority processing, or processed. The receipt remains the
fence against resubmission; the status read never sends anything.

## Acceptance Criteria

- [ ] A read-only command reports received / pending / processed for a completed file-vat Run.
- [ ] It never submits; tests with the mock show no FileVATReturn request.
- [ ] The user guide (fi and en) describes it in the delayed-visibility section.
