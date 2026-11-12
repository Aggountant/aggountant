# The company home: what is stored where

A company's entire books are in one directory, the company home. This page
explains how the program finds the home, what is stored in it, what belongs
in git and what does not, and what is never written into the home.

## One home, one company

The home holds the company's chart of accounts, vouchers, attachments, the
state of its periods, the company's saved knowledge and its settings. If you
keep the books of two companies, they have two separate homes. A home is not
meant to be used from several people or machines at the same time.

## How aggountant finds the home

Every command needs to know whose books it is working on. The program looks
for the home in this order and uses the first one it finds:

1. the `--home` option on the command
2. the environment variable `AGGOUNTANT_HOME`
3. the `home` value in the configuration file (`aggountant config path` tells
   you where the file is)
4. the nearest `.aggountant` directory, looking upwards from the current
   directory. The search stops at the root of a git repository, and the home
   is the directory that contains `.aggountant`. Setting up a home does not
   create this directory. The agent creates it when it writes the
   Tulorekisteri profile, and you can create it yourself at the root of the
   home (`mkdir .aggountant`) if you want commands run inside the home to find
   it.
5. the subdirectory `accounting` of the current directory.

`aggountant config show` tells you which home is in use and where that came
from, for example `home /…/esimerkki-oy (flag)`. If the path is mistyped, a
command may hit an empty home, and the vouchers seem to have disappeared
([When something goes wrong](../troubleshooting.md#vouchers-seem-to-have-disappeared)).

The configuration file is JSON. For the home you write
`{"home": "/path/to/home"}` in it. The path of the direct VAT filing
credential file is stored in the same file, but the program writes that
itself.

## Git and backups

The home is worth keeping as a git repository. Git holds the chart of
accounts, the company's knowledge, the state of the runs and the settings, so
changes leave a history you can inspect.

**But git is not a backup of your books.** The home's `.gitignore` keeps the
voucher database, the archived attachments and the personal data out of git.
The voucher database and the attachments are the most important part of the
books, and personal data must not be stored in a history from which it cannot
be removed. That is why the whole home is backed up separately:
[Back up and restore your books](../tasks/back-up-and-restore.md).

## The company knowledge store

The agent saves the company facts and practices it learns while working in
the home's `knowledge/company/` directory, for example the fiscal year, how a
particular supplier's invoices are recorded, or the fact that the phone
subscription is entirely for company use. Every item has a source and a
history, so later you can see where it came from and when it changed.
Outdated knowledge is not deleted but retired (`knowledge company retire`), so
the earlier knowledge and the reason for the change are kept.

You can ask the agent to show you the knowledge: "what do you know about the
phone subscription?" The agent then uses the commands
`aggountant knowledge company list`, `aggountant knowledge company search` and
`aggountant knowledge company history`.

## The home's AGENTS.md

aggountant does not dictate where receipts come from or where processed
material goes. You describe that in your own words in the `AGENTS.md` file at
the root of the home. The agent reads it before it looks for, copies or moves
anything. The file is written during setup:
[Set up your company's books](../start/set-up-company.md).

## Secrets do not belong in the home

**Passwords, credentials and keys are never written into `AGENTS.md`, the
company knowledge store or the conversation with the agent.** The home is
version-controlled text, and conversations are often saved in logs. The
Vero.fi and Tulorekisteri credentials are in their own file outside the home,
and the program refuses to use a credential file that is inside the home.

## What the home contains

| In the home | What it is | In git? |
|---|---|---|
| `masterdata/reference/` | chart of accounts and VAT roles | yes |
| `knowledge/company/` | the company's saved facts and practices | yes |
| `process/` | state of close and filing runs, open obligations | yes |
| `vero-filing.json` | direct VAT filing settings, without secrets | yes |
| `.aggountant/tulorekisteri.json` | Tulorekisteri profile, without secrets | yes |
| `AGENTS.md` | the company's own description of its material sources | yes |
| `fiscal.sqlite` | sealed vouchers, period state and the voucher chain | **no** |
| `artifacts/` | archived receipts, invoices and bank statements | **no** |
| `masterdata/pii.sqlite` | employees' personal data | **no** |

Some directories are created only when they are needed.

## See also

- [Back up and restore your books](../tasks/back-up-and-restore.md)
- [Set up your company's books](../start/set-up-company.md)
- [You, the agent and aggountant](division-of-work.md)
