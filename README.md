# aggountant

<!-- shipshape-readme:badges-start -->
[![CI](https://github.com/aggountant/aggountant/actions/workflows/ci.yml/badge.svg)](https://github.com/aggountant/aggountant/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
<!-- shipshape-readme:badges-end -->

Bookkeeping for a Finnish limited company, run by you and an AI agent:
**kirjanpito** (double-entry bookkeeping), **ALV** (VAT returns filed to
Vero.fi), payroll reports to Tulorekisteri, and the year-end **tilinpäätös**
(financial statements).

aggountant is a command-line tool plus a set of agent skills. The skills are
the agent's operating manual: they tell an AI coding agent how to turn bank
statements, receipts and your answers into vouchers, close a month, and file
returns. The CLI does the bookkeeping itself, with strict validation,
`--json` output and explicit approval steps, so the agent never writes the
books directly and nothing is filed without your say-so.

## Contents

- [Status](#status)
- [How it works](#how-it-works)
- [Installation](#installation)
- [Getting started](#getting-started)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

## Status

Early, 0.x. aggountant already keeps the live books of one Finnish limited
company, including a closed period and an accepted VAT return, but it has had
one user so far. Expect rough edges, commands and on-disk formats that still
change between minor versions, and skills and a user guide written in Finnish
first. It is built for one trusted user on one machine with one company per
data home; it is not a multi-user or hosted service.

## How it works

- **A company home is a directory.** Masterdata (chart of accounts, company
  facts) is plain text you keep in git; sealed vouchers live in a local SQLite
  store with content hashes, and source documents in a content-addressed
  artifact store. A sealed voucher is never edited; corrections are new
  vouchers.
- **The agent works through the CLI.** Every command validates its input,
  explains failures in machine-readable errors and has a `--json` mode, so an
  agent can drive it and you can read what it did.
- **Sources.** The CLI imports camt.053/camt.052 bank statements (for example
  Holvi's electronic account statement export) and captures receipts and
  invoices as evidence for vouchers.
- **Authorities.** VAT returns go directly to Vero.fi with your own software
  registration and certificate, and an uncertain submission is reconciled
  rather than resent. Wage and travel-allowance reports go to Tulorekisteri.

## Installation
<!-- shipshape-readme:install-start -->
Prebuilt binaries for macOS arm64 and Linux arm64/x86_64 (static musl) are
attached to each [GitHub Release](https://github.com/aggountant/aggountant/releases),
with SHA-256 checksums. The shell installer downloads the right archive:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aggountant/aggountant/releases/latest/download/aggountant-installer.sh | sh
```

On other platforms, or to build from source (Rust 1.89 or newer):

```sh
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
```

aggountant is not published on crates.io.
<!-- shipshape-readme:install-end -->

## Getting started

Install the bundled skills into your agent's skill directory (the default
target is `~/.claude/skills`; pass `--target` for another runtime):

```sh
aggountant skill list
aggountant skill install
```

Then ask your agent to set up the company's books. The `aggountant-init`
skill walks it through creating a company home, recording the company's
basic facts and opening balances, and describing where your source material
comes from. The technical part is one command:

```sh
aggountant init --home ./accounting
```

Every command documents itself; start with:

```sh
aggountant --help
```

## Documentation

The user guide is in [`docs/user/`](docs/user/): installation, the monthly
cycle, VAT filing with your own Vero.fi software registration, payroll, the
year-end close, and recovering from errors. Maintainers' notes on the binary
release channel are in [`docs/DISTRIBUTION.md`](docs/DISTRIBUTION.md).

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
