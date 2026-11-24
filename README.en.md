<img src="site/public/logo.svg" alt="" width="56" height="56">

# aggountant

<!-- shipshape-readme:badges-start -->
[![CI](https://github.com/aggountant/aggountant/actions/workflows/ci.yml/badge.svg)](https://github.com/aggountant/aggountant/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
<!-- shipshape-readme:badges-end -->

*[Suomeksi](README.md)*

aggountant is an open-source tool for AI agents. With it, an agent keeps the statutory books of a small Finnish limited company and provides financial advice, reporting and analysis.

It has to be possible to automate receipt processing, double-entry bookkeeping, filings with the authorities and the other statutory routines. Then the books are always up to date, and the business owner has clear, real-time information about the company's finances at hand.

aggountant is already used for the bookkeeping of a few Finnish companies. The project is still at an early stage, though, and not everything planned has been built yet.

**User guide:** [aggountant.dev](https://aggountant.dev) ·
[suomeksi](docs/user/fi/README.md) ·
[in English](docs/user/en/README.md) ·
[Become an early user](docs/user/en/start/early-users.md)

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

In a small company, the bookkeeping for an ordinary month takes fifteen minutes to half an hour, for example on the first day of the following month.

You ask the agent to process the previous month. When your agent is connected to your bank and email, it fetches the bank statement and the transactions with their attachments, and looks for the remaining receipts in your email and in the folder where you save, for example, photos of paper receipts. If something is missing, the agent tells you and helps you get hold of it. Once you know the material is processed automatically, you can arrange things so that receipts end up in the right place the moment they are created, and few go missing.

The agent can also use contracts, time entries and your calendar. Customer invoices and travel and expense claims are produced from them, and the agent handles their effects on the books.

Once the material is complete, the agent proposes the entries and aggountant checks them. You approve the vouchers, and the agent proposes closing the month. You can go through the material as closely as you like, but usually there is nothing in particular to sort out.

For the closed month, aggountant calculates the VAT return and, with your approval, sends it directly to the Finnish Tax Administration. It reports travel expense reimbursements to the Incomes Register. Your agent can prepare the VAT payment and the expense reimbursements in your bank, for example Holvi, where you approve the payments in the mobile app. aggountant itself does not pay anything.

Not everything always gets finished on the first day, because sometimes you are waiting for a service provider's report or a missing receipt. When the material arrives later, you ask the agent to pick up where you left off. What is left for you is mostly checking the overall picture, approving payments and occasionally supplying a single receipt.

The bank, email and invoicing connections are your agent's own tools, not part of aggountant. aggountant reads the bank statement as a camt.053 or camt.052 file, and the agent fetches the material from wherever you give it access.

## Why the work feels light

Traditional bookkeeping is built around the fact that an accountant's time costs money. Receipts have to be delivered in the right format, and every extra enquiry can mean an extra invoice.

With an agent, that constraint does not exist. If you accidentally bought something with the wrong card, you can get it sorted out right away. If a receipt is missing, your agent looks for it for as long as it takes. If you want to understand the grounds for an entry, the agent goes through the vouchers, the legislation and the Finnish Tax Administration's guidance.

This also changes how it makes sense to organise bookkeeping. An online shop's thousands of sales don't need to be bundled into summary vouchers because of the workload. The level of detail in the books can be chosen according to what you want to know about the business, not according to what minimises the accountant's work.

The biggest benefit is not the savings but that the whole of your books is available all the time. You can ask about your company's finances whenever you like, compare costs or ask for a new report. When the agent is connected to the bank and other services, most of the month's material may already be processed before the month ends, and you can follow the financial situation continuously. Bookkeeping becomes an ongoing conversation about the company's finances rather than an obligation taken care of once a month.

## Reliability

The correctness of the books is not left to an AI model's reasoning.

- **The calculation is ordinary program code.** aggountant contains no AI. It numbers the vouchers, checks that debits and credits balance, calculates VAT from a tax rate table and refuses anything that doesn't add up. The agent interprets and gathers material, but does not write the books itself.
- **The law decides, not the request.** The agent proposes what an expense is, and aggountant works out the consequence the law prescribes. If, for example, you ask it to deduct 70 per cent of the VAT on a passenger car that is also used privately, aggountant deducts zero, because the law does not allow the deduction. The agent tells you why and helps you find the correct treatment.
- **A human approves permanent actions.** Sealing a voucher, closing a month and submitting a filing wait for your approval. Without permission, the agent may only read and preview.
- **The books are traceable.** A sealed voucher is not edited; a mistake is corrected with a new voucher, as the Finnish Accounting Act requires. Every voucher is chained to the previous one, so `aggountant voucher verify` detects changes made afterwards. If the outcome of submitting a filing is left uncertain, aggountant finds out what happened before anything is sent again.
- **The records do not depend on an AI service.** The company's books are in their own directory, partly as plain text files, and they stay readable and auditable regardless of which model is in use.

No system is infallible. That is why programmatic checks, the agent's workflows and, where needed, an accountant's judgement complement each other.

## Status

Version 0.2, and already in real use. Features are added as the companies using it run into new situations, so the ordinary monthly work is in good shape and the rarer cases are being filled in. Commands and file formats may still change between versions.

**Ready for real bookkeeping:** bank statement import, ordinary purchase vouchers with VAT, closing the month, monthly reports, the VAT return (directly to the Finnish Tax Administration or by hand), and tax-exempt travel expense reimbursements (mileage allowances and per diems) to the Incomes Register.

**Not ready yet:** paying wages and reporting them, aggountant's own receipt retrieval, and the financial statements. The commands exist, but they should not be used for real bookkeeping. The user guide explains what to do instead.

aggountant is meant for one person keeping one company's books on their own machine. It is not an online service or a multi-user system, and it does not file on behalf of other companies.

## Goals

The first use case is the bookkeeping of a small Finnish limited company. The long-term goal is a general-purpose, international financial administration toolkit for AI agents.

- **Financial statements.** When the books have been kept carefully throughout the year, the financial statements should largely come together by talking with the agent, and can be handed to an accountant to check if needed.
- **Payroll.** Wages, payslips, payment preparation, and tax and Incomes Register filings without a separate payroll service.
- **International sales.** The tax obligations of selling digital products in different countries, handled by your own agent without a Merchant of Record service such as Paddle or Lemon Squeezy.
- **International employment.** Payroll administration and employer obligations for people working abroad without an Employer of Record service such as Deel or Remote, to the extent that the administrative work can be automated.

aggountant is open source because statutory obligations apply to every company, and the foundation for automating them does not need to be in the hands of individual service providers. Openness also means that the bookkeeping rules, data models and checks are open for anyone to examine. The aim is not to replace finance professionals, but to free them from routine work so they can work where their expertise is of most use.

## Installation
<!-- shipshape-readme:install-start -->
Prebuilt binaries are available for macOS arm64 and Linux arm64/x86_64 (static musl). This installs the right package from the latest [release](https://github.com/aggountant/aggountant/releases):

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aggountant/aggountant/releases/latest/download/aggountant-installer.sh | sh
```

Each release's packages come with SHA-256 checksums. You can also build the program from source with Rust 1.89 or newer (<https://rustup.rs>):

```sh
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
```
<!-- shipshape-readme:install-end -->

## Getting started

The easiest way is to tell your agent that you want to start using aggountant and give it a link to this repository: <https://github.com/aggountant/aggountant>. The agent installs the program and the skills, which are instruction files that teach it to work as a bookkeeper, and sets up your company's books. It creates the company's directory, saves the basic details and opening balances, and records where the bank statements and receipts come from. Filing VAT directly with the Finnish Tax Administration requires the company's own software registration and certificate, and the agent guides you through that when you are ready.

The skills are installed in the agent's skill directory (`~/.claude/skills` by default, elsewhere with the `--target` option):

```sh
aggountant skill list
aggountant skill install
```

The skills are written in Finnish, but you can talk to the agent in English.

The user guide covers the monthly round, VAT, travel expense reimbursements, the state of the financial year and recovering from errors: <https://aggountant.dev>. Every command also documents itself: `aggountant --help`.

## Contributing

We are looking for early users and developers. If you run a small Finnish company and want to keep your books this way, get in touch: [Become an early user](docs/user/en/start/early-users.md).

Bug reports, questions and pull requests are welcome in [GitHub Issues](https://github.com/aggountant/aggountant/issues) and [Discussions](https://github.com/aggountant/aggountant/discussions). See [CONTRIBUTING.md](CONTRIBUTING.md); commits are signed off under the Developer Certificate of Origin. Report security vulnerabilities privately as described in [SECURITY.md](SECURITY.md).

## License
<!-- shipshape-readme:license-start -->
Licensed under the [MIT License](LICENSE). Copyright (c) 2026 Jari Mustonen.
<!-- shipshape-readme:license-end -->
