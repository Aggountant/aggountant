# aggountant user guide

You keep your company's books with an AI agent. The agent prepares vouchers
from bank statements and receipts, you approve them, and aggountant records the
books permanently and files the returns. The guide shows what you do yourself,
what the agent does, and what you approve before anything is stored or sent to
an authority.

aggountant is an early 0.x version that already keeps the real books of a few
companies. [What is ready and what is not](reference/status.md) lists what is
still missing.

The requests to the agent in this guide are examples, and you can say the same
thing in your own words. The example company, Esimerkki Oy, is fictional and
its figures are made up. aggountant is built for Finnish bookkeeping, so the
Finnish names of taxes, authorities and services, such as ALV (Finnish VAT),
OmaVero and Tulorekisteri, stay in Finnish. Each is explained where it first
appears and in the [glossary](reference/glossary.md). The guide is also
available [in Finnish](../fi/README.md), the language it is written in first.

## Start here

1. [Is aggountant for you?](start/is-it-for-you.md)
2. [Install aggountant and its skills](start/install.md)
3. [Practise your first month](start/practice-month.md)
4. [Set up your company's books](start/set-up-company.md)

If you are coming from another bookkeeping program, collect the old material
first: [Move your books from another program](start/move-from-another-program.md).
If you want to join early or help build it, read
[Join as an early user](start/early-users.md).

## Every month

- [Do a month's bookkeeping from start to finish](tasks/monthly-routine.md):
  links to every step
- [Import the bank statement](tasks/import-bank-statement.md)
- [Record receipts and purchase invoices](tasks/record-receipts.md)
- [Record a sales invoice, payroll or another manual entry](tasks/record-other-entries.md)
- [See how the month went](tasks/monthly-report.md)
- [Close the month](tasks/close-month.md)
- [File the VAT return in OmaVero](tasks/file-vat-omavero.md) or
  [send it directly to the Tax Administration](tasks/file-vat-direct.md)
- [File the employer's separate report](tasks/file-employer-report.md), if the
  company is in the employer register

## Once or occasionally

- [Set up direct VAT filing](tasks/set-up-vero-api.md), including certificate
  renewal
- [Compute the tax-free allowances of a trip](tasks/compute-travel-allowances.md)
- [Report travel allowances to Tulorekisteri](tasks/report-travel-allowances.md)
- [Record a travel advance and its settlement](tasks/record-travel-advance.md)
- [Correct a wrong voucher](tasks/correct-voucher.md)
- [Back up and restore your books](tasks/back-up-and-restore.md)
- [Upgrade aggountant](tasks/upgrade.md)

## At year end

- [Prepare the financial statements](tasks/year-end.md)

## When something goes wrong

- [When something goes wrong](troubleshooting.md): by symptom
- [Error codes](reference/error-codes.md): by the code on screen

## How aggountant works

- [You, the agent and aggountant](concepts/division-of-work.md)
- [Sealing and approval](concepts/sealing-and-approval.md)
- [Periods, closing and the fiscal year](concepts/periods.md)
- [How the VAT treatment is decided](concepts/vat-treatment.md)
- [Filings and uncertain outcomes](concepts/filings.md)
- [The company home: what is stored where](concepts/company-home.md)
- [For bookkeepers](concepts/for-bookkeepers.md)

## Reference

- [What is ready and what is not](reference/status.md)
- [Error codes](reference/error-codes.md)
- [Vero.fi readiness blockers](reference/vero-fi-blockers.md)
- [Commands at a glance](reference/commands.md)
- [Glossary](reference/glossary.md)
