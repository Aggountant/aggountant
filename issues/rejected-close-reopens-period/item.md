---
created: 2026-11-10
updated: 2026-11-10
type: bug
status: open
priority: high
lane: period-close
lane_seq: 1
collision: [crates/process]
---

# Rejecting a period close strands the period in Closing

## Problem

Answering `ApproveClose` with `reject` ends the close-period Run as `rejected`
but leaves the period in `Closing`, owned by that Run. The code says so
(`crates/process/src/lib.rs`: the period stays Closing "for the driver to
resolve"), but the CLI has no command that resolves it. `Ledger::reopen_period`
exists; nothing in `crates/cli` exposes it.

## Reproduction (synthetic home)

1. `run start close-period --period 2026-05` → period `closing`.
2. `run answer --gate ApproveClose --answer reject` → Run `rejected`, period
   still `closing`.
3. `capture-receipt record` into 2026-05 → `PERIOD_CLOSING`.
4. `run start close-period --period 2026-05` → `PERIOD_TRANSITION_CONFLICT`.
5. `run resume` → still `rejected`; `run answer … approve` → `GATE_NOT_OPEN`.

The month can no longer take vouchers or be sealed, so its VAT return cannot be
prepared either.

## Expected

A rejected close returns the period to `Open`, or an explicit, guarded command
does, so the user can finish the month and start a new close.

## Acceptance criteria

- [ ] After a rejected close the period accepts vouchers again and a new close can start.
- [ ] A test covers reject → record → close → seal.
- [ ] The user guide's "never reject a close" warning is removed in the same commit.
