---
created: 2026-11-24
updated: 2026-11-24
type: bug
status: fixed
priority: high
provenance: ai-review
lane: vat-filing
lane_seq: 11
collision: [crates/process/src/file_vat.rs, skills/vero-fi/SKILL.md]
closed: 2026-11-24
closed_by: claude-opus-5-5
commits:
- hash: 3f5bc41
  summary: 'file-vat: a Run stopped before filing ends not_filed'
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

- [x] A Run stuck before filing (both shapes) can be retired; `run show` says so.
- [x] Retiring is refused once a filing attempt may have been sent.
- [x] `run resume` on a stuck `reconcile` either progresses or explains.
- [x] `skills/vero-fi/SKILL.md` and the VAT troubleshooting page describe it.

## Comments

### 2026-11-24T16:23:33Z · @conductor

Reframing after reading `start_file_vat` on current main (`8acda3b`): both shapes still arise today, so this is not only legacy cleanup.

- **Rejected reconcile is a normal outcome.** Every `file-vat start` that fails the sealed-ledger cross-check (`VAT_FIGURES_MISMATCH`), the account reconciliation (`VAT_ACCOUNTS_UNRECONCILED`), the authority preflight or the request projection journals `EffectRejected` and leaves the Run `needs_attention` permanently; the error text itself says "start a new file-vat Run". Each corrected retry therefore leaves one more permanent `needs_attention` Run behind.
- **Stuck `executing` can still happen.** Between `EffectExecuting` and the reconcile outcome there are `?` paths that return without journalling a rejection (for example `Ledger::open`), so an error or crash there leaves `reconcile` `executing` and the Run `running`; `run resume` returns it unchanged.

Suggested shape, likely simpler than an abandon command: a reconcile that is rejected before any filing attempt ends the Run in a terminal "not filed, nothing sent" state instead of `needs_attention`; every error path inside reconcile journals its rejection; and `run resume` settles an `executing` reconcile with no filing attempt to that same terminal state, which also retires the old Runs without a new command. Whether an explicit abandon command is still wanted is a triage question.

## Decisions

### 2026-11-24T16:27:19Z · @conductor

Accepted for execution by Jari 24.11.2026, in the reframed shape of the previous comment: a reconcile rejected before any filing attempt ends the Run terminally (not filed, nothing sent); every reconcile error path journals its rejection; run resume settles an executing reconcile with no filing attempt to the same state, which also retires old Runs. Lane vat-filing ahead of tax-payment-references.

## Resolution

### 2026-11-24T16:47:22Z · @claude-opus-5-5

Fixed in the reframed shape: a reconcile rejected before any filing attempt ends the Run not_filed (derived on read, so older needs_attention Runs read not_filed); every reconcile error is journalled; run resume settles a reconcile left planned/executing with nothing later begun (VAT_INTERRUPTED_BEFORE_FILING) and refuses with VAT_FILING_UNRESOLVED otherwise. No abandon command, no migration. See result.md, review.md, assessment.md.
