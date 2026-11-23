# Do a month's bookkeeping from start to finish

A month's bookkeeping goes in the same order every month, and each step has
a page of its own. If the material is all there, an ordinary month is
recorded, closed and filed in one session. If the company closes its books
[by quarter](../concepts/periods.md#the-fiscal-year-and-periods), the close
and the VAT return (steps 7 and 8) come after the last month of the quarter.

## Before you start

- Your company's books are [set up](../start/set-up-company.md).
- The month's bank statement is available as a camt.053 or camt.052 file, and
  the receipts are where the home's `AGENTS.md` says.

## How to do it

1. **Ask the agent to handle the whole month.**

   > Let's do March's books.

   The agent uses the `capture-receipt` skill. It does on its own whatever the
   sources are enough for, and brings you the questions and the vouchers to
   approve together instead of one receipt at a time.

2. **The agent records the receipts and purchase invoices**, each as
   [a voucher of its own](record-receipts.md). A purchase paid from the
   company's account is recorded as a trade payable, and one paid with your
   own money as a debt to you.

3. **The agent reads the bank statement**, looks for the receipts and
   [proposes an entry for every transaction](import-bank-statement.md). A
   receipt's payment row clears the receipt's payable. You approve the list.

4. **The agent records the other vouchers**, such as
   [sales invoices, payroll vouchers and corrections](record-other-entries.md).

5. **Go through the open items.** The agent tells you which transactions were
   left open and what is missing for them, for example a receipt it could not
   find. Get what is missing, or decide that it will be recorded later. The
   agent removes the items settled during the month from
   [the home's `TODO.md`](../concepts/company-home.md#the-homes-todomd) and
   adds what is left waiting, so the next month starts from there.

6. **Check the month.** Look at [the monthly report](monthly-report.md): the
   bank account balance should match the statement's closing balance, and
   trade payables should hold only invoices not yet paid.

7. **Ask the agent to [close the month](close-month.md)** when it is
   finished.

8. **File the VAT return** from the closed month's figures, either
   [in OmaVero](file-vat-omavero.md) or
   [directly to the Tax Administration](file-vat-direct.md). Pay the tax
   yourself.

9. **File [the employer's separate report](file-employer-report.md) to
   Tulorekisteri** if the company is in the employer register. It is filed
   every month, also without payments.

## Done when

- The month's state is `sealed` and `vat_filed true`. Ask the agent to show
  the month's state.
- VAT has been paid with the OmaVero reference, if there was any to pay.

## See also

- [Periods, closing and the fiscal year](../concepts/periods.md)
- [Sealing and approval](../concepts/sealing-and-approval.md)
- [Practise your first month](../start/practice-month.md)
