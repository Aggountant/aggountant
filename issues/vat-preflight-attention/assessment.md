# Assessment of the vat-preflight-attention review

The review confirmed no findings, so nothing needs triage.

FIX: 0   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

The review lists two residuals as outside scope. Neither is filed:

- A state write failing after the filing authority is acquired: local disk
  failure, the end result is a stuck Run rather than wrong bookkeeping, and
  no source we use produces it.
- The projection refusal has no test: the code path is the same
  journal-then-return shape that the preflight tests cover.

## Simplicity

- Reused: the existing `EffectRejected` event on the `reconcile` effect and
  the existing fold rule (a rejected effect means `NeedsAttention`). There is
  no new event, Run state or retry path.
- Repetition: the preflight moved into one function, `read_fileable_vero_period`,
  so its four failure kinds (read failed, malformed, period missing, not
  fileable) each carry their code in one place. The journal-then-return
  pattern now appears twice more in `start_file_vat`, matching the reconcile
  rejections already there. A generic "reject on any error" wrapper around
  the whole function was rejected: it would also have to treat the failure
  after `acquire_filing_authority` differently.
- Left out: putting the run id in a structured error field (the message names
  it), and a CLI `journal show` command for the rejection code (the code is in
  the error message).
