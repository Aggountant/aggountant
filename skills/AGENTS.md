# skills/ — the bundled agent skills

Each directory is one skill that `aggountant skill install` writes into the
user's agent. The skills are written in Finnish, for the user's agent, and are
its manual; the user guide in `docs/user/` is for people.

## What goes into a skill

The reader is a capable agent with the binary at hand. It reads `--help`
before it runs a command and reads the output afterwards, so a skill earns its
words only with what the program cannot tell it: why a step exists, what a
mistake costs and when it shows, the traps, and how the authorities behave in
practice. A skill that also restates the program buries those sentences, and
the restatement goes stale the first time the command changes. In November
2026 one skill had grown to more than half of all skill text this way, slice
by slice, while the month close, the work an agent does every month, was in no
skill at all.

**The program describes itself.** Commands' flags, input formats, field names,
states and error codes belong to `--help` and the output. A skill names a
command, or a flag or field that a consequence hangs on, to say when and why
to reach for it, not to document it. Wanting to write an input format, a list
of states or "the help does not show this" into a skill means the program is
missing something: give the command an INPUT section in its help (`voucher
seal --help` has one), or a `message` and `next_command` that say where the
user stands and what settles it, or an error message that says what to do. Fix
that in the slice. When the fix is outside the slice's files, file it as an
issue; what the agent cannot work without stays in the skill until the program
shows it, and the issue names the lines to remove then. The same goes for the
program's state machine, which `crates/AGENTS.md` already keeps out of skills:
the agent needs the principle behind the states (a filing is never resent on a
guess), not their names.

**One fact, one home.** `aggountant-init/SOUL.md` is read with every skill, so
it holds what applies to every task and nothing else: the role; the house
rules of the company home (what goes into its `AGENTS.md` and `TODO.md`, into
`knowledge company` and `obligation`, and that secrets go into none of them);
what in this work is permanent and why the user approves it first; and how to
read the tool. Task skills rely on it and do not repeat it; they name the
point in their own task where a house rule or an approval applies. A new
feature's reminder goes into the task skill where the agent meets the feature,
not into SOUL because everyone reads SOUL. A fact two task skills need lives
in one of them, and the other links to it.

**The body is what the task needs every time.** What a task needs rarely
(setting a filing channel up, incidents, catching up history) goes into a
reference file next to the skill, linked from the place where the need arises.
A task skill whose body passes about a thousand words is carrying something
that belongs elsewhere: split it or move material out before adding to it.

**A boundary is stated once, with its reason.** The real limits here are few,
such as what cannot be undone and what the user approves. Bold type and a
prohibition in every paragraph make them impossible to tell from the rest.

**Only what works, and only how it is now.** A path that is built but not
connected reads as usable to an agent; it belongs in the guide's
`reference/status.md`, in both languages. How the code used to behave and how
it is implemented give the agent nothing to act on. A dated observation of how
an authority behaves is knowledge and stays, with its date.

**A changed command takes its skills along.** The slice that renames or
changes a command checks the skills naming it in the same change, and updates
what no longer holds, as it does with the user guide. A test in `skill.rs`
catches the crudest case, a command that does not exist. It reads a command
path after the word `aggountant` in any code, and in an inline code span that
starts with two bare words, so write a path from the top-level command down
(`file-vat status`, not a bare subcommand pair). It does not check flags,
fields or codes; those are the writer's to verify against the built binary.

## Bundling

A skill ships only the files listed for it in `crates/cli/src/skill.rs`, with
`SKILL.md` first; a file added here but not listed there never reaches a user.
A skill may link to its own files and to files of the skills it `requires`,
because the installer writes those alongside it; a test checks every relative
link against that. Installing writes and refreshes the listed files and never
deletes anything, so a file renamed or dropped here stays as a stale copy in
existing installs: keep it unlinked and say so in the change.

Every skill reads the shared role, `aggountant-init/SOUL.md`, and requires
`aggountant-init` so that link resolves.

## e-services: operating knowledge for the authorities' web pages

`e-services/` holds how the authorities' and banks' web services behave in
the user's browser, one reference file per service. It is public, general
knowledge; a company's own practice belongs in its `knowledge company`
records and a machine's configuration to its user. Its `SKILL.md` gives the
agent the boundary, the entry format and the freshness rule; the packaging
test in `skill.rs` checks the header lines. What the agent cannot see, and the
maintainer keeps when editing entries:

- **One entry per thing an agent needs to know about a page.** Not a page
  walkthrough, not a session narrative. A new fact about a page that already
  has an entry for something else is a new entry.
- **Synthetic and identifier-free.** No company, person, figures, account or
  business IDs, no URLs carrying an identifier (write `./Retrieve/…`), and
  nothing about extracting cookies, tokens or session identifiers.
- **Knowledge is never authority.** Reject an entry that makes the browser a
  route around aggountant: past a blocker, an unresolved submission or a
  pending approval. A return the user files by hand may be filled on their
  explicit request, but the submit and its confirmation stay theirs and the
  result is recorded with `complete-manual`. *Rajapinta* names the command
  where one exists.
- **Dates move only on evidence**: someone saw the behaviour at the page, in a
  contribution or the maintainer's own use. Changed behaviour is a rewritten
  entry with a new *Havaittu*; the id stays, because local company facts and
  contributions refer to it, and git keeps the old text.
- **Age never fails the build.** The test checks form and order of the dates,
  never against today; an entry older than 180 days is a lead the skill tells
  the agent to treat with caution.
- **What to obtain and why stays in the task skill** (`tulorekisteri` keeps the
  report 630 parameters), which links to the entry for how to work the page.
