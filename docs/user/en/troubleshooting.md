# When something goes wrong

Find what you noticed below. If there is an error code on screen, see also
[Error codes](reference/error-codes.md).

Most errors are deliberate stops: aggountant refuses to do something that
would make the books wrong. A refusal leaves the books as they were.

## A run was interrupted

**What probably happened.** The computer crashed, the connection dropped or
the agent's session ended in the middle of a month's close, a filing or the
year-end run. A run saves every step, so nothing was lost.

**Check.** Ask the agent to show the runs and their state: "show the runs in
progress".

**Do this.** Ask the agent to continue the run. The run first checks the
actual state of the books. If, for example, the month had already closed
before the crash, the run notices that and does nothing a second time.

| Run | Continuing |
|---|---|
| month's close (`cp-…`) | `aggountant run resume --run-id cp-…` |
| VAT return (`fv-…`), stopped before sending (`running`, `reconcile` `executing`) | `aggountant run resume --run-id fv-…` |
| VAT return (`fv-…`), uncertain submission | `aggountant file-vat reconcile --run-id fv-…` |
| VAT return (`fv-…`), interrupted while sending | `aggountant file-vat status --period YYYY-MM`, then the command it names ([below](#a-vat-filing-was-interrupted-while-sending)) |
| Tulorekisteri report (`wr-…`), uncertain submission | `aggountant file-wage reconcile wr-…` |
| a correction of a Tulorekisteri report (`wc-…`), uncertain submission | `aggountant file-wage correction status wr-…` |
| employer's separate report (`er-…`), uncertain submission | `aggountant file-wage employer-report status --month YYYY-MM` |
| year-end (`cy-…`) | `aggountant close-year resume --run-id cy-…` |

A VAT return run that stopped before anything was sent ends as `not_filed`
when resumed, with the code `VAT_INTERRUPTED_BEFORE_FILING` on its
`reconcile` step. A run in the state `not_filed` is finished: the return was
not filed by it and nothing was sent, so it needs nothing and does not stop a
new run for the period. If the run may already have begun sending, resuming
refuses (`VAT_FILING_UNRESOLVED`) and names `file-vat status`.

**Do not** start a filing again from the beginning. The program does not
send a filing to an authority again on its own, and you should not send one
by hand either until you know whether the first one got through.

## A month is stuck in closing

**What probably happened.** The month's close is still waiting for your
answer. The other possibility is that the close was rejected with an earlier
version, or the computer crashed during the rejection. The program refuses to
record in the month with `PERIOD_CLOSING` and to start a new close with
`PERIOD_TRANSITION_CONFLICT`.

**Check.** Ask the agent to show the month's state and the state of the close
run.

**Do this.** If the close run's state is `awaiting_gate`,
[approve or reject the close](tasks/close-month.md). If it is `rejected`, ask
the agent to continue the run (`aggountant run resume --run-id cp-…`). The run
then finishes the rejection and returns the month to `open`, and you can again
record in the month and start a new close.

**Do not** try to change the state by hand in the files.

## Vouchers seem to have disappeared

**What probably happened.** The command hit the wrong home. A mistyped
`--home` path or a command run from the wrong directory can point to another
home, or to a directory that holds no books. The real books are safe where
they have been all along. Where no home has been set up, the command stops
with the code `HOME_NOT_INITIALIZED` and writes nothing. The error message
names the path the command looked at and how that path was chosen.

**Check.** Ask the agent to run `aggountant config show`. It tells you which
home is in use and where that came from: the option, the environment
variable, the configuration file, a directory marker, or the default, which is
the `accounting` subdirectory.

**Do this.** Fix the path or
[set the right home as the default](concepts/company-home.md#how-aggountant-finds-the-home).
Earlier versions could leave an empty home behind in the wrong place (an
`accounting` directory with `fiscal.sqlite` and `artifacts/`). You can delete
it once you have checked that it holds no vouchers.

**Do not** run `init` again to "fix" the home. It does not touch an existing
home, but in the wrong place it sets up new, empty books.

## The bank balance does not match the statement

**What probably happened.** A transaction on the statement is missing from the
books, or the same transaction has been recorded twice. The usual double
booking is a purchase whose receipt was recorded against the bank account even
though its payment is also recorded from the statement. A receipt's
counter-account is trade payables 2700 (or 2740), and the statement row clears
the payable
([When the purchase is also on the bank statement](tasks/record-receipts.md#when-the-purchase-is-also-on-the-bank-statement)).

If the bank balance matches but trade payables hold an amount no supplier is
owed, the same purchase was probably recorded as a payable from the receipt
and again as an expense from the statement row.

**Check.** Ask the agent to compare the month's bank account postings with the
statement rows and find the transactions that are missing or appear twice.
The difference is often exactly the amount of one receipt.

**Do this.** Ask the agent to record the missing transaction and to cancel
the one recorded twice with a
[reversing entry](tasks/correct-voucher.md). Do this before the month's
close, so the correction lands in the same month.

## The return does not show in OmaVero

**What probably happened.** The Tax Administration has received the return
but has not processed it yet. A return sent through the API has usually shown
in OmaVero only about half a day later
([Received is not processed](concepts/filings.md#received-is-not-processed)).

**Check.** Ask the agent to query the return's status
(`aggountant file-vat status --period 2026-03`). The command sends nothing and
changes neither the books nor the run. When the period has an
acknowledgement, the command asks the Tax Administration how the return shows
there and reports one status:

- `received`: there is an acknowledgement, but the Tax Administration does
  not show the return yet.
- `pending_authority_processing`: the Tax Administration is processing the
  return or still shows an earlier one.
- `processed`: the Tax Administration has processed the return with the
  figures you approved.
- `processed_values_differ`: after the acknowledgement the Tax Administration
  has processed a return whose figures differ from the ones you approved. The
  answer lists each differing field with both amounts (`differences`, also in
  the message). Compare those fields in OmaVero. A field the Tax
  Administration leaves out is not a difference when the figure you approved
  is zero.

The acknowledgement proves the return was filed, even if it does not show in
OmaVero yet. If the status query fails (`VAT_STATUS_UNKNOWN`), the status
could not be found out, but the return has still been filed. If the outcome
of the submission is uncertain, the command points to `file-vat reconcile`
([The outcome of a filing is uncertain](#the-outcome-of-a-filing-is-uncertain)).
If the run was interrupted while sending, the command asks the Tax
Administration about that run's return
([A VAT filing was interrupted while sending](#a-vat-filing-was-interrupted-while-sending)).

**Do this.** Wait until the next working day. If the return still does not
show, or the due date is close, call the Tax Administration's VAT service.
For the call the agent gathers the period, the acknowledgement's identifier
and timestamp, the Business ID and the fields that were sent.

**Do not** send the return again because it does not show yet. A second
submission would be a replacement return at the Tax Administration, and
aggountant refuses a new filing run for a period that has already been filed
(`PERIOD_TRANSITION_CONFLICT`).

## The outcome of a filing is uncertain

**What probably happened.** The connection dropped or the authority's service
returned an error during sending, or the service reported success without a
readable receipt. The program does not know whether the filing got through,
and the run stays in
[a state that needs attention](concepts/filings.md#uncertain-outcome).

**Check.** Ask the agent to query the status from the authority. The query
does not send the filing.

**Do this.**

- If the authority has exactly the same filing, the run ends without a new
  submission.
- If the filing does not show or differs, wait and ask the agent to query
  again later, or check OmaVero or Tulorekisteri's e-service yourself.
- For a VAT return, the run closes as not filed once the Tax Administration
  has answered "not found" twice, at least one bank day apart. The command's
  response says from which moment the second query counts
  (`next_check_from`): the same time of day on the next bank day. After the
  run has closed, start a new filing run for the period.
- If you decide to file by hand, first make sure the API submission did not
  get through. Recording a VAT return filed by hand then requires evidence of
  the Tax Administration's status.

**Do not** repeat the approval or start the filing again before that.
Repeating the approval does not send again, and aggountant refuses a new VAT
filing run for the same period (`VAT_FILING_UNRESOLVED`), because it could
produce a duplicate return.

## A VAT filing was interrupted while sending

**What probably happened.** The computer crashed or the session ended while
the VAT return was being sent, and the outcome was not saved. The return may
or may not have got through. The run shows the filing as still in progress,
and aggountant refuses a new filing run for the period
(`VAT_FILING_UNRESOLVED`).

**Check.** Ask the agent to query the status
(`aggountant file-vat status --period 2026-03`). The command looks at what
the interrupted run left behind and asks the Tax Administration. It sends
nothing and records nothing. It reports one status:

- `interrupted_filed`: the Tax Administration accepted the return. Either the
  acknowledgement was saved before the interruption, or the Tax
  Administration shows the return you approved as processed.
- `interrupted_not_filed`: the return did not reach the Tax Administration,
  or the Tax Administration refused it outright, and the Tax Administration
  has no return for the period.
- `filing_interrupted`: not settled yet. The return may have reached the Tax
  Administration, which does not show it yet, or the Tax Administration shows
  something else for the period.

**Do this.** Ask the agent to run the command the status names
(`aggountant file-vat reconcile --run-id fv-…`). It records the answer:

- If the Tax Administration accepted the return, the command records the
  filing and its acknowledgement, when there is one, and the run continues to
  the payment step. Nothing is sent.
- If the return did not reach the Tax Administration, the command closes the
  run as not filed and stores the Tax Administration's answer in the run.
  Start a new filing run for the period. It sends the return once.
- If the return may have reached the Tax Administration but does not show,
  proceed as with an
  [uncertain outcome](#the-outcome-of-a-filing-is-uncertain). A return can
  take days to show, so the run closes only after the second "not found"
  answer.

If the run sent nothing but the Tax Administration shows the return you
approved as processed, the return was filed some other way. Confirm it in
OmaVero and ask the agent to record it as filed by hand
(`file-vat complete-manual`, with the Tax Administration's answer as
evidence). After that, `file-vat reconcile` closes the interrupted run without
claiming that filing.

If the query fails (`VAT_STATUS_UNKNOWN`), nothing changes. Ask again later.

**Do not** start a new filing run or file by hand before the status has said
what happened. An unknown answer is not a reason to file again.
`file-vat retry` applies only to the payment step, not to the return.

## Tulorekisteri rejected the report

**What probably happened.** Tulorekisteri has processed the report and
rejected it. The run stays in a state that needs attention. The program does
not show Tulorekisteri's error text, because it may contain a personal
identity code.

**Check.** See the exact error in Tulorekisteri's e-service.

**Do this.** What you do depends on which report was rejected:

- A travel allowance report: file it in Tulorekisteri's e-service and give
  the agent the delivery and report identifiers the service gave you. The
  agent records them in aggountant
  (`aggountant file-wage complete-manual wr-… --ir-delivery-id … --ir-report-id …`).
  The same fallback works when the API submission fails for some other
  reason. The command records only a report that, as far as aggountant knows,
  Tulorekisteri has not yet accepted. For an accepted report it refuses
  (`WAGE_REPORT_ALREADY_ACCEPTED`), and
  [the correction is made with the program](tasks/report-travel-allowances.md#correct-an-accepted-report).
- A correction: the original report is still in the register as it was. Fix
  the cause of the rejection and ask the agent to preview the correction
  again.
- An employer's separate report: fix the cause of the rejection and ask the
  agent to preview and send the report again. You can also file the report in
  the e-service, and the agent then records its reference
  (`aggountant file-wage employer-report complete-manual --month … --ir-report-id …`).

**Do not** make a new original report in the e-service before you have
checked whether an uncertain API submission already got through. Do not
change the payment identifier or the payment date to get round the rejection
either, because that would quietly create a new report.

## The VAT return run waits for payment

**What probably happened.** The return was sent with an earlier version of
aggountant, which left a return with tax to pay waiting at the payment
approval `ApprovePayment`. aggountant has no payment connection, so that
approval cannot go through. The month has been filed (`vat_filed true`).

**Do this.** Pay the tax yourself with the OmaVero reference. The agent ends
the run with `file-vat user-pays --run-id fv-…`; it sends nothing and records
that you pay the tax, not that it has been paid.

## The chain check fails

**What probably happened.** The voucher database has been handled with
something other than aggountant, or the file is damaged.
`aggountant voucher verify` checks that no sealed voucher has been changed or
deleted.

**Do this.**
[Restore the home from a backup](tasks/back-up-and-restore.md) and run the
check again.

**Do not** try to fix the database by hand. That does not make the chain
whole again, and you may lose the record of what happened.

## A run does not continue after an upgrade

**What probably happened.** The run was started with an older version of the
program, and the run's definition has changed since
(`WORKFLOW_VERSION_MISMATCH`).

**Do this.** Give the agent permission to
[move the run to the new version](tasks/upgrade.md).

## A return was marked filed although nothing was sent

**What probably happened.** The environment variable `AGGOUNTANT_BROKER_SCRIPT`
is set. It is meant only for testing the program and replaces the Tax
Administration's responses with pre-written ones. `file-vat` can then mark a
period as filed even though nothing was sent.

**Check.** Ask the agent to check whether the variable is set. The command
also says so itself: its output has the field `test_seam` (an error too), and
without the `--json` option a warning starting
`warning: AGGOUNTANT_BROKER_SCRIPT is set` goes to standard error.

**Do this.** Unset the variable and
[file the return in OmaVero](tasks/file-vat-omavero.md). Marking the period as
filed is permanent, so report the situation in the project's
[issue tracker](https://github.com/aggountant/aggountant/issues).

**Do not** file real returns while the variable is set.

## See also

- [Error codes](reference/error-codes.md)
- [What is ready and what is not](reference/status.md)
- [Filings and uncertain outcomes](concepts/filings.md)
