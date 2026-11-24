---
created: 2026-11-10
updated: 2026-11-24
type: bug
status: fixed
priority: high
lane: vat-filing
lane_seq: 11
collision: [crates/process/src/file_vat.rs, skills/vero-fi/SKILL.md]
closed: 2026-11-24
closed_by: claude-opus-5-5
---

# file-vat Run never finishes after a direct filing of a payable return

## Problem

On the direct Vero.fi route a payable VAT return leaves the `file-vat` Run
unfinished. After `approve-filing` succeeds (receipt stored, period marked
`vat_filed`), the Run opens an `ApprovePayment` gate whose bank-payment effect
stays `planned`. The direct route has no payment adapter, so approving that gate
cannot succeed (the `vero-fi` skill already says so), and nothing else closes
it. `run show` keeps reporting `awaiting_gate` for a return that is filed and
whose payment the user has made or scheduled in their own bank.

## Expected

The Run reaches a terminal, truthful state once filing is accepted: either
`completed` with the payment recorded as the user's responsibility (amount, due
date, payee account from the proposal), or an explicit command such as
`file-vat complete-payment --run-id … --evidence-ref …` that records an
externally made payment without contacting a bank. Neither path may claim the
tax is paid without evidence.

## Acceptance criteria

- [x] A filed, payable return on the direct route does not stay `awaiting_gate` indefinitely.
- [x] An externally handled payment can be recorded with evidence, or the Run states plainly that payment is outside aggountant.
- [x] Refund (negative net) behaviour unchanged.
- [x] `skills/vero-fi/SKILL.md` updated to match.

## Resolution

### 2026-11-24T07:44:43Z · @claude-opus-5-5

Fixed: direct-route payable Runs complete with the payment left to the user; parked Runs finish with `file-vat user-pays`. See result.md.
