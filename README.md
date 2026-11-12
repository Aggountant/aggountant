# aggountant

<!-- shipshape-readme:badges-start -->
[![CI](https://github.com/aggountant/aggountant/actions/workflows/ci.yml/badge.svg)](https://github.com/aggountant/aggountant/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
<!-- shipshape-readme:badges-end -->

aggountant lets you keep a Finnish limited company's books by talking to an AI
agent.

You hand the agent the month's bank statement and receipts. It works out the
entries and the VAT, asks you only what it cannot work out, and shows you the
result. When you approve, aggountant records the entries permanently, closes the
month and sends the VAT return to the Finnish Tax Administration. Nothing is
recorded or sent without your approval.

- [A month with aggountant](#a-month-with-aggountant)
- [Status](#status)
- [Installation](#installation)
- [Getting started](#getting-started)
- [Documentation](#documentation)
- [Under the hood](#under-the-hood)
- [Contributing](#contributing)
- [License](#license)

## A month with aggountant

Esimerkki Oy's owner does March's books in an afternoon:

1. **Hand over the material.** They download March's bank statement from the
   bank (the camt.053 or camt.052 file; Holvi's "Electronic account statement"
   export works) and put the month's receipts in the folder the agent knows to
   look in. Then they tell the agent: "let's do March".
2. **The agent prepares.** For each bank transaction it finds the receipt,
   proposes the account and the VAT treatment, and collects what it cannot
   settle into a few questions, such as whether a restaurant bill was a
   customer meeting.
3. **The owner approves.** The agent shows each proposed voucher for
   approval. A €124.00 software licence, for example, becomes €98.80 of IT expenses, €25.20
   of VAT receivable and €124.00 out of the bank account. After approval the
   vouchers are sealed: numbered and never edited again. A mistake is corrected
   with a new voucher, as Finnish accounting law requires.
4. **The month is closed.** Once the bank balance matches the statement, the
   owner approves closing March, and nothing more can be booked into it.
5. **VAT is filed.** aggountant computes the VAT return from the closed month.
   With the owner's approval it sends the return straight to Vero.fi, or the
   owner enters the figures in OmaVero by hand. The tax itself is paid in the
   bank or OmaVero as usual; aggountant pays nothing.

A monthly report (income statement and balance sheet) comes out as an HTML
page to open in a browser.

## Status

Early: version 0.1. aggountant keeps the live books of one Finnish limited
company, but it has had one user so far. Expect rough edges and commands and
file formats that still change between versions.

Ready for real bookkeeping: bank statement import, ordinary purchase receipts
with their VAT, closing a month, monthly reports, the VAT return (filed directly
to Vero.fi or by hand), and tax-free travel allowances (mileage and per diems)
reported to Tulorekisteri, the Incomes Register.

Not ready yet: paying and reporting wages, automatic receipt search from email,
and the year-end financial statements (tilinpäätös). The commands exist, but do
not use them for real books; the user guide explains what to do instead.

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
