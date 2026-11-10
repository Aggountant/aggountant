# 9. Glossary

This glossary explains the bookkeeping and aggountant terms you meet while using
the program. Terms are in alphabetical order. The Finnish term is given in
parentheses, because the agent's skills, the chart of accounts and the
authorities' services use it. Where the program prints a name on screen, that
name is shown as code.

**Acceptance tests (hyväksyntätestit).** Tests in the Tax Administration's API
portal that software must pass before production use. aggountant runs them with
`vero-fi verify-portal`.

**Account (tili).** An entry in the chart of accounts that a line is booked to,
for example 1910 Pankkitili (bank account) or 7690 Atk-laite- ja
ohjelmistokulut (IT hardware and software expenses).

**Accrual (jaksotus).** Booking income or an expense into the period it belongs
to, even if the money moves in another period.

**Acknowledgement (kuittaus).** The Tax Administration's response saying it has
received a return. An acknowledgement does not yet mean the return shows in
OmaVero.

**Agent (agentti).** An AI assistant (Claude Code, for example) that reads the
material, proposes entries and runs aggountant's commands for you. The agent
seals and sends nothing without your approval.

**ALV (arvonlisävero).** Finnish value added tax. A company charges it on its
sales and may deduct it on its purchases. The difference is paid to the Tax
Administration or received as a refund.

**ALV period (ALV-kausi).** The span of time a VAT return covers. In aggountant
it is the same as the bookkeeping close period: a month or a quarter.

**ALV rate (ALV-kanta).** The VAT percentage. The rate depends on the product
and the date of the document: the general rate is 25.5 % from 1 September 2024.
aggountant picks the rate by the date of the receipt or invoice, not the payment
date.

**ALV return (ALV-ilmoitus).** A period's VAT figures reported to the Tax
Administration in numbered fields. In aggountant the return is made from a
closed period's figures, either directly through the interface or by hand in
OmaVero.

**Approval (hyväksyntä).** Your explicit decision before a permanent act or an
act with legal effect. An approval is tied to what you were shown. If the
content changes, the approval is no longer valid.

**Attachment (liite).** Original material archived with a voucher, such as a
receipt, an invoice or a bank statement. aggountant recognises the same file if
someone tries to book it a second time.

**Attestation (vakuutus).** Your confirmation of something the program cannot
check itself, for example that the software is registered or the production
permission granted. The program relies on your attestation.

**Balance sheet (tase).** A statement of the company's assets, liabilities and
equity on a given date.

**Balance sheet specification (tase-erittely).** A breakdown of what the balance
sheet accounts contain at the year-end close.

**Business ID (Y-tunnus).** The company's identifier in the Trade Register and
in taxation.

**camt.053 and camt.052.** File formats for banks' electronic statements.
aggountant imports their transactions as vouchers.

**Certificate (varmenne).** An electronic certificate issued by the Tax
Administration that identifies your company to the interface. It is valid for
two years. Its private key is secret.

**Chart of accounts (tilikartta).** The company's list of bookkeeping accounts.
`init` creates a default chart, which you can see with `aggountant chart list`.

**Closed (`sealed`, suljettu).** The final state of a period or a fiscal year.
Nothing can be booked into a closed period, and corrections go into the next
open period.

**Closing (`closing`, suljettavana).** A period's state between the start of
the close and its approval. Nothing can be booked into the period in this
state.

**Company home (yrityskoti).** The directory holding one company's entire
bookkeeping: chart of accounts, vouchers, attachments, period status and
settings.

**Company knowledge store (yrityksen tietovarasto).** The home's
`knowledge/company/` directory, where the agent stores the company's facts and
practices with their sources and history.

**Correcting voucher (korjaustosite).** A new voucher that corrects a mistake in
an earlier one. The original is neither changed nor deleted.

**Debit and credit (debet ja kredit).** The two sides of double-entry
bookkeeping. In every voucher, the debit and credit lines add up to the same
amount.

**Depreciation (poisto).** Spreading the cost of a long-lived purchase, such as
a machine or a device, as an expense over several years.

**Entry line (vienti).** One line of a voucher: account, amount and side (debit
or credit).

