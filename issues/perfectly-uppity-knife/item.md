---
created: 2026-11-13
updated: 2026-11-13
type: improvement
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

# Interrupted file-vat Runs have no CLI resume path

## Description

## Problem

`ProcessEngine::resume_file_vat` (`crates/process/src/file_vat.rs`) is the only
recovery for a `file-vat` Run interrupted between the broker submit, the fiscal
filing record and the journal append (the crash windows documented on `Seam`
in `crates/process/src/lib.rs`). No CLI command calls it: `aggountant run
resume` dispatches to the close-period resume (`ProcessEngine::resume`), and
the `file-vat` subcommands have no resume.

A Run left in such a window shows the filing effect `Executing` (or
`Succeeded` without `vat_filed`). The Tax Administration may already have
accepted the return, yet the user has no command that converges the Run, and
`file-vat status` can only answer `filing_interrupted` with "do not file
again until the Run has been resumed".

## Suggested fix

Wire `resume_file_vat` to the CLI (for example `file-vat resume --run-id`, or
make `run resume` dispatch by workflow) with the same broker construction as
`file-vat reconcile`, and let `file-vat status` name that command as
`next_command` for `filing_interrupted`.

Observed while building `file-vat status` (issue vat-filed-status, taskfleet
run 01m79z4hcfcrhvd8c8jtkffejr, 2026-11-13).
