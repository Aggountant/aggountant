# aggountant — user guide

This guide explains how to keep your company's books with aggountant and an AI
agent. It covers what you do yourself, what the agent does for you, and what
you approve before anything is stored permanently or sent to an authority.

The guide is written for a company owner or bookkeeper, not a programmer. It
includes commands because the agent runs them, and you should be able to
recognise what it is doing. Every example uses the fictional company Esimerkki
Oy, and all figures are made up.

aggountant is built for Finnish bookkeeping. Finnish terms that have no exact
English equivalent, such as ALV (Finnish VAT), OmaVero (the Tax
Administration's online service) and Tulorekisteri (the Incomes Register), are
kept in Finnish and explained where they first appear and in the
[glossary](09-glossary.md). The Finnish original of this guide is in
[`../fi/`](../fi/README.md).

## Contents

1. [What aggountant is and who it is for](01-what-aggountant-is.md)
2. [Installation and setting up a company home](02-installation-and-company-home.md)
3. [The agent and its skills](03-agent-and-skills.md)
4. [The monthly cycle: bank, receipts and period close](04-monthly-cycle.md)
5. [VAT and Vero.fi](05-vat-and-vero-fi.md)
6. [Payroll and Tulorekisteri](06-payroll-and-tulorekisteri.md)
7. [Financial statements, reports and fiscal-year status](07-financial-statements-and-reports.md)
8. [Errors and recovery](08-errors-and-recovery.md)
9. [Glossary](09-glossary.md)

## What is ready and what is not

aggountant is at an early stage (version 0.1). These parts are already used
for real bookkeeping:

- recording vouchers and archiving their attachments
- importing bank statements (camt.053 and camt.052)
- booking ordinary purchase receipts, including their VAT treatment
- closing a month
- monthly reports
- filing the VAT return directly with the Finnish Tax Administration, or by
  hand in OmaVero
- reporting tax-free travel allowances to Tulorekisteri.

Payroll, automatic receipt search and the year-end close are in the program,
but they cannot yet be used for real bookkeeping. Their chapters explain why
and what to do instead.
