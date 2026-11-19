---
created: 2026-11-11
updated: 2026-11-19
type: bug
reporter: jari
status: fixed
priority: high
preparation: ready
lane: vat-filing
lane_seq: 7
collision: [crates/process/src/file_vat.rs]
closed: 2026-11-19
closed_by: agent
---

# A failed preflight read leaves the file-vat effect executing

## Description

Before submitting, the file-vat Run reads the period from the Tax
Administration. When that read fails, the effect stays in `executing` and the
Run gives no sign that it has stopped and needs the user.

Journal the failure terminally with the existing rejection / needs-attention
event. No automatic retry and no redesign of Run state.

## Acceptance Criteria

- [x] A failed period read leaves the Run in a needs-attention state visible in `run show`.
- [x] A test with the mock returning an error for the period read covers it.

## Resolution

### 2026-11-19T12:06:25Z · @agent

A failed or refused GetVATPeriods preflight now rejects the reconcile effect, so the Run shows needs_attention; see result.md.
