# ADR-10 rethink: what was taken out

ADR-10 was rewritten on 2026-11-19 so that it can be read and decided on. No
decision changed: the bundled `skills/e-services/` skill, the four entry
header lines, the 180-day freshness rule, the path through
`knowledge company` and a public synthetic issue, the boundary that browser
guidance is never a filing channel, and the rejected options with their
reasons all stay. The original is in git history
(`git log -p -- docs/site/adr-10-eservice-knowledge.md`).

## Cut or shortened

- **Authors line:** the list of every source file read is shortened to the
  kinds of sources. It told the decider nothing about the decision.
- **Context examples:** the OmaVero letter, form and browser examples are
  kept but shortened; the full letter example survives as the entry-format
  sample, so the detail was there twice.
- **"What exists" inventory** (how bundling uses `include_str!`, what
  `knowledge list/show` do, the user guide, the contribution path, the
  `computer-use-helpers` proposal): restated facts that live in code and
  other docs. Replaced by the issue's four constraints, which are what the
  options are judged against.
- **Directory tree for `skills/e-services/`:** reduced to the file names in
  the Decision paragraph.
- **"Reference files are created only when the first entry exists, no
  placeholders" and "files are Finnish":** implementation detail; Finnish is
  already the convention of the other skills.
- **Report 630 parameters** (Finnish, empty reception interval, at most 24
  months): already in `vero-fi`; the ADR only needs to say they stay there.
- **How the boundary on helpers is stated** (`helpers/`, run from a checkout,
  "code would be a separate decision"): belongs to `computer-use-helpers`;
  one line remains.
- **`file-vat complete-manual` by name:** replaced by "aggountant's manual-filing
  path", since the command name is in the code and the user guide.
- **Packaging test detail** (*Vahvistettu* not before *Havaittu* nor after the
  build date): implementation of "well-formed dates"; the slice decides it.
- **`eservice-` prefix for local fact ids:** implementation detail. The
  decision is that the local fact names the entry id; how the agent spots
  its own facts is for slice 2.
- **Field list of the issue form and the list of what a draft must not
  contain** (names, business IDs, amounts, letter contents, screenshots,
  identifying URLs): the root `AGENTS.md` publication rule already says it;
  the form itself belongs to slice 2.
- **"Contradicting observations" section:** merged into the accumulation
  steps (local fact wins on that machine; the maintainer replaces the entry,
  git keeps the old text). One rule was cut outright: *when two credible
  recent reports disagree, the entry keeps both, each with its date and
  condition, until one goes away.* It is a reasonable maintainer practice,
  not part of the decision; reinstate it if you want it binding.
- **Consequence "the rules move to `skills/e-services/SKILL.md` and
  `crates/AGENTS.md`, and this page is removed":** restates the ADR life
  cycle in `docs/AGENTS.md`.
- **Simplicity section** (reused, removed, deliberately left out): its content
  is already in the rejected options and the installer consequence. The
  "left out" items were an entry schema in code, a bundled store, a CLI read
  surface, run-time fetching, automatic submission, age-based CI failures and
  placeholder files.
- **Implementation slices** reduced to one Consequences bullet. The detail
  removed, for whoever files the slices:
  - Slice 1 (spinoff): generalise `BundledSkill` in `crates/cli/src/skill.rs`
    to a file list and drop the `SOUL.md` special case; add
    `skills/e-services/` with `SKILL.md`, `references/omavero.md`,
    `references/tulorekisteri.md` and `references/selain.md` seeded from the
    synthetic material; make `skill install vero-fi` include it; link
    `vero-fi`'s portal steps to the references and replace its "ADR-9"
    citation; extend the packaging test with the entry-header check; update
    the user guide's skill list. It collides with `skill-install-pi` (lane
    `cli-canon`) on `skill.rs` and with `skill-cli-contract-tests` on
    `skills/`.
  - Slice 2 (spinoff, after 1): the recording and contribution sections of
    `e-services/SKILL.md`, a one-line pointer from `SOUL.md`, the *E-service
    observation* form under `.github/ISSUE_TEMPLATE/`, a maintainer note in
    `CONTRIBUTING.md`, and a short user-guide section (fi and en).
