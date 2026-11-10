# 1. What aggountant is and who it is for

aggountant is a tool for Finnish bookkeeping that you use together with an AI
agent. The agent (Claude Code, for example) reads receipts, bank statements and
other material, proposes the entries and runs aggountant's commands.
aggountant calculates, checks and stores. It makes sure the entries balance,
that VAT is computed from rules rather than guessed, and that a stored voucher
cannot be changed afterwards without anyone noticing.

The work is divided like this:

- **You** supply the material or say where to find it, answer the questions
  only the owner can answer, and approve every permanent change before it is
  made.
- **The agent** does the routine work: it reads the sources, proposes accounts
  and VAT treatment, gathers its questions together and shows you a complete
  set to approve.
- **aggountant** does what has to be exactly right: it numbers vouchers without
  gaps, checks that debits and credits balance, computes VAT from the rate
  table, closes periods, and sends returns only after you have approved them.

## Who it is for

aggountant is made for a small company whose books are kept by one trusted
person, usually the owner, with an agent's help. The program assumes that:

- **there is one user.** The same books are not meant to be used by several
  people or from several machines at the same time.
- **each company has its own company home.** A company home is a directory
  holding the company's chart of accounts, vouchers, attachments and period
  status. If you keep the books of two companies, they have two separate homes.
- **the company files its own returns.** The connections to Vero.fi (the
  Finnish Tax Administration, Verohallinto) and to Tulorekisteri (the Finnish
  Incomes Register) are the company's own. Accounting-firm use, where you file
  on behalf of another company, is not supported.

## What aggountant does not do

- **It does not pay anything.** You pay VAT, wages and invoices yourself in
  your bank or in OmaVero, the Tax Administration's online service. aggountant
  tells you the amount due.
- **It does not decide for you.** The program never seals a voucher or sends a
  return without your explicit approval.
- **It is not a web application.** Everything happens on the command line
  through the agent. Reports come as HTML files that you open in a browser.

## Why a voucher cannot be edited

When a voucher is stored, it is sealed: it gets a permanent number, and a
fingerprint is computed from it and chained to the vouchers before it. If
someone later changes an old voucher, the chain check
(`aggountant voucher verify`) reveals it. A mistake is corrected with a new
correcting voucher, just as the Finnish Accounting Act requires, and the
original is never deleted.

That is why the agent always asks for your approval before sealing. Approval is
the moment you decide what stays in the books permanently.
