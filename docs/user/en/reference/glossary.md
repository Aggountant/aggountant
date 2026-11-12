# Glossary

This glossary explains the bookkeeping and aggountant terms you meet while
using the program. Terms are in alphabetical order. The Finnish term is in
parentheses, because the skills, the chart of accounts and the authorities'
services use it. Where the program prints its own English name on screen,
that name is shown as code.

**Acceptance tests (hyväksyntätestit).** Tests in the Tax Administration's API
portal that software must pass before production use. aggountant runs them
with `vero-fi verify-portal`.

**Account (tili).** An item in the chart of accounts that a posting is
recorded to, for example 1910 Pankkitili (bank account) or 7690 Atk-laite- ja
ohjelmistokulut (IT hardware and software).

**Accrual (jaksotus).** Recording income or an expense in the period it
belongs to, even if the money moves in another period.

**Acknowledgement (kuittaus).** The Tax Administration's response saying it
has received a return. An acknowledgement does not yet mean the return shows
in OmaVero.

**Agent (agentti).** An AI assistant (for example Claude Code) that reads the
material, proposes entries and runs aggountant's commands for you. The agent
seals and sends nothing without your approval.

**ALV (arvonlisävero).** Finnish value added tax, VAT. A company charges it on
its sales and may deduct it on its purchases. The difference is paid to the
Tax Administration or received as a refund. This guide says VAT.

**Approval (hyväksyntä).** Your explicit decision before a permanent step or
one with legal effect. An approval is bound to what you were shown. If the
content changes, the approval is no longer valid.

**Attachment (liite).** Original material archived with a voucher, such as a
receipt, an invoice or a bank statement. aggountant recognises the same file
if someone tries to record it a second time.

**Attestation (vakuutus).** Your confirmation of something the program cannot
check itself, for example that the software has been registered or the
production permission granted. The program relies on your attestation.

**Balance sheet (tase).** A statement of the company's assets, liabilities and
equity on a given date.

**Balance sheet breakdown (tase-erittely).** A breakdown of what the balance
sheet accounts contain at the time of the financial statements.

**Business ID (Y-tunnus).** The company's identifier in the trade register and
in taxation.

**camt.053 and camt.052.** File formats for banks' electronic statements.
aggountant imports their transactions as vouchers.

**Certificate (varmenne).** An electronic certificate issued by the Tax
Administration with which your company identifies itself to the interface.
The certificate is valid for two years. Its private key is secret.

**Chart of accounts (tilikartta).** The list of accounts of a company's books.
`init` creates a default chart of accounts, which you can see with
`aggountant chart list`.

**Closed (`sealed`, suljettu).** The final state of a period or fiscal year.
Nothing can be recorded in a closed period, and corrections go into the next
open period.

**Closing (`closing`, suljettavana).** A period's state between the start of
the close and its approval. Nothing can be recorded in the period in this
state.

**Company home (yrityskoti).** The directory that holds one company's entire
books: chart of accounts, vouchers, attachments, period state and settings.

**Company knowledge store (yrityksen tietovarasto).** The home's
`knowledge/company/` directory, where the agent saves the company's facts and
practices with their sources and history.

**Completeness (kattavuus).** The monthly report's note on whether all the
material has been recorded. It always says "varmistamaton" (unverified),
because the program cannot know. You and the agent judge it against the
sources.

**Correcting voucher (korjaustosite).** A new voucher that corrects a mistake
in an earlier one. The original is not changed or deleted.

**Debit and credit (debet ja kredit).** The two sides of double-entry
bookkeeping. In every voucher, the debit and credit postings add up to the
same amount.

**Depreciation (poisto).** Spreading the cost of a long-lived purchase, such
as a machine or a device, as an expense over several years.

**Financial statements (tilinpäätös).** The income statement, balance sheet
and notes prepared at the end of the fiscal year.

**Fingerprint (sormenjälki).** The identifier of a certificate (SHA-256). The
fingerprint is not secret, and you can use it to tell, for example, the Tax
Administration's support which certificate you mean.

**Fiscal year (tilikausi).** The year for which financial statements are
prepared. In aggountant the fiscal year is the calendar year.

**Ilmoitin.fi.** The Tax Administration's service for sending return files.
aggountant does not prepare Ilmoitin.fi files.

**Income statement (tuloslaskelma).** A statement of a period's income,
expenses and result.

**Income type (tulolaji).** The Tulorekisteri code for the kind of payment,
for example 311 mileage allowance or 331 per diem.

