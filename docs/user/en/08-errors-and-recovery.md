# 8. Errors and recovery

## How an error appears

When a command fails, it prints the error with its code, for example:

```json
{"error_code":"PERIOD_SEALED","error_kind":"user","message":"PERIOD_SEALED: period 2026-03 is Sealed; corrections go to the next open period"}
```

- `error_code` is a stable code that tells the agent what to do.
- `error_kind` is `user` when the cause is in the input or in the state of the
  books and can be fixed (exit status 1), or `system` when the cause is in the
  environment, such as a file or a connection (exit status 2).
- `message` describes the cause in words.

Most errors are deliberate stops: the program refuses to do something that
would make the books wrong. A refusal leaves the books unchanged.

## The most common errors

| Code | What happened | What to do |
|---|---|---|
| `MASTERDATA_NOT_INITIALIZED` | the home has no chart of accounts | wrong home, or `init` not run; check `aggountant config show` |
| `ALREADY_INITIALIZED` | `init` was run on an existing home | nothing; the home already exists |
| `INVALID_INPUT` | the input is malformed or has an unknown field | the agent fixes the input |
| `PERIOD_SEALED` | the period is closed | book into the next open period |
| `PERIOD_CLOSING` | the period's close is in progress | approve the close or book into the next period (chapter 4) |
| `PERIOD_NOT_SEALED` | VAT preview of an open period | close the period first |
| `PERIOD_TRANSITION_CONFLICT` | the period's state does not allow the requested change, for example the period has already been filed, or months and quarters are being mixed in the same year | read the message; a filed period is not filed again |
| `OPEN_OBLIGATION` | an open obligation blocks the period's close | deal with the matter and mark the obligation resolved |
| `CAPTURE_PREPARATION_CHANGED` | the receipt's details changed after approval | new preview and new approval |
| `APPROVAL_REQUIRED` | booking was attempted without an approved preview | preview first |
| `OUTPUT_EXISTS` | the report file already exists | give a new file name |
| `VAT_FIGURES_MISMATCH` | the VAT return figures do not match the books | use the preview's figures |
| `VERO_NOT_READY` | Vero.fi readiness is not in order | `aggountant vero-fi status` (chapter 5) |
| `GATE_NOT_OPEN` | the approval has already been answered | check the run's state with `run show` |
| `WORKFLOW_VERSION_MISMATCH` | an unfinished run was started with an older program version | see "Updating the program" below |

## An interrupted run

Close, filing and year-end runs store every step. If the machine crashes or the
connection drops midway, nothing is lost. The run is continued like this:

| Run | How to continue |
|---|---|
| period close | `aggountant run resume --run-id cp-…` |
| ALV return, uncertain submission | `aggountant file-vat reconcile --run-id fv-…` (chapter 5) |
| Tulorekisteri report, uncertain submission | `aggountant file-wage reconcile wr-…` (chapter 6) |
| year-end close | `aggountant close-year resume --run-id cy-…` |

Resuming first checks the actual state of the books. If, for example, the
period was sealed before the crash, the run notices and does nothing a second
time. A return to an authority is never resent automatically.

`aggountant run list` shows all runs, and `aggountant run show --run-id …`
shows a single run's steps and any approvals it is waiting for.

## A voucher sealed by mistake

A sealed voucher is never edited or deleted. The mistake is corrected with a
new correcting voucher in an open period:

- **wrong account or VAT treatment**: a voucher that moves the amount to the
  right account
- **a transaction booked twice, or one that does not belong to the company**: a
  reversing entry that cancels the original
- **wrong amount**: an entry for the difference.

The agent prepares the correcting voucher as a special entry, refers to the
voucher being corrected in the description, and shows it to you for approval.
The program has no correction command of its own yet. The original and the
correction both stay in the books. This is exactly what the Accounting Act
requires.

## The chain check fails

`aggountant voucher verify` checks that no sealed voucher has been changed or
deleted. If the check fails, the voucher database has been handled by something
other than aggountant, or the file is damaged. Do not try to repair the
database by hand. Restore the home from a backup (chapter 2) and run the check
again.

## Vouchers seem to have disappeared

First check which home the program found: `aggountant config show`. A mistyped
`--home` path, or a command run from the wrong directory, can point to an empty
or newly created home. Your real books are then safe somewhere else.

## A period is stuck in closing

If a period's close has been rejected (`--answer reject`), the period stays in
`closing`, and in this version it cannot be reopened or closed again. Nothing
can be booked into it either. This is a known shortcoming. That is why
chapter 4 advises: start the close only when the period is ready, and correct
anything you notice afterwards in the next period.

## Updating the program

When you update aggountant:

1. Reinstall the skills (`aggountant skill install`), because the instructions
   are written for the commands of the same version.
2. If a run was unfinished and resuming it gives `WORKFLOW_VERSION_MISMATCH`,
   the run's definition has changed between versions. The agent can move the
   run to the new version with `aggountant run migrate --run-id … --confirm`.
   The `--confirm` option is required because the move permanently changes the
   run's version.
3. If the new version expects something in the home that the old home does not
   contain, there is no separate upgrade command yet. Do not copy files by
   hand; ask for help in the project's issue tracker.

## The test environment variable

The environment variable `AGGOUNTANT_BROKER_SCRIPT` is meant only for testing
the program. It replaces the Tax Administration's responses with responses
written in advance. **If it is set, `file-vat` can mark a period as filed even
though nothing was sent.** Make sure the variable is not set when you file a
real return.
