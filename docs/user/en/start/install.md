# Install aggountant and its skills

Before the agent can keep your books, two things are installed on your
computer: the aggountant program and the skills that come with it, the
instructions from which the agent learns to work as a bookkeeper. You install
them once, and again every time you upgrade the program.

## Before you start

- The Rust toolchain, version 1.89 or newer. Installation instructions are at
  <https://rustup.rs>. There are no ready-made installation packages yet, so
  aggountant is compiled from source.
- An agent that can read skills, for example Claude Code.

## How to do it

1. **Install the program.** You can do this step yourself in a terminal or ask
   the agent to do it:

   > Install aggountant from source on GitHub.

   Compiling takes a few minutes.

2. **Check that the program is found.** `aggountant --version` prints the
   version. `aggountant --help` lists all commands, and every command has its
   own help. The agent reads this help itself, so you do not need to learn the
   commands.

3. **Install the skills.** Ask the agent:

   > Install aggountant's skills.

   There are three skills:

   | Skill | What the agent uses it for |
   |---|---|
   | `aggountant-init` | setting up a company home, or continuing an unfinished setup |
   | `capture-receipt` | handling receipts, vouchers and the month's transactions |
   | `vero-fi` | setting up direct VAT filing, the VAT return and Tulorekisteri travel allowances |

   By default the skills go into Claude Code's skill directory
   `~/.claude/skills/`. For another agent, give its skill directory with
   `--target`.

4. **Restart the agent** so that it reads the new skills.

<details>
<summary>Commands the agent runs</summary>

```sh
cargo install --git https://github.com/aggountant/aggountant aggountant --locked
aggountant --version
aggountant skill list
aggountant skill install
aggountant skill install --target <the agent's skill directory>
aggountant skill print capture-receipt
```

If you already have the source code, install from its root with
`cargo install --locked --path crates/cli`. `skill print` shows a skill's
content without installing anything. When you install `capture-receipt` on
its own, `aggountant-init` comes with it, because they share a role
description.
</details>

## Done when

- `aggountant --version` prints the version, for example `aggountant 0.1.0`.
- `aggountant skill list` lists three skills, and they are in the agent's
  skill directory.

The skills are written in Finnish, but you can talk to the agent in English.

## Next

[Practise your first month](practice-month.md) in a separate practice home,
or if you have already done that,
[set up your company's books](set-up-company.md).

## See also

- [Upgrade aggountant](../tasks/upgrade.md)
- [You, the agent and aggountant](../concepts/division-of-work.md)
