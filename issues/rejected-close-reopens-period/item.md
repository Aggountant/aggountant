---
created: 2026-11-10
updated: 2026-11-13
type: bug
status: in-progress
priority: high
lane: period-close
lane_seq: 1
collision: [crates/process, crates/ledger]
preparation: ready
---

# Rejecting a period close strands the period in Closing

## Problem

Answering `ApproveClose` with `reject` ends the close-period Run as `rejected`
but leaves the period in `Closing`, owned by that Run. The code says so
(`crates/process/src/lib.rs`: the period stays Closing "for the driver to
resolve"), but nothing can resolve it: the ADR-6 period state machine
(`is_legal_period_edge` in `crates/ledger/src/lib.rs`) has no `Closing→Open`
edge, and `Ledger::reopen_period` only reopens a *Sealed* period
(`Sealed→Open|Closing`), so it does not apply here.

## Reproduction (synthetic home)

1. `run start close-period --period 2026-05` → period `closing`.
2. `run answer --gate ApproveClose --answer reject` → Run `rejected`, period
   still `closing`.
3. `capture-receipt record` into 2026-05 → `PERIOD_CLOSING`.
4. `run start close-period --period 2026-05` → `PERIOD_TRANSITION_CONFLICT`.
5. `run resume` → still `rejected`; `run answer … approve` → `GATE_NOT_OPEN`.

The month can no longer take vouchers or be sealed, so its VAT return cannot be
prepared either.

## Direction (product owner, 2026-11-13)

Rejecting a close means "not ready yet": the rejection itself returns the
period to `Open`, so the user can fix the month and start a new close. No
separate command.

This needs a new legal edge `Closing→Open` in the period state machine, taken
only by the Run that owns the Closing period and recorded in the hash-chained
period transitions with that Run as authority, and `verify_periods` must accept
it. A home that contains the new edge will not verify under an older binary;
that is acceptable (binaries move forward), but say so in the changelog-facing
result. Anything the rejected Run left in the period must be accounted for:
if close-period writes vouchers or other state into a Closing period, either
it stays valid in an Open period or the rejection is refused with a clear
error; nothing is silently discarded.

## Acceptance criteria

- [ ] After a rejected close the period is `Open`, accepts vouchers again and a new close can start.
- [ ] A test covers reject → record → close → seal, and verification of the home passes afterwards.
- [ ] Only the owning Run can take `Closing→Open`; other transitions stay refused.
- [ ] The user guide's "never reject a close" warning is removed in the same commit (fi and en).
