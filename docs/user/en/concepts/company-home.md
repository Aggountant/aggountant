# The company home: what is stored where

A company's entire books are in one directory, the company home: the chart of
accounts, vouchers, attachments, the state of its periods, the company's saved
knowledge and its settings.

## One home, one company

If you keep the books of two companies, they have two separate homes. A home
is not built for several people or machines to use at the same time.

## How aggountant finds the home

aggountant looks for the home in this order and uses the first one it finds:

1. the `--home` option on the command
2. the environment variable `AGGOUNTANT_HOME`
3. the `home` value in the configuration file
4. the nearest `.aggountant` directory, looking upwards from the current
   directory
5. the subdirectory `accounting` of the current directory.

The configuration file is JSON, and `aggountant config path` tells you where
it is. For the home you write `{"home": "/path/to/home"}` in it. The path of
the direct VAT filing credential file is stored in the same file, and the
program writes that itself.

The search for `.aggountant` stops at the root of a git repository, and the
home is the directory that contains `.aggountant`. Setting up a home does not
create this directory. The agent creates it when it writes the Tulorekisteri
profile, and you can create it yourself at the root of the home
(`mkdir .aggountant`) if you want commands run inside the home to find it.

`aggountant config show` tells you which home is in use and where that came
from, for example `home /…/esimerkki-oy (flag)`. Only `init` creates a home.
If the path is mistyped, every other command stops with `HOME_NOT_INITIALIZED`
and names the path it looked at and how that path was chosen. For what to do
then, see
[Vouchers seem to have disappeared](../troubleshooting.md#vouchers-seem-to-have-disappeared).

## Git and backups

The home is worth keeping as a git repository. Git holds the chart of
accounts, the company's knowledge, the state of the runs and the settings, so
changes leave a history you can inspect.

Git is still not a backup of your books, because the home's `.gitignore`
keeps the most important part of them out of it: the voucher database and the
archived attachments. Personal data stays out too, because it must not be
stored in a history from which it cannot be removed. That is why
[the whole home is backed up separately](../tasks/back-up-and-restore.md).

## The company knowledge store

The agent saves the company facts and practices it learns while working in
the home's `knowledge/company/` directory, for example the fiscal year, how a
particular supplier's invoices are recorded, or the fact that the phone
subscription is entirely for company use. Every item has a source and a
history, so later you can see where it came from and when it changed.
Outdated knowledge is not deleted. It is retired
(`knowledge company retire`), and the earlier knowledge and the reason for
the change are kept.

You can ask the agent to show you the knowledge, for example "what do you
know about the phone subscription?" The agent then uses the commands
`aggountant knowledge company list`, `aggountant knowledge company search` and
`aggountant knowledge company history`.

## The home's AGENTS.md

In the `AGENTS.md` file at the root of the home you describe in your own
words where receipts come from and where processed material goes, because
aggountant dictates neither. The agent reads the file before it looks for,
copies or moves anything. The file is written when you
[set up your company's books](../start/set-up-company.md).

## The home's TODO.md

The `TODO.md` file at the root of the home is a list of what is waiting:
invoices and receipts someone else has promised, which months are still open
and what closing them still needs, and deadlines such as the VAT return and
the financial statements (tilinpäätös). The agent reads it at the start of a
session and updates it when it records the matter that settles an item, so
the next session picks up where the last one left off. The file is plain
text that you can read and edit yourself. The program does not read it.

Pending items do not belong in the company knowledge store, which holds only
lasting facts with their sources. [Obligations](periods.md#obligations)
(`aggountant obligation`) are a different thing too: the program raises them
and they block closing the period.

## Secrets do not belong in the home

Passwords, credentials and keys are never written into `AGENTS.md`, `TODO.md`,
the company knowledge store or the conversation with the agent, because the
home is version-controlled text and conversations are often saved in logs.
The Vero.fi and Tulorekisteri credentials are in their own file outside the
home, and the program refuses to use a credential file that is inside the
home.

## What the home contains

| In the home | What it is | In git? |
|---|---|---|
| `masterdata/reference/` | chart of accounts and VAT roles | yes |
| `knowledge/company/` | the company's saved facts and practices | yes |
| `process/` | state of close and filing runs, open obligations | yes |
| `vero-filing.json` | direct VAT filing settings, without secrets | yes |
| `.aggountant/tulorekisteri.json` | Tulorekisteri profile, without secrets | yes |
| `AGENTS.md` | the company's own description of its material sources | yes |
| `TODO.md` | pending items, open months and deadlines | yes |
| `fiscal.sqlite` | sealed vouchers, period state and the voucher chain | **no** |
| `artifacts/` | archived receipts, invoices and bank statements | **no** |
| `masterdata/pii.sqlite` | employees' personal data | **no** |

Some directories are created only when they are needed.

## See also

- [Back up and restore your books](../tasks/back-up-and-restore.md)
- [Set up your company's books](../start/set-up-company.md)
- [You, the agent and aggountant](division-of-work.md)
