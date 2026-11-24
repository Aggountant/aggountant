# Assessment of the file-vat-retire-unfiled-run review

Assessed against `3f5bc41`.

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| (none) | The finder and the verifier both answered "No findings" | n/a | n/a | n/a | n/a | n/a | n/a |

FIX: 0   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

No data-integrity finding is open. The verifier's evidence agrees with the
host's own reading. A Run can send its filing only after its journal holds
an approved `ApproveFiling` and `submit-filing` executing, and both are what
`filing_never_began` excludes.

Two related problems were seen during the work. They are outside the brief
and are not findings of this review:

- `run resume` runs month-close recovery on `run-payroll`,
  `capture-receipt`, `hunt-receipts` and `close-year` Runs, and on an open
  period that can move the month to `closing`. Filed unlaned as
  `run-resume-refuses-other-workflows`.
- A `close-period` start refused at `begin_close` stays `needs_attention`,
  the same shape as this issue. Proposed as a spin-off in the run report.

## Simplicity

- **Reused:** the derived Run status. `not_filed` is one predicate inside
  `Fold::status`, with no stored status, no new journal event type and no
  store migration. `EffectRejected` records the resume settle, and the
  existing `VAT_FILING_UNRESOLVED` covers the refusal. `run resume` is the
  retirement path, so there is no `file-vat abandon` command.
- **Repetition removed:** the seven per-branch `EffectRejected` appends in
  `start_filing` are now one, at the closure's error exit. The preflight
  code that differs from the caller's error code is kept through
  `ReconcileFailure`.
- **One rule:** `filing_never_began` is the single statement of "nothing of
  this Run can have left". Both the derived status and the resume settle use
  it.
- **Left out on purpose:** an outbox check. Attempts are keyed by period, so
  they may be another Run's, and the journal already shows any attempt this
  Run made. Also left out: re-driving a reconcile on resume (the figures may
  be stale, and a new Run is the supported way), routing `run resume` to
  `resume_file_vat` for later stages (that needs a broker and credentials),
  and an explicit abandon command (items 1–4 cover every shape in the issue).
