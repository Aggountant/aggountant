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
| Tulorekisteri report (`wr-…`), uncertain submission | `aggountant file-wage reconcile wr-…` |
| year-end (`cy-…`) | `aggountant close-year resume --run-id cy-…` |

**Do not** start a filing again from the beginning. A filing is never sent
again automatically, and it should not be sent by hand either until you know
whether the first one got through.

## A month is stuck in closing

**What probably happened.** The month's close was rejected. A rejected close
leaves the month permanently in the state `closing`: nothing can be recorded
in it, and it cannot be closed again (`PERIOD_TRANSITION_CONFLICT`). This is a
known gap.

**Check.** Ask the agent to show the month's state. If it is `closing` and the
close run is `rejected`, this is the case.

**Do this.** In this version the month cannot be made open again. Record the
missing and corrected items in the next month, and report the situation in
the project's [issue tracker](https://github.com/aggountant/aggountant/issues).
The month's VAT return must still be made in OmaVero. aggountant does not
calculate its figures or record it as filed, because the month has not been
closed (`PERIOD_NOT_SEALED`), so ask the agent to calculate the figures from
the month's vouchers and check them yourself.

**Do not** try to change the state by hand in the files. From now on, start a
close only when the month is finished, and approve a close you have started
([Close the month](tasks/close-month.md)).

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
of double booking is a purchase recorded first from the receipt against the
bank account and then again from the statement row as an expense. The bank
statement import requires an entry for every row, including the payment of a
receipt already recorded
([What is ready and what is not](reference/status.md#bank-statement)).

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
  after the acknowledgement whose figures differ from the approved ones.
  Compare it in OmaVero.

The acknowledgement proves the filing, not whether the return shows in
OmaVero. If the status query fails (`VAT_STATUS_UNKNOWN`), the status is only
unknown: the return has still been filed. If the outcome of the submission is
uncertain, the command points to `file-vat reconcile`
([below](#the-outcome-of-a-filing-is-uncertain)).

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
returned an error during sending, and the program does not know whether the
return got through. The run stays in a state that needs attention
([Filings](concepts/filings.md#uncertain-outcome)).

**Check.** Ask the agent to query the status from the authority. The query
does not send the return.

**Do this.**

- If the authority has exactly the same return, the run ends without a new
  submission.
- If the return does not show or differs, wait and ask the agent to query
  again later, or check OmaVero or Tulorekisteri's e-service yourself.
- If you decide to file by hand, first make sure the API submission did not
  get through. Recording a VAT return made by hand then requires evidence of
  the Tax Administration's status.

**Do not** repeat the approval or start the filing again. Repeating the
approval does not send again, and a new run could produce a duplicate return.
`file-vat retry` applies only to the payment step, not to the return.

## Tulorekisteri rejected the report

**What probably happened.** Tulorekisteri has processed the report and
rejected it. The run stays in a state that needs attention. The program does
not show Tulorekisteri's error text, because it may contain a personal
identity code.

**Check.** See the exact error in Tulorekisteri's e-service.

**Do this.** Make the report or the correction in Tulorekisteri's e-service.
Then give the agent the delivery and report identifiers the service gave you,
and the agent records them in aggountant
(`aggountant file-wage complete-manual wr-… --ir-delivery-id … --ir-report-id …`).
The same fallback works when the API route does not succeed for other
reasons.

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

**Check.** Ask the agent to check whether the variable is set.

**Do this.** Unset the variable and file the return in OmaVero
([File the VAT return in OmaVero](tasks/file-vat-omavero.md)). Marking the
period as filed is permanent, so report the situation in the project's
[issue tracker](https://github.com/aggountant/aggountant/issues).

**Do not** file real returns while the variable is set.

## See also

- [Error codes](reference/error-codes.md)
- [What is ready and what is not](reference/status.md)
- [Filings and uncertain outcomes](concepts/filings.md)
