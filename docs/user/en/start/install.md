# Install aggountant and its skills

Two things are installed on your computer: the aggountant program and the
skills that come with it, the instructions from which the agent learns to work
as a bookkeeper. You install them once, and again every time you upgrade the
program.

## Before you start

- A Mac with an Apple processor (arm64) or a Linux machine (arm64 or x86_64).
  These have a prebuilt package. On an Intel Mac aggountant is compiled from
  source, and for that you need the Rust toolchain, version 1.89 or newer;
  installation instructions are at <https://rustup.rs>. Windows has not been
  tested.
- An agent that can read skills, for example Claude Code.

## How to do it

1. **Install the program.** You can do this yourself in a terminal or ask the
   agent:

   > Install aggountant.

   The installer fetches the latest release from GitHub and picks the package
   that fits your machine. If there is no package, ask the agent to install
   aggountant from source. Compiling takes a few minutes.

2. **Check that the program is found.** `aggountant --version` prints the
   version. The program goes into `~/.cargo/bin`, and the installer adds
   that directory to your command path. If the command is not found, open a
   new terminal. `aggountant --help` lists all commands, and every command has its
   own help. The agent reads it itself, so you do not need to learn the
   commands.

3. **Install the skills.** Ask the agent:

   > Install aggountant's skills.

   The agent gets these skills:

   | Skill | What the agent uses it for |
   |---|---|
   | `aggountant-init` | setting up a company home with a readiness checklist and one source request, or continuing an unfinished setup |
   | `migrate` | moving from another program one area at a time (the books, VAT, the Incomes Register, expense claims): exporting, checking and archiving the old material |
   | `capture-receipt` | the month's books: receipts, the bank statement, manual entries, reconciliation and the month close |
   | `vero-fi` | the VAT return to the Tax Administration and setting up direct VAT filing |
   | `tulorekisteri` | the Incomes Register travel allowance report and the employer's separate report |
   | `close-year` | closing a fiscal year: the year-end entries, the approvals and sealing the year |
   | `e-services` | working OmaVero and the Incomes Register e-service in your browser: how their pages behave, as dated observations |

   By default the skills go into Claude Code's skill directory
   `~/.claude/skills/`. For another agent, give its skill directory with
   `--target`. The skills are written in Finnish, but you can talk to the
   agent in English.

4. **Restart the agent** so that it reads the new skills.

<details>
<summary>Commands the agent runs</summary>

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/aggountant/aggountant/releases/latest/download/aggountant-installer.sh | sh
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
aggountant --version
aggountant skill list
aggountant skill install
aggountant skill install --target <the agent's skill directory>
aggountant skill print capture-receipt
aggountant skill print aggountant-init --file SOUL.md
```

The first command installs the prebuilt package.

`cargo install` compiles the program from source and is needed only when
there is no prebuilt package. If you already have the source code, do the
same from its root with `cargo install --locked --path crates/cli`.

When you install `capture-receipt` on its own, `aggountant-init` comes with
it, because they share a role description. `vero-fi` and `tulorekisteri` also
bring each other and `e-services`, which they rely on for the steps done in
the authorities' web services. `migrate` brings all of these, because the
VAT and Incomes Register parts of a move continue in them.

`migrate` used to be called `migrate-books`. If the skill directory still
has the old `migrate-books`, installing replaces its `SKILL.md` with a
pointer to `migrate`, so that the agent does not read the old instructions.
Installing never deletes anything, so you can remove the directory yourself.
`skill install migrate-books` answers with the error `SKILL_RENAMED`, which
names the new name.

`skill print` shows a skill's content without installing anything. `--file`
shows the skill's other files, such as the shared role description `SOUL.md`.
</details>

## Done when

- `aggountant --version` prints the version number.
- `aggountant skill list` lists seven skills, and they are in the agent's
  skill directory.

## Next

[Practise your first month](practice-month.md) in a separate practice home,
or if you have already done that,
[set up your company's books](set-up-company.md).

## See also

- [Upgrade aggountant](../tasks/upgrade.md)
- [You, the agent and aggountant](../concepts/division-of-work.md)