**Fingerprint (sormenjälki).** An identifier unique to a certificate (SHA-256).
The fingerprint is not secret, and you can use it to tell the Tax
Administration's support, for example, which certificate you mean.

**Fiscal year (tilikausi).** The year the financial statements are made for. In
aggountant the fiscal year is the calendar year.

**Ilmoitin.fi.** The Tax Administration's service for submitting filing files.
aggountant does not produce Ilmoitin.fi files.

**Income statement (tuloslaskelma).** A statement of a period's revenue,
expenses and result.

**Income type (tulolaji).** A Tulorekisteri code that tells what kind of payment
is being reported, for example 311 mileage allowance or 331 per diem.

**Mileage allowance (kilometrikorvaus).** A tax-free allowance for using your
own car on work trips. Tulorekisteri income type 311.

**Notes (liitetiedot).** The additional information that accompanies the
statements in the financial statements.

**Obligation (velvoite).** An open matter with a due date, such as a missing
mileage log. An open obligation blocks the close of its period.

**OmaVero.** The Tax Administration's online service (MyTax). That is where you
see your returns and pay the tax.

**Open (`open`, avoin).** A period's state in which it can still be booked into.

**Opening balances (avaavat saldot).** The account balances on the day the
books move to aggountant from another system. They are booked as one voucher.

**Part-ownership (pääomaosakkuus).** Whether the income earner owns part of the
company. A Tulorekisteri report always states it explicitly, and aggountant
does not infer it.

**Per diem (päiväraha).** A tax-free allowance for living costs caused by a work
trip. Tulorekisteri income type 331.

**Period (kausi).** A bookkeeping close period: a month (`2026-03`) or a quarter
(`2026-Q1`).

**Period close (kauden sulku).** Locking a period's books. After the close
nothing can be booked into the period, and the VAT return is made from its
figures.

**Preview (esikatselu).** Running a command so that it shows the outcome but
stores nothing. You approve the preview before the actual storing.

**Production permission (tuotantolupa).** The Tax Administration's permission,
granted after the acceptance tests, to use the interface for real returns.

**Readiness (valmius).** The state of the Vero.fi connection, computed by
`aggountant vero-fi status`. Direct filing is possible only when readiness is
in order (`ready`).

**Replacement return (korvaava ilmoitus).** A new return that replaces one
already filed. aggountant does not make replacement returns. They are made in
OmaVero or in Tulorekisteri's e-service.

**Reverse charge (käännetty verovelvollisuus).** A VAT procedure in which the
buyer, not the seller, accounts for the tax, for example when buying services
from another EU country.

**Run (`run`, ajo).** A multi-step task that stores its progress and can be
continued after an interruption, such as a period close or a VAT return. A run
has an identifier, such as `cp-…` (close), `fv-…` (VAT return) or `wr-…`
(Tulorekisteri report).

**Sealing (sinetöinti).** Storing a voucher permanently. A sealed voucher gets a
number and is added to the voucher chain, and it can no longer be changed.

**Skill (skilli).** An instruction for the agent that ships with aggountant, for
example `capture-receipt` for handling receipts.

**Software key (ohjelmistoavain).** A key issued by the Tax Administration's API
portal that identifies registered software. Every aggountant user registers
their own software and gets their own key. The key is secret.

**Tax Administration (Verohallinto).** The Finnish tax authority. Its online
services are at Vero.fi.

**Tilinpäätös.** The Finnish statutory year-end close: the income statement,
balance sheet and notes drawn up at the end of the fiscal year.

**Tulorekisteri.** The Finnish Incomes Register, the national register to which
wages, fees and allowances are reported.

**Uncertain outcome (`outcome unknown`, epävarma lopputulos).** A situation in
which the connection dropped or the service answered with an error while a
return was being sent, so it is not known whether the return arrived. The
return is not sent again; instead the status is asked from the authority.

**Vero API.** The Tax Administration's interface through which aggountant sends
the ALV return directly.

**Voucher (tosite).** The basic unit of bookkeeping: one transaction with its
entry lines, date and description, plus an attachment where needed. In
aggountant vouchers are numbered without gaps.

**Voucher chain (tositeketju).** The sealed vouchers chained so that each one
contains an identifier of the one before it. If any voucher is changed, the
chain check (`voucher verify`) notices.
