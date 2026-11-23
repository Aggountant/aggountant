# You, the agent and aggountant

Three parties keep the books: you, the agent and the aggountant program.

## Three parties

You bring the material or say where it is. You answer the questions only the
company's owner can answer, and you approve every permanent change before it
is made.

The agent (for example Claude Code) does the routine work. It reads the
receipts, bank statements and other material, proposes accounts and VAT
treatments, and shows you a complete set to approve. It runs aggountant's
commands for you.

aggountant does what has to be exactly right. It numbers the vouchers without
gaps, checks that debits and credits balance, calculates VAT from its VAT rate
table, closes the periods and sends returns only after approval.

There is no AI in aggountant; the AI is in the agent you use. The agent, in
turn, does not write the books itself. It asks aggountant to record them, and
aggountant refuses anything that does not add up.

## Why the work is divided this way

The agent can read mixed material and work out what a transaction is, but it
can make mistakes. aggountant works nothing out, but it calculates and checks
the same way every time. So the agent proposes and aggountant checks. What is
left to you is what neither can do: you know whether an expense belongs to the
company, and you decide what stays in the books.

## The agent works like a colleague who knows bookkeeping

The agent takes its instructions from the skills that come with aggountant,
which share a role description. It tells the agent to settle on
its own what the sources and the company's earlier books are enough for, and
to ask you only what it cannot work out.

Without asking, the agent runs anything that only reads or previews and leaves
no permanent trace. For example, it

- reads the chart of accounts, earlier vouchers and the company's saved
  knowledge
- previews and validates a voucher
- previews a closed month's VAT return figures
- checks direct filing readiness without a network connection
- makes reports.

Permanent steps, such as sealing a voucher, closing a month and sending a
return, wait for your approval:
[Sealing and approval](sealing-and-approval.md).

## The agent gathers its questions

A month's material can have dozens of transactions, so the agent does not
interrupt you at every receipt. It asks when the difference between the
options matters to you (the account, the VAT treatment, whether the expense
belongs to the company) and it cannot work out which you would choose. It asks
its questions in one go. It can save your answers in
[the company knowledge store](company-home.md#the-company-knowledge-store) so
that the same thing is not asked again.

## What this means for you

- Tell the agent the goal, not commands. "Let's do March's books" is enough.
- Answer questions precisely, because your answer also guides future entries.
- Look at every approval request. The approval is where the agent's mistake is
  stopped.

## See also

- [Sealing and approval](sealing-and-approval.md)
- [Install aggountant and its skills](../start/install.md)
- [The company home: what is stored where](company-home.md)
