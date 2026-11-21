---
created: 2026-11-21
updated: 2026-11-21
type: bug
status: fixed
priority: high
preparation: ready
lane: year-end-surface
lane_seq: 0
collision: [crates/cli/src/main.rs]
closed: 2026-11-21
---

# close-year answer defaults to approve, also on the gate that seals the fiscal year

## Description

`close-year answer` defaults `--answer` to approve, also on `ApproveStatements`, which seals the fiscal year irreversibly. `--gate`, `--answer`, `--run-id` and `--correction` have no help text, and the gate names live only in the command summary. Wanted: `--answer` is required (no default), every flag is documented, and the help names the gates and says which approval is permanent.

Found by the `skill-close-year` worker (run 01m7yftv83pzx3er3gew04q7s9) while fact-checking the new skill against a synthetic year-end close.

## Resolution

### 2026-11-21T07:52:53Z · @issuectl

The answer flag is now required; help names the gates and the permanent approvals (see result.md).
