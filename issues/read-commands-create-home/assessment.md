# Assessment of the read-commands-create-home review

Assessed against `b22ab4e`.

The review produced no findings, so there is nothing to triage.

FIX: 0   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

One change came from the host's own check rather than the model: the first
version of the gate reused the voucher-read preflight (all of
`fiscal.sqlite`, `artifacts/`, `masterdata/reference/`, `masterdata/pii.sqlite`)
and refused the earlier-binary fixture home, which has no empty `artifacts/`.
Fixed before commit by requiring only the voucher database and the reference
masterdata; both cases are tested.

## Simplicity

- Reused: the existing home resolution (`resolve_home`, its `source` and
  `marker_path`, the same vocabulary `config show` prints), the existing
  `HOME_NOT_INITIALIZED` code and `CliError::with_details`.
- Repetition removed: the voucher-read preflight `open_existing_ledger(home)`
  and the per-command `Ledger::open(&home_path(cli)?)` collapse into one
  `existing_home(cli)` that every home-opening command passes through;
  `open_existing_ledger(cli)` is a one-line wrapper for the many ledger-only
  commands.
- Left out: no change to `Ledger::open` itself (the process engine opens it
  per step on a home the CLI has already checked, and tests across the crates
  use it on fresh directories); no per-command allow list or flag to create a
  home on demand; `MASTERDATA_NOT_INITIALIZED` stays the ledger's code for a
  home without a chart.
