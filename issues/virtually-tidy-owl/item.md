---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: untriaged
priority: normal
provenance: ai-review
source_ref: taskfleet:01m7yf4gqpbnqmpjp50392d9xd/review-finding:close-gate-shows-no-content
review_source: ai-review
originating_run: 01m7yf4gqpbnqmpjp50392d9xd
originating_run_kind: spinoff
labels:
- ai-review-model:gpt-6-astra
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_target: skills/aggountant-init/SOUL.md
lane: agent-surface
lane_seq: 4
collision: [crates/process/src/lib.rs]
preparation: ready
---

# Month close approval gate shows a hash, not what the user approves

## Description

## What goes wrong for the user

Closing a month locks its entries. The shared role file
(`skills/aggountant-init/SOUL.md`) tells the agent that the user approves a
permanent act from what they see: the proposal as it will be stored. For a
voucher (`capture-receipt record --dry-run`) and a VAT return
(`file-vat preview`) the program produces that content. For the month close it
does not: the approval step of `run start close-period` answers with step
states and a hash, so the agent has to invent what "close March" means to the
user, and two agents will show different things.

## Observed (synthetic empty home, binary built 2026-11-21)

```
$ aggountant --home <home> --json run start close-period --period 2026-07
{"effects":[{"effect_type":"TransitionPeriod","key":"transition","status":"succeeded"},
 {"effect_type":"SealPeriod","key":"seal","status":"planned"}],
 "gates":[{"answered":null,"gate_id":"ApproveClose","gate_kind":"Approval","snapshot_hash":"f437…"}],
 "period":"2026-07","run_id":"cp-…","status":"awaiting_gate","workflow":"close-period", …}
```

Nothing in the answer says what the snapshot covers: how many vouchers, the
period's result and balance, open obligations, or what the close will lock.
There is also no read-only preview of a close: `run start` itself moves the
period to `Closing`.

## Impact

Every company closes every month, so this is the most frequent permanent
approval in the product, and it is the one the user gives without content.
`close-year-gates-show-content` fixed the same defect for the year-end close.

## Direction

Give the `ApproveClose` gate the content the user approves (what the snapshot
hash stands for: voucher count and range, the period's income statement and
closing balances or the report commands that show them, open obligations), and
a `message`/`next_command` that says how to answer. Until then the month skill
(`skill-month-rebuild`) has to say what the agent shows before a close; when
the gate carries it, that passage goes.

Found while rebuilding SOUL (`skill-soul-rebuild`): an instruction-file review
asked what the agent shows the user for a close, and the binary gave no answer.