**Manual entry (erikoiskirjaus).** A voucher the agent puts together posting
by posting, because neither the bank statement import nor the receipt route
fits it, for example a sales invoice, a payroll voucher, opening balances or a
correction.

**Mileage allowance (kilometrikorvaus).** A tax-free allowance for using your
own car for work journeys. Tulorekisteri income type 311.

**Notes (liitetiedot).** The additional information that accompanies the
statements in the financial statements.

**Obligation (velvoite).** An open matter with a deadline, for example a
missing mileage log. An open obligation blocks the close of its period.

**OmaVero.** The Tax Administration's online service. There you see your
returns and pay tax.

**Open (`open`, avoin).** A period's state in which entries can still be
recorded in it.

**Opening balances (avaavat saldot).** The account balances on the day the
books move to aggountant from another system. They are recorded as one
voucher.

**Outcome unknown (epävarma lopputulos).** A situation in which the
connection dropped or the service returned an error while a return was being
sent, and it is not known whether the return got through. The return is not
sent again; the status is asked from the authority.

**Per diem (päiväraha).** A tax-free allowance for living costs caused by a
work trip. Tulorekisteri income type 331.

**Period (kausi).** The bookkeeping close period: a month (`2026-03`) or a
quarter (`2026-Q1`).

**Period close (kauden sulku).** Locking a period's books. After the close
nothing can be recorded in the period, and the VAT return is made from its
figures.

**Posting (vienti).** One line of a voucher: account, amount and side (debit
or credit).

**Practice home (harjoituskoti).** A separate company home for practising
with the guide's practice material. It has nothing to do with real books and
can be deleted.

**Preview (esikatselu).** Running a command so that it shows the result but
stores nothing. You approve the preview before the actual recording.

**Production permission (tuotantolupa).** The Tax Administration's permission
to use the interface for real returns after the acceptance tests.

**Readiness (valmius).** The state of the Vero.fi connection, which
`aggountant vero-fi status` calculates. A direct return is possible only when
readiness is in order (`ready`).

**Replacement return (korvaava ilmoitus).** A new return that replaces one
already filed. aggountant does not make replacement returns. They are made in
OmaVero or in Tulorekisteri's e-service.

**Reverse charge (käännetty verovelvollisuus).** A VAT procedure in which the
buyer, not the seller, accounts for the tax, for example when buying services
from another EU country.

**Run (`run`, ajo).** A multi-step task that saves its progress and can be
continued after an interruption, for example a period close or a VAT return.
A run has an identifier such as `cp-…` (close), `fv-…` (VAT return), `wr-…`
(Tulorekisteri report) or `cy-…` (year-end).

**Sealing (sinetöinti).** Recording a voucher permanently. A sealed voucher
gets a number and is added to the voucher chain, and it can no longer be
changed.

**Shareholding (pääomaosakkuus).** Whether the income earner owns part of the
company. In a Tulorekisteri report it is always stated explicitly, and
aggountant does not infer it.

**Skill (skilli).** Instructions for the agent that come with aggountant, for
example `capture-receipt` for handling receipts.

**Software key (ohjelmistoavain).** A key from the Tax Administration's API
portal that identifies registered software. Every aggountant user registers
their own software and gets their own key. The key is secret.

**Tulorekisteri.** The Finnish Incomes Register, a national register to which
wages, fees and allowances are reported.

**Unconfirmed transaction (vahvistamaton tapahtuma).** A statement row the
bank has not booked yet, for example a card reservation. aggountant does not
import it; it is recorded from the statement on which it has gone through.

**VAT period (ALV-kausi).** The span of time a VAT return covers. In
aggountant it is the same as the bookkeeping close period: a month or a
quarter.

**VAT rate (ALV-kanta).** The VAT percentage. The rate depends on the product
and the document's date: the general rate is 25.5 % from 1 September 2024.
aggountant picks the rate by the date of the receipt or invoice, not the
payment date.

**VAT return (ALV-ilmoitus).** A period's VAT figures reported to the Tax
Administration in numbered fields. In aggountant the return is made from a
closed period's figures, either directly through the interface or by hand in
OmaVero.

**Vero API.** The Tax Administration's interface through which aggountant
sends the VAT return directly.

**Voucher (tosite).** The basic unit of bookkeeping: one transaction with its
postings, date and description, and an attachment where needed. In aggountant
vouchers are numbered without gaps.

**Voucher chain (tositeketju).** The sealed vouchers chained so that each one
contains a hash of the previous one. If a voucher is changed, the chain check
(`voucher verify`) notices.

## See also

- [Commands at a glance](commands.md)
- [Error codes](error-codes.md)
