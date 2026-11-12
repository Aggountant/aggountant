# aggountant — user guide

You keep your company's books with an AI agent. The agent prepares vouchers
from bank statements and receipts, you approve them, and aggountant records the
books permanently and files the returns. This guide tells you what you do, what
the agent does, and what you approve before anything is stored or sent to an
authority.

aggountant is at version 0.1 and already keeps the real books of a few
companies. Not everything is ready yet:
[What is ready and what is not](reference/status.md).

The requests to the agent in this guide are examples, not magic words. You can
say the same thing in your own words. The examples are about the fictional
company Esimerkki Oy, and the figures are made up. aggountant is built for
Finnish bookkeeping, so Finnish names of taxes, authorities and services, such
as ALV (Finnish VAT), OmaVero and Tulorekisteri, stay in Finnish and are
explained where they first appear and in the [glossary](reference/glossary.md).
The Finnish original of this guide is in [`../fi/`](../fi/README.md).

## Start here

1. [Is aggountant for you?](start/is-it-for-you.md)
2. [Install aggountant and its skills](start/install.md)
3. [Practise your first month](start/practice-month.md)
4. [Set up your company's books](start/set-up-company.md)

Want to join early or help build it? [Join as an early user](start/early-users.md).

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

## Once or occasionally

- [Set up direct VAT filing](tasks/set-up-vero-api.md), including certificate
  renewal
- [Report travel allowances to Tulorekisteri](tasks/report-travel-allowances.md)
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
