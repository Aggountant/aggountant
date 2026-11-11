---
created: 2026-11-11
updated: 2026-11-11
type: improvement
reporter: jari
status: open
priority: normal
preparation: ready
lane: capture
lane_seq: 3
collision: [skills/capture-receipt]
---

# Agent wording: not found in evidence is not unpaid

## Description

The agent could call an invoice unpaid when it found no matching payment in
the bank material, although the invoice may have been paid with a personal
card or from another account. Unpaid leads to a wrong liability booking.

Pin the skill guidance and tests to four distinct states: not found in the
available evidence, user-confirmed personal payment, matched to a bank
transaction, and genuinely unpaid. No payment inference engine.

## Acceptance Criteria

- [ ] The capture skill names the four states and when each may be stated.
- [ ] A test or eval case covers a payment missing from the bank material.
