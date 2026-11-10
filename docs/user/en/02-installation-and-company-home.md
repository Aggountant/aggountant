# 2. Installation and setting up a company home

## Installing the program

There are no ready-made installation packages yet. For now aggountant is built
from source, which needs the Rust toolchain (version 1.89 or newer, installed
from <https://rustup.rs>). In the source directory, install it like this:

```sh
cargo install --locked --path crates/cli
```

Check that the program is found:

```sh
aggountant --version
aggountant --help
```

`aggountant --help` lists every command, and each command has its own help, for
example `aggountant voucher seal --help`. The agent reads this help itself. You
do not need to learn the commands by heart.

## What a company home is

A company home is one directory that holds one company's entire bookkeeping:

| In the home | What it is | Goes into git |
|---|---|---|
| `masterdata/reference/` | chart of accounts and VAT roles | yes |
| `knowledge/company/` | the company's stored facts and practices | yes |
| `process/` | the state of close and filing runs, open obligations | yes |
| `vero-filing.json` | Vero.fi settings, without secrets | yes |
| `AGENTS.md` | your own description of where material comes from (chapter 3) | yes |
| `fiscal.sqlite` | sealed vouchers, period status and the voucher chain | **no** |
| `artifacts/` | archived receipts, invoices and bank statements | **no** |
| `masterdata/pii.sqlite` | employees' personal data | **no** |

## Setting up the home

Choose a place for the home and run `init`:

```sh
aggountant --home ~/accounting/esimerkki-oy init
```

The command creates the directory, the storage structures, the default chart of
accounts (`fi-general-2026`, 98 accounts) and a `.gitignore` file. If the home
already exists, `init` refuses (`ALREADY_INITIALIZED`) and leaves it alone. So
you cannot accidentally wipe existing books with it, and neither can you use it
to repair or upgrade an old home.

`init` does not ask for the company's name, Business ID (Y-tunnus) or fiscal
year. Those are collected during onboarding into the company knowledge store,
and the agent handles this with the `aggountant-init` skill (chapter 3).

The chart of accounts uses Finnish account names. You can list the accounts
with:

```sh
aggountant chart list
```

## How the program finds the home

Every command needs to know which company's books it is working on. The program
looks for the home in this order and uses the first one it finds:

1. the `--home` option on the command
2. the environment variable `AGGOUNTANT_HOME`
3. the `home` value in the configuration file (`aggountant config path` shows
   where that file is)
4. the nearest `.aggountant` directory, searching upwards from the current
   directory. The search stops at the root of a git repository. The home is the
   directory that contains `.aggountant`. `init` does not create this marker:
   create it yourself in the home's root (`mkdir .aggountant`) if you want
   commands run inside the home to find it. The agent also creates it when it
   writes the Tulorekisteri profile (chapter 6).
5. the subdirectory `accounting` of the current directory.

`aggountant config show` tells you which home is in use and where that came
from. Run it whenever you are unsure. If you mistype the home path, the program
may create an empty directory in the wrong place (a known shortcoming). That
does no harm to your real books, but your vouchers will seem to have
disappeared until you notice the mistake.

The configuration file is JSON. To set the home, write
`{"home": "/path/to/home"}` in it. The path to your Vero.fi credentials
(chapter 5) is stored in the same file, but the program writes that itself.

## Git and backups

The home should be a git repository. The chart of accounts, company facts, run
state and settings go into git, so changes leave a history you can review.

**Git is not a backup of your books, though.** `init` excludes the voucher
database (`fiscal.sqlite`), the archived attachments (`artifacts/`) and the
personal data (`masterdata/pii.sqlite`) from git. The voucher database and the
attachments are the most important part of the books, and personal data must
not be stored in a history it cannot be removed from.

So back up the whole home directory separately, including the files git
ignores:

- Copy the home while no aggountant command is running. Include every file
  whose name starts with `fiscal.sqlite`, and the `artifacts/` directory.
- Keep the copy somewhere other than the original. Attachments must be kept
  for the retention period the Accounting Act requires.
- When you restore a copy, run `aggountant voucher verify` first. It checks that
  the voucher chain is intact.

aggountant has no backup or export command of its own.

## Moving over from another system

If the company is moving to aggountant from another bookkeeping program, there
is more to onboarding than `init`. The agent goes through it with you using the
`aggountant-init` skill:

- **Basic details**: the official name, Business ID, fiscal year, the date from
  which bookkeeping in aggountant starts, and whether the old chart of accounts
  is carried over. They are stored with their sources in the company knowledge
  store. The best source is an extract from the Trade Register or a report from
  the old system, not an answer from memory.
- **Opening balances** are booked as one ordinary voucher, with the old
  system's balance list attached. The voucher is dated on the last day in the
  old system. If you date it on the first day of the new books, the whole
  earlier result shows up as the result of the first month.
- **Open matters** that the balance list does not show: whether company bills
  have been paid from your own account, whether there is a tax debt or a
  payment plan on the tax account, and whether there are unpaid sales or
  purchase invoices.

After the opening, the agent produces a report for the first month
(chapter 7). In it, the month's result must be zero before any new
transactions, the balance sheet must match the old system's report, and the
bank account balance must match the bank statement.

Importing several years of history from the old system is not ready yet.
Onboarding starts from the opening balances.

## Fiscal year and periods

aggountant treats the fiscal year as the calendar year. The books are closed
monthly (`2026-03`) or quarterly (`2026-Q1`), but you cannot use both in the
same year. Monthly closing is the recommended and better-tested option. A
fiscal year that differs from the calendar year is not supported yet.
