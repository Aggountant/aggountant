---
created: 2026-11-13
updated: 2026-11-13
type: improvement
status: untriaged
priority: normal
provenance: other
provenance_detail: found during spinoff run
source_ref: taskfleet:01m7a1259e86vgr944j9z8tfnh/close-year-blind-approval
originating_run: 01m7a1259e86vgr944j9z8tfnh
originating_run_kind: spinoff
---

# close-year approval gates do not show the statements they bind

## Description

# close-year approval gates do not show what they approve

## Problem

`aggountant close-year start` and `close-year answer` return only the Run view
(effect and gate states, snapshot hashes). The four gates
(`ApproveAdjustments`, `ApproveTaseErittelyt`, `ApproveLiitetiedot`,
`ApproveStatements`) bind a hash of the adjustments, the balance sheet
specifications, the notes and the statements, but no command shows those
contents to the person approving them. `start` echoes the depreciation lines;
the accruals, corrections, statements, tase-erittelyt and liitetiedot are
never shown before approval.

## Workaround today

After `ApproveAdjustments` the adjustments are attributed to the closing year,
so `report statements --period <last month of the year>` shows the year's
income statement and balance sheet including them. The user guide points to
it (`docs/user/*/reference/status.md`, year-end section).

## Expected

Each gate's output (or a `close-year show --run-id` read command) shows the
exact content whose hash the gate binds, so an approval is never blind.

Found while fixing `close-year-adjustment-attribution`; the user guide listed
it as the second reason not to use close-year for a real close.
