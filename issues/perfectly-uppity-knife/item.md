---
created: 2026-11-13
updated: 2026-11-13
type: bug
status: open
priority: normal
provenance: other
provenance_detail: observed while implementing vat-filed-status
source_ref: taskfleet:01m79z4hcfcrhvd8c8jtkffejr/observation:file-vat-resume-cli
originating_run: 01m79z4hcfcrhvd8c8jtkffejr
originating_run_kind: spinoff
lane: vat-filing
lane_seq: 8
collision: [crates/process/src/file_vat.rs]
---

# An interrupted file-vat Run: check with the Tax Administration, then adopt or start over

## Description

## Problem

A `file-vat` Run can be interrupted between the broker submit, the fiscal
filing record and the journal append (the crash windows documented on `Seam`
in `crates/process/src/lib.rs`). The Run is then left with the filing effect
`Executing` (or `Succeeded` without `vat_filed`). Today `file-vat status`
answers `filing_interrupted` with "do not file again until the Run has been
resumed", but no CLI command resumes it (`ProcessEngine::resume_file_vat` is
library-only), so the user is stuck.

## Direction (product owner, 2026-11-13)

No resume command. Recovery works the way a person would do it: ask the Tax
Administration what happened, then act on the answer.

- `file-vat status` for an interrupted Run queries `GetFiledVATReturn` (read
  only, as it does for a receipted filing) instead of only answering locally.
- If the return is there, the Run is converged from that evidence (the same
  adoption `file-vat reconcile` does for an unknown outcome: receipt and
  `vat_filed` recorded, nothing sent), and status names that command.
- If the return is not there, the interrupted Run is closed as not filed, with
  the evidence recorded, and a fresh `file-vat` Run for the period can start
  from the beginning. Nothing blocks the fresh start once the check has said
  "not filed".
- A failed or ambiguous read is "status unknown" and changes nothing; the user
  retries the check later. Never file again on an unknown answer.

## Acceptance criteria

- [ ] Interrupted → status shows filed → adopt command records the receipt; no FileVATReturn request.
- [ ] Interrupted → status shows not filed → the Run is closed and a new `file-vat` Run starts and files once.
- [ ] Interrupted → read fails → nothing changes and a new Run is still refused.
- [ ] The user guide (fi, en) describes the recovery in the delayed-visibility / VAT filing section.

Originally observed while building `file-vat status` (issue vat-filed-status,
taskfleet run 01m79z4hcfcrhvd8c8jtkffejr, 2026-11-13).
