# skills/ — the bundled agent skills

Each directory is one skill that `aggountant skill install` writes into the
user's agent. The skills are written in Finnish, for the user's agent, and are
its manual; the user guide in `docs/user/` is for people.

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
- **What to obtain and why stays in the task skill** (`vero-fi` keeps the
  report 630 parameters), which links to the entry for how to work the page.
