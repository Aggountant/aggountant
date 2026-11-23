# Is aggountant for you?

aggountant is a bookkeeping tool for Finnish limited companies (osakeyhtiö),
used together with an AI agent.

## What aggountant does

A month goes like this:

1. The agent reads the month's bank statement and receipts and proposes an
   account and a VAT treatment for every transaction.
2. It asks you only what it cannot work out, for example whether a restaurant
   bill was a client meeting.
3. You look at the proposals and approve them. aggountant records the vouchers
   permanently, closes the month with your approval and calculates the VAT
   return.
4. The VAT return goes directly to the Finnish Tax Administration
   (Verohallinto), or you enter the figures yourself in OmaVero, the Tax
   Administration's online service.

[You, the agent and aggountant](../concepts/division-of-work.md) describes
the division of work in more detail.

aggountant is an early 0.x version and already keeps the books of a few companies
that have no accounting firm. Features are added as these companies meet new
situations, so ordinary monthly work is in good shape and rarer cases are
filled in step by step. Commands and file formats may still change between
versions. If you want to join early, we can also meet:
[Join as an early user](early-users.md).

## Who it suits

aggountant is made for a small company whose books are kept by one person,
usually the owner, with the help of an agent. It suits you if:

- your company is a small Finnish limited company
- your bank gives you the statement as a camt.053 or camt.052 file (Holvi's
  statement has been tried in practice; other banks' files have not yet)
- you can start an agent that runs in a terminal, such as Claude Code, but
  would rather not learn a bookkeeping program's commands
- you know roughly what a voucher, VAT and a balance sheet are.

The same books are not used by several people or from several machines at the
same time. Every company has its own company home, a directory that holds all
of its books, so two companies have two homes. The Vero.fi and Tulorekisteri
connections belong to the company itself. Filing on behalf of another company,
as an accounting firm does, is not supported.

## What it asks of you

- You get and start the agent yourself. aggountant is a command-line program
  with no AI of its own.
- You install the program and its skills on your computer:
  [Install aggountant and its skills](install.md).
- You look at and approve the vouchers, the month's close and the returns.
  The agent does the routine work, but nothing is recorded permanently or
  sent without your approval.
- If you want to send the VAT return directly, you need your own Vero API
  registration. The registration, the API agreement and the certificate are
  your company's own, and setting them up takes several steps:
  [Set up direct VAT filing](../tasks/set-up-vero-api.md). Without it you
  enter the VAT figures yourself in OmaVero.

## What it does not do

- It does not pay anything. You pay VAT, wages and invoices yourself in your
  bank or in OmaVero, and aggountant tells you the amount to pay.
- It is not a web application. Everything happens on the command line through
  the agent, and you get reports as HTML files that you open in a browser.
- It does not handle everything yet. Cash wages cannot yet be calculated,
  paid or reported, automatic receipt search is not in use, and the official
  financial statements document (tilinpäätös) and the tax return are prepared
  outside aggountant. All the limitations:
  [What is ready and what is not](../reference/status.md).

## Who is responsible for the books

You or your company's board, as always. aggountant calculates, checks and
refuses to record anything that does not add up, but it does not know whether
an expense belongs to the company or whether all the material has been
recorded. The agent can make mistakes, which is why every permanent step waits
for your approval. This guide is not legal or tax advice.

## Try it without risk

[Practise your first month](practice-month.md) records a fictional company's
March in a separate practice home in about half an hour. You see what working
with the agent feels like before you start using aggountant for your own
company.

## See also

- [Install aggountant and its skills](install.md)
- [For bookkeepers](../concepts/for-bookkeepers.md)
- [What is ready and what is not](../reference/status.md)
