# When something goes wrong

This page is organised by what you notice. For each symptom it says what
probably happened, what to check and what to do, and where a wrong reaction
cannot be undone, also what not to do. If there is an error code on screen,
see also [Error codes](reference/error-codes.md).

Most errors are deliberate stops: the program refuses to do something that
would make the books wrong. A refusal leaves the books as they were.

## A run was interrupted

**What probably happened.** The computer crashed, the connection dropped or
the agent's session ended in the middle of a month's close, a filing or the
year-end run. Runs save every step, so nothing was lost.

**Check.** Ask the agent to show the runs and their state: "show the runs in
progress".

**Do this.** Ask the agent to continue the run. Continuing first checks the
actual state of the books. If, for example, the month's close had already
happened before the crash, the run notices that and does nothing a second
time.

| Run | Continuing |
|---|---|
| month's close (`cp-…`) | `aggountant run resume --run-id cp-…` |
| VAT return (`fv-…`), uncertain submission | `aggountant file-vat reconcile --run-id fv-…` |
| VAT return (`fv-…`), interrupted while sending | `aggountant file-vat status --period YYYY-MM`, then the command it names ([below](#a-vat-filing-was-interrupted-while-sending)) |
| Tulorekisteri report (`wr-…`), uncertain submission | `aggountant file-wage reconcile wr-…` |
| a correction of a Tulorekisteri report (`wc-…`), uncertain submission | `aggountant file-wage correction status wr-…` |
| employer's separate report (`er-…`), uncertain submission | `aggountant file-wage employer-report status --month YYYY-MM` |
| year-end (`cy-…`) | `aggountant close-year resume --run-id cy-…` |

**Do not** start a filing again from the beginning. A filing is never sent
again automatically, and it should not be sent by hand either until you know
whether the first one got through.

## A month is stuck in closing

**What probably happened.** The month's close is still waiting for your
answer, or the close was rejected with an earlier version, or the computer
crashed during the rejection. An entry is refused with `PERIOD_CLOSING`, and
a new close with `PERIOD_TRANSITION_CONFLICT`.

**Check.** Ask the agent to show the month's state and the state of the close
run.

**Do this.** If the close run is `awaiting_gate`, approve or reject the close
([Close the month](tasks/close-month.md)). If it is `rejected`, ask the agent
to continue the run (`aggountant run resume --run-id cp-…`): continuing
finishes the rejection and returns the month to `open`. You can then record
in the month and start a new close.

**Do not** try to change the state by hand in the files.

## Vouchers seem to have disappeared

**What probably happened.** The command hit the wrong home. A mistyped
`--home` path or a command run from the wrong directory can point to an empty
or newly created home. A wrong path can even create an empty directory (a
known gap). The real books are then safe somewhere else. The same cause is
often behind the code `MASTERDATA_NOT_INITIALIZED`.

**Check.** Ask the agent to run `aggountant config show`. It tells you which
home is in use and where that came from (option, environment variable,
configuration file or directory marker).

**Do this.** Fix the path or set the right home as the default
([The company home](concepts/company-home.md#how-aggountant-finds-the-home)).
An empty directory created in the wrong place can be deleted once you have
made sure it holds no vouchers.

**Do not** run `init` again to "fix" the home. It does not touch an existing
home, but in the wrong place it sets up new, empty books.

## The bank balance does not match the statement

**What probably happened.** A transaction on the statement is missing from the
books, or the same transaction has been recorded twice. The most common cause
of double booking is a receipt recorded against the bank account even though
its payment is also recorded from the statement. A receipt's counter-account
is trade payables 2700 (or 2740), and the statement row clears it
([When the purchase is also on the bank statement](tasks/record-receipts.md#when-the-purchase-is-also-on-the-bank-statement)).
If the bank balance matches but trade payables hold an amount no supplier is
owed, the same purchase was probably recorded as a payable from the receipt
and again as an expense from the statement row.

**Check.** Ask the agent to compare the month's bank account postings with the
statement rows and find the transactions that are missing or appear twice.
The difference is often exactly the amount of one receipt.

**Do this.** Record the missing transaction. Reverse a transaction recorded
twice with a reversing entry
([Correct a wrong voucher](tasks/correct-voucher.md)). Do this before the
month's close, so the correction lands in the same month.

## The return does not show in OmaVero

**What probably happened.** The Tax Administration has received the return
but has not processed it yet. A return sent through the API has usually shown
in OmaVero only about half a day later
([Filings](concepts/filings.md#received-is-not-processed)).

**Check.** Ask the agent to query the return's status
(`aggountant file-vat status --period 2026-03`). The command only reads: it
sends nothing and changes neither the books nor the run. When the period has
an acknowledgement, the command asks the Tax Administration how the return
shows there and reports one status:

- `received`: there is an acknowledgement, but the Tax Administration does
  not show the return yet.
- `pending_authority_processing`: the Tax Administration is processing the
  return or still shows an earlier one.
- `processed`: the Tax Administration has processed the return with the
  approved figures.
- `processed_values_differ`: the Tax Administration has processed a return
  after the acknowledgement whose figures differ from the approved ones. The
  answer lists each differing field with the approved and the Tax
  Administration's amount (`differences`, also in the message). A field the
  Tax Administration shows as zero or leaves out is not a difference when the
  approved figure is zero too. Compare the listed fields in OmaVero.

The acknowledgement proves the filing, not whether the return shows in
OmaVero. If the status query fails (`VAT_STATUS_UNKNOWN`), the status is only
unknown: the return has still been filed. If the outcome of the submission is
uncertain, the command points to `file-vat reconcile`
([below](#the-outcome-of-a-filing-is-uncertain)). If the run was interrupted
while sending, the command asks the Tax Administration about that run
([below](#a-vat-filing-was-interrupted-while-sending)).

**Do this.** Wait until the next working day. If the return still does not
show, or the due date is close, call the Tax Administration's VAT service.
The agent gathers for the call the period, the acknowledgement's identifier
and timestamp, the Business ID and the fields that were sent.

**Do not** send the return again because it does not show yet. A second
submission would be a replacement return at the Tax Administration, and
aggountant does not even allow a second submission for the same period
(`PERIOD_TRANSITION_CONFLICT`).

## The outcome of a filing is uncertain

**What probably happened.** The connection dropped or the authority's service
returned an error during sending, or it reported success without a readable
receipt, and the program does not know whether the return got through. The run stays in a state that needs attention
([Filings](concepts/filings.md#uncertain-outcome)).

**Check.** Ask the agent to query the status from the authority. The query
does not send the return.

**Do this.**

- If the authority has exactly the same return, the run ends without a new
  submission.
- If the return does not show or differs, wait and ask the agent to query
  again later, or check OmaVero or Tulorekisteri's e-service yourself.
- For a VAT return, two "not found" answers at least one bank day apart close
  the run as not filed. Then start a new filing run for the period. The
  command's response tells from which moment the second query counts
  (`next_check_from`): the same time of day on the next bank day.
- If you decide to file by hand, first make sure the API submission did not
  get through. Recording a VAT return made by hand then requires evidence of
  the Tax Administration's status.

**Do not** repeat the approval or start the filing again before that.
Repeating the approval does not send again, and aggountant refuses a new VAT
filing run for the same period (`VAT_FILING_UNRESOLVED`), because it could
produce a duplicate return.

## A VAT filing was interrupted while sending

**What probably happened.** The computer crashed or the session ended while
the VAT return was being sent, before its outcome reached the books. The return
may or may not have got through. The run shows the filing as still in
progress, and aggountant refuses a new filing run for the period
(`VAT_FILING_UNRESOLVED`).

**Check.** Ask the agent to query the status
(`aggountant file-vat status --period 2026-03`). For an interrupted run the
command looks at what the run left behind and asks the Tax Administration
(`GetFiledVATReturn`). It only reads; nothing is sent or recorded. It reports:

- `interrupted_filed`: the Tax Administration accepted the return; its
  acknowledgement was saved before the interruption, or the Tax
  Administration shows the approved return processed.
- `interrupted_not_filed`: the return never reached the Tax Administration
  (or it refused it outright), and the Tax Administration has no return for
  the period.
- `filing_interrupted`: not settled yet. Either the return may have reached
  the Tax Administration, which does not show it yet, or the Tax
  Administration shows something else for the period.

**Do this.** Ask the agent to run the command the status names
(`aggountant file-vat reconcile --run-id fv-…`). It records the answer:

- If the return was accepted, the filing and its acknowledgement, when there
  is one, are recorded, and the run continues to the payment step. Nothing is
  sent.
- If the return never reached the Tax Administration, the run is closed as
  not filed, with the Tax Administration's answer stored in the run. Start a
  new filing run for the period; it sends the return once.
- If the return may have reached the Tax Administration but does not show,
  it is treated like an [uncertain outcome](#the-outcome-of-a-filing-is-uncertain):
  a return can take days to show, so two "not found" answers at least one
  bank day apart are needed before the run is closed.

If the run sent nothing but the Tax Administration shows the approved return
processed, the return was filed some other way. Confirm it in OmaVero and
record it as a manual filing (`file-vat complete-manual`, citing the Tax
Administration's answer); `file-vat reconcile` then closes the interrupted run
without claiming that filing.

If the query fails (`VAT_STATUS_UNKNOWN`), nothing changes; ask again later.

**Do not** start a new filing run or file by hand before the status has said
what happened. An unknown answer is never a reason to file again.
`file-vat retry` applies only to the payment step, not to the return.

## Tulorekisteri rejected the report

**What probably happened.** Tulorekisteri has processed the report and
rejected it. The run stays in a state that needs attention. The program does
not show Tulorekisteri's error text, because it may contain a personal
identity code.

**Check.** See the exact error in Tulorekisteri's e-service.

**Do this.** If the rejected report was a travel allowance report, make it in
Tulorekisteri's e-service. Then give the agent the delivery and report
identifiers the service gave you, and the agent records them in aggountant
(`aggountant file-wage complete-manual wr-… --ir-delivery-id … --ir-report-id …`).
The same fallback works when the API route does not succeed for other
reasons. The command records only a report that, as far as aggountant knows,
Tulorekisteri has not yet accepted; for an accepted report it refuses
(`WAGE_REPORT_ALREADY_ACCEPTED`), and the correction is made with the program
([Report travel allowances to Tulorekisteri](tasks/report-travel-allowances.md#correct-an-accepted-report)).
If what was rejected was a correction, the original report is still in the
register as it was: fix the cause and ask the agent to preview the correction
again. An employer's separate report can also be previewed and sent again
once the cause is fixed, or the reference of the report filed in the e-service
is recorded
(`aggountant file-wage employer-report complete-manual --month … --ir-report-id …`).

**Do not** make a new original report in the e-service before you have
checked whether an uncertain API submission already got through. Do not
change the payment identifier or the payment date either.

## The VAT return run waits for payment

**What probably happened.** The return has been sent, and the run opened a
payment approval because there is tax to pay. aggountant has no payment
connection, so this step never completes.

**Do this.** Pay the tax yourself with the OmaVero reference. The month has
been filed (`vat_filed true`), and the waiting payment step can stay as it
is.

## The chain check fails

**What probably happened.** The voucher database has been handled by
something other than aggountant, or the file is damaged.
`aggountant voucher verify` checks that no sealed voucher has been changed or
deleted.

**Do this.** Restore the home from a backup and run the check again
([Back up and restore your books](tasks/back-up-and-restore.md)).

**Do not** try to fix the database by hand. It will not make the chain whole
again, and you may lose the record of what happened.

## A run does not continue after an upgrade

**What probably happened.** The run was started with an older version of the
program, and its definition has changed (`WORKFLOW_VERSION_MISMATCH`).

**Do this.** Give the agent permission to move the run to the new version
([Upgrade aggountant](tasks/upgrade.md)).

## A return was marked filed although nothing was sent

**What probably happened.** The environment variable `AGGOUNTANT_BROKER_SCRIPT`
is set. It is meant only for testing the program and replaces the Tax
Administration's responses with pre-written ones. When it is set, `file-vat`
can mark a period as filed even though nothing was sent.

**Check.** The command says so itself: its output has the field `test_seam`
(an error too), and without `--json` a warning starting
`warning: AGGOUNTANT_BROKER_SCRIPT is set` goes to standard error. Ask the
agent to check whether the variable is set.

**Do this.** Unset the variable and file the return in OmaVero
([File the VAT return in OmaVero](tasks/file-vat-omavero.md)). Marking the
period as filed is permanent, so report the situation in the project's
[issue tracker](https://github.com/aggountant/aggountant/issues).

**Do not** file real returns while the variable is set.

## See also

- [Error codes](reference/error-codes.md)
- [What is ready and what is not](reference/status.md)
- [Filings and uncertain outcomes](concepts/filings.md)
