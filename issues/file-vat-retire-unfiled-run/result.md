# Result: file-vat-retire-unfiled-run

A `file-vat` Run that stopped before its filing was attempted now ends in the
terminal status `not_filed`, and `run show` says so. No new command, no
store migration.

## What was built

- **`RunStatus::NotFiled` (`not_filed`)**, derived on read in `Fold::status`
  through `file_vat::stopped_before_filing`. The rule: a `file-vat` Run whose
  `reconcile` is rejected, with no gate and `submit-filing` and
  `submit-payment` still planned. Because it is derived, Runs that older
  builds left `needs_attention` read `not_filed` without anything being
  written (item 4). A stored status was not an option: `run show` folds
  status from effect states. Settling those Runs through `run resume` would
  have made the view wrong until someone ran it.
- **Every reconcile error is journalled.** `start_filing` runs everything
  from the reconcile's `EffectExecuting` to its outcome in one closure, whose
  only error exit journals `EffectRejected`. The `?` paths that used to skip
  the journal (`Ledger::open`, the state write, the payment hash) now end
  the Run `not_filed` as well. A preflight refusal still journals the
  authority's code (for example `VERO_VAT_PERIOD_NOT_FILEABLE`) while the
  caller gets the detail. The error text says the Run "was not filed and
  nothing was sent" instead of "needs attention".
- **`run resume` on a `file-vat` Run** (`resume_file_vat_before_filing`):
  - If `reconcile` is planned or executing and nothing later began, it
    journals `EffectRejected { reconcile, VAT_INTERRUPTED_BEFORE_FILING }`
    and the Run reads `not_filed`. Running it again records nothing.
  - If the journal shows a gate or a filing or payment step, it refuses with
    `VAT_FILING_UNRESOLVED` and names `file-vat status` and
    `file-vat reconcile`, and writes nothing.
  - Every other `file-vat` Run is returned unchanged. Before this change,
    `run resume` ran month-close recovery on `file-vat` Runs.
  - The workflow-version check is skipped on this path, because settling
    runs no workflow step.
- **`EffectView.error_code`**: a rejected effect shows its code in
  `run show`. This makes the guide's existing claim about
  `VERO_VAT_PERIOD_NOT_FILEABLE` true.
- `file-vat status` and the unresolved-filing guard were already blind to
  these Runs, because they read only `submit-filing`. Tests now pin that,
  for first returns and for replacements.

## Why the outbox is not consulted

The brief asked for "nothing in the broker outbox or journal for it". Outbox
attempts are keyed by period (and for a replacement by its hash), not by Run.
An attempt found there may be another Run's, and in the real case another Run
did file the period. A Run journals `submit-filing` executing behind an
approved `ApproveFiling` before the Broker records any attempt, so the
journal alone shows whether this Run can have sent anything. The review's
verifier traced this ordering back to the oldest public revision.

## Tests

- `file_vat/tests.rs`: hand-written legacy journals in a current (v9) home.
  - Reconcile rejected: reads `not_filed`, resume writes nothing, status is
    `not_filed` (first return) or `filed_without_receipt` (replacement), and
    the guard passes.
  - Reconcile executing: resume settles it, a second resume is a no-op, a
    new first return starts.
  - Positive controls: filing executing, and gate opened. Resume refuses
    with `VAT_FILING_UNRESOLVED` and the journal is unchanged.
- `tests/file_vat.rs`: refused starts are `not_filed`, the code is on the
  view, status is `not_filed`, the corrected retry reaches `ApproveFiling`,
  and resume is a no-op.
- `cli/tests/vat_reconciliation_cli.rs`: `run show` and `run resume` JSON
  give `status: not_filed` and the reconcile `error_code`.

## Docs

`skills/vero-fi/SKILL.md`. In both languages: the guide's
`file-vat-direct`, the troubleshooting "run was interrupted" table and its
paragraph, and the error-code tables (new `VAT_INTERRUPTED_BEFORE_FILING`;
`VAT_FILING_UNRESOLVED` now covers the resume refusal). Also the
`run show` / `run resume` help and the CHANGELOG.

## Not done

- No `file-vat abandon`: items 1–4 cover both shapes in the issue.
- Runs that reached `ApproveFiling` or later are unchanged.
- Runs that `file-vat reconcile` closes as not filed still read `completed`.
  They were left alone to keep later-stage behaviour as it was, so there are
  two "not filed" endings that read differently.
