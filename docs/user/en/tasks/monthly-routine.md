# Do a month's bookkeeping from start to finish

A month's bookkeeping goes in the same order every month. This page shows the
order and takes you to each step's own page. If the material is all there, an
ordinary month is recorded, closed and filed in one session.

## Before you start

- Your company's books are set up
  ([Set up your company's books](../start/set-up-company.md)).
- The month's bank statement is available as a camt.053 or camt.052 file, and
  the receipts are where the home's `AGENTS.md` says.

## How to do it

1. **Ask the agent to handle the whole month.**

   > Let's do March's books.

   The agent uses the `capture-receipt` skill. It does on its own whatever the
   sources are enough for, and brings you the questions and the vouchers to
   approve all together, not one receipt at a time.

2. **Receipts and invoices.** Every receipt and purchase invoice is recorded
   as a voucher of its own. One paid from the company's account is recorded
   as a trade payable, one paid with your own money as a debt to you:
   [Record receipts and purchase invoices](record-receipts.md).

3. **Bank statement.** The agent reads the statement, looks for the receipts
   and proposes an entry for every transaction. A receipt's payment row clears
   the receipt's payable. You approve the list:
   [Import the bank statement](import-bank-statement.md).

4. **Other entries**, such as sales invoices, payroll vouchers and
   corrections:
   [Record a sales invoice, payroll or another manual entry](record-other-entries.md).

5. **Open items.** The agent tells you which transactions were left open and
   what is missing for them, for example a receipt it could not find. Get
   what is missing, or decide that it will be recorded later. The agent keeps
   the home's `TODO.md` up to date: it removes the items settled during the
   month and adds what is left waiting, so the next month starts from there
   ([The company home](../concepts/company-home.md#the-homes-todomd)).

6. **Check.** Check in the monthly report that the bank account balance
   matches the statement's closing balance and that trade payables are only
   the invoices not yet paid:
   [See how the month went](monthly-report.md).

7. **Close**, when the month is finished: [Close the month](close-month.md).

8. **VAT return** from the closed month's figures:
   [in OmaVero](file-vat-omavero.md) or
   [directly to the Tax Administration](file-vat-direct.md). Pay the tax
   yourself.

9. **Employer's separate report** to Tulorekisteri, if the company is in the
   employer register. It is filed every month, also without payments:
   [File the employer's separate report](file-employer-report.md).

## Done when

- The month's state is `sealed` and `vat_filed true`. Ask the agent to show
  the month's state.
- VAT has been paid with the OmaVero reference, if there was any to pay.

## See also

- [Periods, closing and the fiscal year](../concepts/periods.md)
- [Sealing and approval](../concepts/sealing-and-approval.md)
- [Practise your first month](../start/practice-month.md)
