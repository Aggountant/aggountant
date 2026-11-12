# Is aggountant for you?

This page helps you decide whether aggountant is worth trying: what it does,
what it asks of you, what it does not do and who is responsible for the books.
It takes five minutes to read.

## What aggountant does

aggountant is a bookkeeping tool for Finnish limited companies (osakeyhtiö),
used together with an AI agent. A month goes like this:

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

The division of work is described in more detail in
[You, the agent and aggountant](../concepts/division-of-work.md).

aggountant is at version 0.1 and already keeps the books of a few companies
that have no accounting firm. Features are added as these companies meet new
situations. That is why ordinary monthly work is in good shape, while rarer
cases are filled in step by step. Commands and file formats may still change
between versions.

## Who it suits

aggountant is made for a small company whose books are kept by one trusted
person, usually the owner, with the help of an agent. It suits you if:

- your company is a small Finnish limited company whose fiscal year is the
  calendar year
- your bank provides the statement as an electronic camt.053 or camt.052 file.
  Holvi's statement has been tried in practice; other banks' files have not yet
- you can start an agent that runs in a terminal, such as Claude Code, but
  would rather not learn a bookkeeping program's commands
- you know roughly what a voucher, VAT and a balance sheet are.

The program assumes three things:

- **There is one user.** The same books are not used from several people or
  machines at the same time.
- **Every company has its own company home**, a directory that holds all of
  its books. Two companies have two homes.
- **The company files its own returns.** The Vero.fi and Tulorekisteri
  connections belong to the company itself. Filing on behalf of another
  company, as an accounting firm does, is not supported.

## What it asks of you

- **An agent and a computer.** aggountant is a command-line program without
  AI of its own. The AI is in the agent, which you obtain and start yourself.
- **Installing from source.** There are no ready-made installation packages
  yet: [Install aggountant and its skills](install.md).
- **Your approval of every permanent step.** The agent does the routine work,
  but you look at and approve the vouchers, the month's close and the returns.
- **Your own Vero API registration**, if you want to send the VAT return
  directly. The registration, the API agreement and the certificate are your
  company's own, and setting them up takes several steps:
  [Set up direct VAT filing](../tasks/set-up-vero-api.md). Without it you
  enter the VAT figures yourself in OmaVero.

## What it does not do

- **It does not pay anything.** You pay VAT, wages and invoices yourself in
  your bank or in OmaVero. aggountant tells you the amount to pay.
- **It does not decide for you.** Nothing is stored or sent without your
  explicit approval.
- **It is not a web application.** Everything happens on the command line
  through the agent. You get reports as HTML files that you open in a browser.
- **It does not handle everything yet.** Cash wages cannot yet be paid or
  reported, automatic receipt search is not in use, and the financial
  statements (tilinpäätös) are prepared outside aggountant for now. More
  detail: [What is ready and what is not](../reference/status.md).

## Who is responsible for the books

You or your company's board, as always. aggountant is a tool: it calculates,
checks and refuses to record anything that does not add up, but it does not
know whether an expense belongs to the company or whether all the material has
been recorded. The agent can make mistakes, which is why every permanent step
waits for your approval. This guide is not legal or tax advice.

## Try it without risk

[Practise your first month](practice-month.md) records a fictional company's
March in a separate practice home in about half an hour. You see what working
with the agent feels like before you start using aggountant for your own
company.

## See also

- [Install aggountant and its skills](install.md)
- [For bookkeepers](../concepts/for-bookkeepers.md)
- [What is ready and what is not](../reference/status.md)
