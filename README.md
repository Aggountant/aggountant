<img src="site/public/logo.svg" alt="" width="56" height="56">

# aggountant

<!-- shipshape-readme:badges-start -->
[![CI](https://github.com/aggountant/aggountant/actions/workflows/ci.yml/badge.svg)](https://github.com/aggountant/aggountant/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
<!-- shipshape-readme:badges-end -->

*[Suomeksi](README.fi.md)*

aggountant is an open-source toolkit for AI agents. With it, an agent keeps the statutory books of a small Finnish limited company and offers financial advice, reporting and analysis.

Processing receipts, double-entry bookkeeping, filings with the authorities and the other statutory routines have to be automated. Then the books are always up to date, and the entrepreneur has clear, real-time information about the company's finances at hand.

aggountant already keeps the books of a few Finnish companies. The project is still at an early stage, and not everything planned has been built yet.

**User guide:** [aggountant.dev](https://aggountant.dev) ·
[suomeksi](docs/user/fi/README.md) ·
[in English](docs/user/en/README.md) ·
[Join as an early user](docs/user/en/start/early-users.md)

- [Life with aggountant](#life-with-aggountant)
- [Why the work feels light](#why-the-work-feels-light)
- [Reliability](#reliability)
- [Status](#status)
- [Goals](#goals)
- [Installation](#installation)
- [Getting started](#getting-started)
- [Contributing](#contributing)

## Life with aggountant

When you keep your books with aggountant, an hourly-billed agreement with an accounting firm is enough: you buy expert help when you need it.

In a small company, an ordinary month's bookkeeping takes fifteen minutes to half an hour, for example on the first day of the following month.

You ask the agent to process the previous month. When your agent is connected to your bank and your email, it fetches the bank statement and transactions with their attachments, and looks for the remaining receipts in your email and in a folder where you keep, for example, photos of paper receipts. If something is missing, the agent tells you and helps you get it. Once you know the material is processed automatically, you can arrange things so that receipts land in the right place as they are created, and few go missing.

The agent can also use contracts, timesheets and your calendar. From them come customer invoices and travel and expense claims, and the agent works out their effect on the books.

When the material is complete, the agent proposes the entries and aggountant checks them. You approve the vouchers, and the agent proposes closing the month. You can go through the material in as much detail as you like, but usually there is nothing in particular to look into.

From the closed month, aggountant computes the VAT return and, with your approval, sends it straight to the Finnish Tax Administration. It reports travel expense reimbursements to the Incomes Register. Your agent can prepare the VAT payment and the reimbursements in your bank, for example Holvi, where you approve the payments in the mobile app. aggountant itself pays nothing.

Not everything is always done on the first day, because sometimes you wait for a service provider's report or a missing receipt. When the material arrives later, you ask the agent to continue where you left off. What is left for you is mostly checking the whole, approving payments and now and then supplying a single receipt.

The bank, email and invoicing connections are your agent's own tools, not part of aggountant. aggountant reads the bank statement as a camt.053 or camt.052 file, and the agent fetches material from wherever you give it access.

## Why the work feels light

Traditional bookkeeping is built around the fact that an accountant's time costs money. Receipts have to be delivered in the right form, and every extra question can mean an extra bill.

With an agent, that limit is gone. If you bought something on the wrong card by mistake, you can sort it out right away. If a receipt is missing, your agent looks for it as long as it takes. If you want to understand why an entry was made, the agent examines the receipts, the legislation and the Tax Administration's guidance.

This also changes how bookkeeping is best organised. An online shop's thousands of sales do not need to be bundled into summary vouchers to save work. The level of detail can be chosen by what you want to know about the business, not by what minimises the accountant's work.

The biggest benefit is not the saving but that the whole of the books is always available. You can ask about your company's finances at any time, compare costs or ask for a new report. When the agent is connected to the bank and other services, most of the month's material can be processed before the month ends, and you can follow the finances all the time. Bookkeeping becomes an ongoing conversation about the company's finances instead of a duty done once a month.

## Reliability

The correctness of the books is not left to an AI model's reasoning.

- **The computation is ordinary code.** aggountant contains no AI. It numbers the vouchers, checks that debits and credits balance, computes VAT from the rate table and refuses anything that does not balance. The agent interprets and gathers material, but never writes the books itself.
- **The law decides, not the request.** The agent proposes what an expense is, and aggountant computes the lawful consequence. If, for example, you ask to deduct 70 per cent of the VAT on a passenger car that is also used privately, aggountant deducts nothing, because the law does not allow it. The agent explains why and helps find the right treatment.
- **A person approves permanent acts.** Sealing a voucher, closing a month and sending a filing wait for your approval. Without permission the agent may only read and preview.
- **The books are traceable.** A sealed voucher is never edited; a mistake is corrected with a new voucher, as Finnish accounting law requires. Each voucher is chained to the one before it, so `aggountant voucher verify` detects any later change. If the outcome of a submission is uncertain, aggountant finds out what happened before anything is sent again.
- **The records do not depend on an AI service.** The company's books live in their own directory, partly as plain text files, and stay readable and auditable whatever model is in use.

No system is infallible. That is why programmatic checks, the agent's workflows and, when needed, an accountant's review complement each other.

## Status

Version 0.2, and already in real use. Features are added as the companies using it run into new situations, so the ordinary monthly work is in good shape and rarer cases are filled in as they come up. Commands and file formats can still change between versions.

**Ready for real bookkeeping:** bank statement import, ordinary purchase receipts with their VAT, closing a month, monthly reports, the VAT return (filed directly to the Tax Administration or by hand), and tax-free travel allowances (mileage and per diems) reported to the Incomes Register.

**Not ready yet:** paying and reporting wages, aggountant's own receipt hunt, and the year-end financial statements. The commands exist, but do not use them for real books; the user guide explains what to do instead.

aggountant is for one person keeping one company's books on their own computer. It is not a hosted or multi-user service, and it does not file on behalf of other companies.

## Goals

The first use is the bookkeeping of a small Finnish limited company. The long-term goal is a general, international financial-administration toolkit for AI agents.

- **Financial statements.** When the books have been kept carefully through the year, the year-end accounts should come together largely by talking to the agent, and can then go to an accountant for review if you want.
- **Payroll.** Wages, payslips, preparing the payments and the tax and Incomes Register reports, without a separate payroll service.
- **International sales.** Tax obligations for selling digital products in other countries handled by your own agent, without a Merchant of Record service such as Paddle or Lemon Squeezy.
- **International employment.** Payroll and employer obligations for people working abroad, without an Employer of Record service such as Deel or Remote, to the extent the administrative work can be automated.

aggountant is open source because statutory obligations apply to every company, and the foundation for automating them does not need to be held by individual service providers. Openness also means the bookkeeping rules, data models and checks are open for anyone to examine. The aim is not to replace financial professionals but to free them from routine work for where their expertise helps most.

## Installation
<!-- shipshape-readme:install-start -->
Prebuilt binaries are available for macOS arm64 and Linux arm64/x86_64 (static musl). This installs the right archive from the latest [release](https://github.com/aggountant/aggountant/releases):

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aggountant/aggountant/releases/latest/download/aggountant-installer.sh | sh
```

Each release's archives come with SHA-256 checksums. You can also build from source with Rust 1.89 or newer (<https://rustup.rs>):

```sh
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
```
<!-- shipshape-readme:install-end -->

## Getting started

The easiest way is to tell your agent that you want to start using aggountant and give it a link to this repository: <https://github.com/aggountant/aggountant>. The agent installs the program and the skills, the instruction files that teach it to work as a bookkeeper, and sets up your company's books. It creates the company directory, records the company's basic facts and opening balances, and notes where your bank statements and receipts come from. Filing VAT directly to the Tax Administration needs your company's own software registration and certificate; the agent walks you through that when you are ready.

The skills are installed into the agent's skill directory (`~/.claude/skills` by default; `--target` points elsewhere):

```sh
aggountant skill list
aggountant skill install
```

The skills are written in Finnish, but you can talk to the agent in English.

The user guide covers the monthly cycle, VAT, travel allowances, year-end status and recovering from errors: <https://aggountant.dev>. Every command also documents itself: `aggountant --help`.

## Contributing

We are looking for early users and contributors. If you run a small Finnish company and want to keep your books this way, we can go through your situation together: [Join as an early user](docs/user/en/start/early-users.md).

Bug reports, questions and pull requests are welcome through [GitHub Issues](https://github.com/aggountant/aggountant/issues) and [Discussions](https://github.com/aggountant/aggountant/discussions). See [CONTRIBUTING.md](CONTRIBUTING.md); commits are signed off under the Developer Certificate of Origin. Report security vulnerabilities privately as described in [SECURITY.md](SECURITY.md).

## License
<!-- shipshape-readme:license-start -->
Licensed under the [MIT License](LICENSE). Copyright (c) 2026 Jari Mustonen.
<!-- shipshape-readme:license-end -->
