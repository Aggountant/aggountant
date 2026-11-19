# Result: perfectly-uppity-knife

An interrupted `file-vat` Run no longer leaves the user stuck. There is still
no resume command: `file-vat status` asks what happened and names
`file-vat reconcile`, which records the answer.

## What changed

- `file-vat status` for an interrupted Run (filing effect `executing`, period
  not filed) reads the period's outbox and GetFiledVATReturn/v2, records
  nothing and sends nothing. New statuses: `interrupted_filed` (an outbox
  receipt, or the approved return processed), `interrupted_not_filed` (no
  request for the period could have been accepted, and the authority has no
  return); otherwise `filing_interrupted`. `next_command` is
  `aggountant file-vat reconcile --run-id …` when reconcile can act. A failed
  read is `VAT_STATUS_UNKNOWN` and changes nothing.
- `file-vat reconcile` accepts an interrupted Run:
  - fiscal filing already landed → the Run catches up, no read;
  - receipt in the outbox → the receipt is recorded in the ledger, no read and
    no FileVATReturn request;
  - request may have arrived (an `Unknown` attempt, or a `MALFORMED`/`HTTP_2…`
    answer) → the unknown-outcome rule: adopt a matching processed return, or
    close after two `Missing` answers at least one bank day apart;
  - no request could have been accepted → one `Missing` closes the Run as not
    filed (`NOT_FILED` reason and the answer in the journal), and a new Run
    files once; a matching processed return is reported as filed elsewhere,
    never adopted;
  - a manual filing recorded meanwhile → the Run is closed as `superseded`
    without claiming it.
- The `VAT_FILING_UNRESOLVED` message for an interrupted Run now points to
  `file-vat status`. A receipted filing whose Run lags names `reconcile` too.
- Seams `CrashBeforeFilingDispatch` and `CrashAfterFilingDispatch` for tests.
- User guide (fi, en): troubleshooting section "A VAT filing was interrupted
  while sending", table row, error codes, direct-filing page; `vero-fi` skill.

## Decision taken

The product owner's direction says a "not there" answer closes the Run. Where
the Run's request may have reached the Tax Administration, a single `Missing`
does not prove the return was lost (broker rule, and the existing reconcile
rule for unknown outcomes), so that case keeps the two-answers-a-bank-day-apart
rule. A Run whose request provably never left is closed on one answer.

## Verification

Tests in `crates/process/tests/file_vat.rs` (scripted broker) and
`file_vat_status.rs` (loopback mock, FileVATReturn request count) cover the
three acceptance cases plus the review findings. `scripts/ci-local.sh` passed.
Review and assessment: `review.md`, `assessment.md` in this directory.
