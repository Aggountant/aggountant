# 3. The agent and its skills

aggountant contains no AI. The AI is in the agent you use, for example Claude
Code. So that the agent knows how to work as a bookkeeper, aggountant ships
with **skills**: instructions the agent reads before it starts a task. The
skills are written in Finnish, because the bookkeeping they describe is
Finnish. That does not stop you from talking to the agent in English.

## Installing the skills

To see the skills bundled with the program:

```sh
aggountant skill list
```

There are three:

| Skill | What the agent uses it for |
|---|---|
| `aggountant-init` | setting up a company home, or continuing an unfinished setup |
| `capture-receipt` | handling receipts, vouchers and a month's transactions |
| `vero-fi` | Vero.fi onboarding, the VAT return and Tulorekisteri travel allowances |

Install them all at once:

```sh
aggountant skill install
```

By default the skills go into Claude Code's skill directory `~/.claude/skills/`.
For another agent, give its skill directory with the `--target` option.
Reinstall the skills every time you update aggountant, because the
instructions are written for the commands of the same version. The command
`aggountant skill print <name>` shows a skill's content without installing
anything.

All skills share a role description (`SOUL.md`). It tells the agent to act like
a colleague who knows bookkeeping: it settles whatever the sources and the
company's earlier books are enough for, and asks you only what it cannot work
out.

## What the agent does on its own

The agent runs, without asking, anything that only reads or previews and leaves
no permanent trace. For example:

- reading the chart of accounts, earlier vouchers and company facts
- previewing and checking a voucher draft
- previewing the VAT return figures for a sealed period
- checking Vero.fi readiness without a network connection
- producing reports.

The agent also gathers its questions together. A month's material can contain
dozens of transactions, and the agent should not interrupt you at every
receipt. It asks when the options differ in a way that matters to you (the
account, the VAT treatment, whether a cost belongs to the company) and it
cannot work out which you would choose.

## What you approve

Permanent acts, and acts with legal effect, happen only after your explicit
approval:

| Act | Why approval is required |
|---|---|
| sealing a voucher | a sealed voucher is permanent and can only be corrected with a correcting voucher |
| closing a period | nothing more can be booked into a closed period |
| sending the VAT return | the return has legal effect and cannot be withdrawn |
| sending a Tulorekisteri report | same as the VAT return |
| switching Vero.fi to production, and the attestations that go with it | the program relies on your attestations and cannot check them |

Approval is tied to what you were shown. When booking a receipt, the agent
shows you a preview, and the program computes a fingerprint from it. If the
source data, the proposal or the chart of accounts changes before sealing, the
program refuses and the agent has to show you a new preview. So you cannot
approve one thing and get another.

A good approval request shows the essentials of each voucher: date, amount,
counterparty, account and VAT treatment. "Shall I book 14 vouchers?" on its own
is not enough. Ask to see the lines if the agent does not show them.

## The company knowledge store

The agent stores the company facts and practices it learns in the home's
`knowledge/company/` directory: the fiscal year, for example, how a particular
supplier's invoices are booked, or that the phone subscription is used entirely
by the company. Each fact has a source and a history, so later you can see
where it came from and when it changed.

```sh
aggountant knowledge company list
aggountant knowledge company search --query "phone"
aggountant knowledge company history --id "identifier"
```

Outdated facts are not deleted. They are retired
(`knowledge company retire`), which keeps the earlier fact and the reason for
the change.

## The home's AGENTS.md: where material comes from

aggountant does not dictate where receipts come from or where processed
material goes. Write that down with the agent, in your own words, in a file
called `AGENTS.md` at the root of the home. It is worth covering:

- where receipts and invoices arrive (email, a cloud folder, a local folder,
  the person responsible)
- where drafts of expense claims and travel claims are kept
- where the mileage log is kept
- where processed original material is archived, and when it may be moved out
  of its source
- which folders in the home belong in git and which do not.

Without this file the agent has to ask every time, or guess. `aggountant-init`
helps write the file, and `capture-receipt` reads it before it searches for,
copies or moves anything.

**Passwords, credentials and keys never go into `AGENTS.md`, the company
knowledge store or the conversation with the agent.** The home is
git-versioned text, and conversations often end up in logs.
