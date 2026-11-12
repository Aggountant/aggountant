# You, the agent and aggountant

When you use aggountant, three parties keep the books: you, the agent and the
aggountant program. This page explains what belongs to whom and why the work
is divided this way.

## Three parties

- **You** bring the material or say where it is, answer the questions only
  the company's owner can answer, and approve every permanent change before
  it is made.
- **The agent** (for example Claude Code) does the routine work: it reads the
  receipts, bank statements and other material, proposes accounts and VAT
  treatments, gathers the questions together and shows you a complete set to
  approve. It runs aggountant's commands for you.
- **aggountant** does what has to be exactly right: it numbers the vouchers
  without gaps, checks that debits and credits balance, calculates VAT from
  its VAT rate table, closes the periods and sends returns only after
  approval.

aggountant contains no AI. The AI is in the agent you use. The agent, in turn,
does not write the books itself but asks aggountant to record them, and
aggountant refuses anything that does not add up.

## Why the work is divided this way

The agent is good at reading mixed material and working out what a
transaction is, but it can make mistakes. aggountant works nothing out, but it
calculates and checks the same way every time. When the agent proposes and
aggountant checks, each does what it is reliable at. You do what neither can:
you know whether an expense belongs to the company, and you decide what stays
in the books.

## The agent works like a colleague who knows bookkeeping

The skills that come with aggountant tell the agent how to work. All the
skills share a role description (`SOUL.md`): the agent works like a colleague
who knows bookkeeping. It settles on its own what the sources and the
company's earlier books are enough for, and asks you only what it cannot work
out.

Without asking, the agent runs anything that only reads or previews and
leaves no permanent trace, for example:

- reading the chart of accounts, earlier vouchers and the company's saved
  knowledge
- previewing and validating a voucher
- previewing a closed month's VAT return figures
- checking direct filing readiness without a network connection
- making reports.

Permanent steps, such as sealing a voucher, closing a month and sending a
return, wait for your approval:
[Sealing and approval](sealing-and-approval.md).

## The agent gathers its questions

A month's material can have dozens of transactions, and the agent should not
interrupt you at every receipt. It asks when the options differ in a way that
matters to you (the account, the VAT treatment, whether the expense belongs to
the company) and it cannot work out your choice. The questions come all
together, and the agent can save your answers in the company knowledge store
so that the same thing is not asked again
([The company home](company-home.md#the-company-knowledge-store)).

## What this means for you

- Tell the agent the goal, not commands: "let's do March's books" is enough.
- Answer questions precisely. Your answer also guides future entries.
- Always look at approval requests. The agent can make mistakes, and the
  approval is where a mistake is stopped.

## See also

- [Sealing and approval](sealing-and-approval.md)
- [Install aggountant and its skills](../start/install.md)
- [The company home: what is stored where](company-home.md)
