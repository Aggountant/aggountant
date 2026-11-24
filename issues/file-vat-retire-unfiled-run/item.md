---
created: 2026-11-24
updated: 2026-11-24
type: bug
status: untriaged
priority: normal
provenance: ai-review
---

# file-vat: a Run that stopped before filing cannot be retired

## Description

## Problem

A `file-vat` Run that stopped before its filing was ever attempted cannot be
closed. Two shapes occur in a real company home, both from early builds:

- `reconcile` left `executing` (the process died during reconciliation);
  `run show` reports `running` forever, and `run resume` returns the same view
  without progressing or settling it.
- `reconcile` `rejected`; the Run is `needs_attention`.

In both, `submit-filing` and `submit-payment` are `planned`, no gate was ever
opened, and nothing was sent. The period was later filed by another Run (or
manually), so these Runs are dead. `approve-filing --answer reject` does not
apply because there is no `ApproveFiling` gate, and no other command
retires them. They stay in `run list` as `running` / `needs_attention`, which
tells every later agent that VAT work is in progress or needs attention when
it does not.

## Expected

A supported way to retire such a Run, for example
`file-vat abandon --run-id <id> --reason <text>`:

- allowed only when no filing attempt can have left the process (submit-filing
  `planned`, nothing for it in the outbox or journal); refused otherwise, with
  `file-vat reconcile` / `status` named as the way forward;
- records the reason and leaves the Run in a terminal, truthful state
  (e.g. `abandoned`), shown by `run show` and `run list`;
- never touches vouchers, periods or another Run's filing.

Separately, `run resume` on a Run whose `reconcile` is stuck at `executing`
should either re-drive reconciliation or say why it cannot, rather than return
the unchanged view silently.

## Acceptance criteria

- [ ] A Run stuck before filing (both shapes) can be retired; `run show` says so.
- [ ] Retiring is refused once a filing attempt may have been sent.
- [ ] `run resume` on a stuck `reconcile` either progresses or explains.
- [ ] `skills/vero-fi/SKILL.md` and the VAT troubleshooting page describe it.
