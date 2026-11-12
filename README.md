# aggountant

<!-- shipshape-readme:badges-start -->
[![CI](https://github.com/aggountant/aggountant/actions/workflows/ci.yml/badge.svg)](https://github.com/aggountant/aggountant/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
<!-- shipshape-readme:badges-end -->

aggountant lets you keep a Finnish limited company's books by talking to an AI
agent.

The goal is that you can run a small Finnish company's bookkeeping from start
to finish without hiring an accounting firm: every statutory obligation is
handled, and the whole year's bookkeeping takes a few quarter-hours of
prompting a month. We are not there yet; [Status](#status) says how far we are.

You and the agent gather the month's bank statement and receipts. The agent
works out the entries and the VAT, asks you only what it cannot work out, and
shows you the result. When you approve, aggountant records the entries
permanently, closes the month and sends the VAT return to the Finnish Tax
Administration. Nothing is recorded or sent without your approval.

- [A month with aggountant](#a-month-with-aggountant)
- [Status](#status)
- [Installation](#installation)
- [Getting started](#getting-started)
- [Documentation](#documentation)
- [Under the hood](#under-the-hood)
- [Contributing](#contributing)
- [License](#license)

## A month with aggountant

Say you run Esimerkki Oy and it is time for March:

1. **You and the agent gather the material.** Tell the agent "let's do
   March". It collects the bank statement (the camt.053 or camt.052 file;
   Holvi's "Electronic account statement" export works) and the month's
   receipts from wherever you keep them: your email, a folder, your phone's
   photo library, or the bank and other services through an API or by using
   the browser on your computer. What the agent can reach depends on what you
   give it access to; anything it cannot fetch, you hand it yourself.
2. **The agent prepares.** For each bank transaction it finds the receipt,
   proposes the account and the VAT treatment, and collects what it cannot
   settle into a few questions, such as whether a restaurant bill was a
   customer meeting.
3. **You approve.** The agent shows you each proposed voucher. A €124.00
   software licence, for example, becomes €98.80 of IT expenses, €25.20 of
   VAT receivable and €124.00 out of the bank account. After your approval the
   vouchers are sealed: numbered and never edited again. A mistake is
   corrected with a new voucher, as Finnish accounting law requires.
4. **The month is closed.** Once the bank balance matches the statement, you
   approve closing March, and nothing more can be booked into it.
5. **VAT is filed.** aggountant computes the VAT return from the closed
   month. With your approval it sends the return straight to Vero.fi, or you
   enter the figures in OmaVero by hand. You pay the tax in your bank or
   OmaVero as usual; aggountant pays nothing.

A monthly report (income statement and balance sheet) comes out as an HTML
page to open in a browser.

## Status

Version 0.1, and already in real use: aggountant keeps the books of a few
Finnish companies that have no accounting firm. Features are added as these
companies run into new situations, so the common monthly work is in good shape
and the rarer cases are filled in as they come up. Commands and file formats
can still change between versions.

Ready for real bookkeeping: bank statement import, ordinary purchase receipts
with their VAT, closing a month, monthly reports, the VAT return (filed directly
to Vero.fi or by hand), and tax-free travel allowances (mileage and per diems)
reported to Tulorekisteri, the Incomes Register.

Not ready yet: paying and reporting wages, aggountant's own automated receipt
hunt (`hunt-receipts`; your agent can still look for receipts with its own
tools), and the year-end financial statements (tilinpäätös). The commands exist, but do
not use them for real books; the user guide explains what to do instead.

**We are looking for early users.** If you run a small Finnish company and
want to keep your own books this way, tell us about your company and what you
need in [GitHub Discussions](https://github.com/aggountant/aggountant/discussions)
or by opening an issue. We answer quickly and build what early users actually
need next.

aggountant is for one person keeping one company's books on their own
computer. It is not a hosted or multi-user service, and it does not file on
behalf of other companies.

## Installation
<!-- shipshape-readme:install-start -->
Build from source with Rust 1.89 or newer (install it from
<https://rustup.rs>):

```sh
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
```

aggountant is not published on crates.io.

Prebuilt binaries for macOS arm64 and Linux arm64/x86_64 (static musl), with
SHA-256 checksums, will be attached to each
[GitHub Release](https://github.com/aggountant/aggountant/releases), together
with a shell installer. No release has been published yet; once one is, this
installs the right archive:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aggountant/aggountant/releases/latest/download/aggountant-installer.sh | sh
```
<!-- shipshape-readme:install-end -->

## Getting started

You need an AI coding agent that reads skills, the instruction files that
teach it to work as a bookkeeper. Install the skills that come with
aggountant into the agent's skill directory (`~/.claude/skills` by default;
`--target` points elsewhere):

```sh
aggountant skill list
aggountant skill install
```

Then ask the agent to set up your company's books. It creates the company
directory, records the company's basic facts and opening balances, and notes
where your bank statements and receipts come from. Filing VAT directly to
Vero.fi needs your company's own software registration and certificate from
the Tax Administration; the agent walks you through that when you are ready.

The skills are written in Finnish, but you can talk to the agent in English.

## Documentation

The user guide explains the monthly cycle, VAT and Vero.fi, travel allowances,
year-end status and recovering from errors:

- in Finnish: [`docs/user/fi/`](docs/user/fi/README.md)
- in English: [`docs/user/en/`](docs/user/en/README.md)

Every command also documents itself:

```sh
aggountant --help
```

## Under the hood

aggountant is a command-line program plus the agent skills. The agent never
writes the books itself; it runs aggountant's commands, which check every
entry, compute VAT from the rate table and refuse anything that does not
balance. Each command has a `--json` mode for the agent and readable output for
you.

A company's books live in one directory. The chart of accounts and company
facts are plain text you can keep in git; sealed vouchers are in a local
database where each voucher is chained to the one before it, so
`aggountant voucher verify` detects any later change. If a submission to an
authority has an uncertain outcome, aggountant finds out what happened before
anything is sent again.

## Contributing

Bug reports, questions and pull requests are welcome through
[GitHub Issues](https://github.com/aggountant/aggountant/issues) and
[Discussions](https://github.com/aggountant/aggountant/discussions). See
[CONTRIBUTING.md](CONTRIBUTING.md); commits are signed off under the
Developer Certificate of Origin. Report security vulnerabilities privately as
described in [SECURITY.md](SECURITY.md).

## License
<!-- shipshape-readme:license-start -->
Licensed under the [MIT License](LICENSE). Copyright (c) 2026 Jari Mustonen.
<!-- shipshape-readme:license-end -->
