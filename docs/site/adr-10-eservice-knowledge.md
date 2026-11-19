# ADR-10 — E-service operating knowledge: a bundled skill, fed through company knowledge and public issues

- **Status:** Proposed
- **Date:** 2026-11-19
- **Authors:** technical-decision worker for issue `eservice-knowledge`. The
  skills, the `skill install` code (`crates/cli/src/skill.rs`), the
  `knowledge` command (`crates/cli/src/knowledge.rs`,
  `crates/process/src/knowledge.rs`, `company_knowledge.rs`), `SOUL.md`, the
  user guide and the open issues read. No panel: the existing mechanisms
  settled it. Decider: Jari Mustonen.
- **Refines:** no ADR page. It restates the pre-publication decision that
  browser automation is not a filing channel, which `skills/vero-fi/SKILL.md`
  still cites as "ADR-9" although that number now belongs to the date-time
  ADR and the old page is not in the public history. It is bound by the
  "Company knowledge is prose the agent judges" rule in `crates/AGENTS.md`
  and by the publication rule in the root `AGENTS.md`.

---

## Context

A user's agent keeps the books, and many tasks take it into the
authorities' and banks' web services in the user's browser: OmaVero
(letters, forms, tax-account statements), the Incomes Register e-service
(report 630), the bank's web app, pension insurers. Working these pages takes
know-how that no API document gives, for example (synthetic, but of the kind
a production agent observed):

- OmaVero letters: clicking a letter adds a `./Retrieve/…` link. Fetch it in
  the same session as binary (XHR with
  `overrideMimeType('text/plain; charset=x-user-defined')`) and base64 it.
  Go back with the "Palaa heti" link, not `history.back()`. The list
  reorders after a letter is opened, so pick letters by name, not position.
- OmaVero forms: a field takes a value only through
  `document.execCommand('insertText')`; setting the value and firing events
  is not enough. In the preview the values sit in disabled fields and are
  missing from `innerText`. Table rows are added by clicking an empty cell
  and moving with Tab. Every submit asks for a separate OK.
- The browser: these services open pop-ups and hand out several files at
  once, so the browser must allow pop-ups and multiple downloads for their
  domains.

None of this is about one company. Today each company's agent rediscovers it
and writes it into that company's private notes, where it helps nobody else
and goes stale unseen.

What exists:

- **Bundled skills** (`skills/`), compiled into the binary with
  `include_str!` and written out by `aggountant skill install`. Each skill
  is one `SKILL.md`; the one extra file, `aggountant-init/SOUL.md`, is
  special-cased in the installer. Skills already reference each other by
  relative path (`../aggountant-init/SOUL.md`).
- **`aggountant knowledge`**: `list`/`show` read
  consult-generated documents under `<home>/knowledge/`, written only through
  a review gate; `knowledge company` is the company's fact store (statement,
  prose scope, dated source, revision history, retirement). Both live in the
  company home. `SOUL.md` tells the agent to read company knowledge first and
  to save useful facts there.
- **The user guide** (`docs/user/`, Finnish and English, kept in step) is for
  people.
- **Contribution**: public GitHub issues (`CONTRIBUTING.md`, issue forms in
  `.github/ISSUE_TEMPLATE/`), mirrored into `issues/` when accepted. Public
  material is synthetic.
- The open issue `computer-use-helpers` proposes `helpers/<program>/` in the
  repository for executable helpers that pull data out of *other accounting
  programs* during a migration.

The options weighed:

1. **A bundled skill for e-services** with one reference file per service,
   reached from a short `SKILL.md` (progressive disclosure).
2. **A new bundled knowledge kind** served by `aggountant knowledge`:
   structured entries (service, page, observed and verified dates) compiled
   into the binary and readable offline.
3. **User-guide pages only**, linked from the skills.
4. **Fetched at run time** from the public repository (added here): always
   current, but needs the network, ties the agent's guidance to a branch
   rather than the binary it runs, and gives a web page the standing of
   bundled instructions. Rejected on those grounds without further analysis.

## Decision

**Option 1.** E-service operating knowledge is a bundled skill,
`skills/e-services/`, with a short `SKILL.md` and one reference file per
service under `references/`. A company agent holds a new observation in
`knowledge company` until it ships; it reaches the product through a
synthetic public issue that the user submits and the maintainer merges into
the reference file; the next binary carries it.

### Why a skill and not a knowledge kind or guide pages

The agent needs this knowledge exactly when it is about to work a page, which
is what a skill's progressive disclosure gives: the `SKILL.md` description
routes the agent in, and it opens only the one service file it needs. The
skill mechanism already ships with the binary, installs offline and is
versioned with the commands it sits next to, so the knowledge cannot drift
from the binary the user runs.

A knowledge kind (option 2) would add an entry schema, a parser, a bundled
store beside the home store, CLI read commands and their tests, all to hand
prose to an agent that reads Markdown files anyway. The structure it would
enforce (service, page, dates) is a handful of header lines that a packaging
test can check. `aggountant knowledge` is also, today, the company home's
own store; putting product-wide content behind the same verb blurs exactly
the line the constraints draw between product and company.

Guide pages (option 3) are written for people, must exist in two languages,
and are not installed beside the agent. A page that says "call
`execCommand('insertText')`" serves no human reader. The guide gets one short
section saying what the skill is for and what contributing an observation
means (slice 2), nothing more.

### Layout and boundaries

```
skills/e-services/
  SKILL.md                 rules, freshness, recording and contribution; an index of references
  references/
    omavero.md             OmaVero: letters, forms, tax-account statements
    tulorekisteri.md       Incomes Register e-service: report orders and downloads
    selain.md              browser prerequisites the user's environment must meet
```

A reference file is created when its first entry exists; banks and pension
insurers get files when observations arrive, not empty placeholders. Files
are Finnish, like the other skills.

One rule has one home:

- **What to obtain and why** stays in the task skill. `vero-fi` keeps the
  report 630 parameters (Finnish, empty reception interval, at most 24
  months), because they decide whether the import is correct. Its step links
  to `../e-services/references/tulorekisteri.md` for *how to work the
  page*.
- **How to work a service's pages** is in `e-services`.
- **This company's practice** (which bank account, who signs in, which
  insurer) is in `knowledge company`.
- **Machine configuration** is the user's environment. `selain.md` states
  what the browser must allow and for which services; how a machine enforces
  it (a managed browser policy, a profile setting) is not described.
- **Executable migration helpers** for other accounting programs stay in the
  repository's `helpers/` per `computer-use-helpers`, run from a checkout and
  not shipped. If a recurring operation needs code, that is a separate
  decision.

### Knowledge is not authority

`SKILL.md` opens with the rule, and every entry is written under it:

- An entry says how a page behaves. It never authorises an action; the
  authority comes from the user's instruction and aggountant's own rules.
- Where aggountant has a command for the task, the entry names it and the
  agent uses it. A VAT return goes through `file-vat`, an employer report
  through `file-wage`, each with its approval; the browser is never a
  fallback channel for a filing aggountant can make.
- A form the user files through the portal (a manual return, a correction
  aggountant does not produce) may be filled by the agent on
  the user's explicit instruction. The final submit and its OK are the
  user's act, and the result is recorded through aggountant's own manual
  path (`file-vat complete-manual` and the like).
- No entry describes extracting cookies, tokens or session identifiers, and
  no entry or contribution carries a URL with an identifier in it. Paths are
  written as patterns (`./Retrieve/…`).

### Entry format

An entry is a level-two heading with a stable id, four fixed header lines,
and prose. The header is what a packaging test checks and what the agent
reads to judge freshness:

```markdown
## omavero-kirje-lataus — Kirjeen lataaminen tiedostoksi

- **Palvelu ja sivu:** OmaVero › Kirjeet ja päätökset › kirjeen avaus
- **Lähde:** havaittu (ei dokumentoitu)
- **Havaittu:** 2026-10-02 · **Vahvistettu:** 2026-11-12
- **Rajapinta:** ei; kirjeitä ei saa Vero API:sta

Kirjeen avaaminen lisää sivulle `./Retrieve/…`-linkin. Hae tiedosto samassa
istunnossa binäärinä: XHR, jossa
`overrideMimeType('text/plain; charset=x-user-defined')`, ja muunna vastaus
base64:ksi ennen tallennusta. Palaa listaan "Palaa heti" -linkillä, ei
`history.back()`-kutsulla. Lista järjestyy uudelleen avaamisen jälkeen, joten
valitse kirjeet nimen perusteella, ei sijainnin.
```

- **Lähde** is `havaittu` (seen working) or `dokumentoitu` with a link to
  the service's public documentation, which may be combined (`dokumentoitu
  ja havaittu`).
- **Havaittu** is when the behaviour was first seen; **Vahvistettu** the
  latest date someone saw it still work. A changed behaviour is a rewritten
  entry with a new *Havaittu*, not a new date on the old text.
- **Rajapinta** names the aggountant command or authority API that does the
  task, or says there is none. It is the line that keeps the browser from
  becoming a channel.
- The id is what a local fact, a contribution and a later contradiction
  refer to. It does not change when the text does.

### Freshness rules

- The agent compares *Vahvistettu* with today. An entry verified within 180
  days is the expected behaviour. An older one is a lead: the agent tries it
  but checks the page and expects change, and tells the user when it is
  working from old guidance. 180 days is roughly one change cycle of these
  services in the observed material; it is a judgement, written once in
  `SKILL.md`.
- The packaging test checks that every entry has the header lines and valid
  dates, and that *Vahvistettu* is not before *Havaittu* or after the build
  date. It does not fail on age: a test that turns red with the calendar
  would block unrelated releases, and only someone at the page can verify it.
- Dates are refreshed by evidence only: a contribution confirming an entry
  ("still works, seen on date") or the maintainer's own production use. A
  confirmation is a one-line edit to *Vahvistettu*.

### Accumulation path

1. **Observe and hold locally.** When the agent finds that a page behaves
   differently from the shipped entry, or works a page that has no entry, it
   records the finding in `knowledge company` under an id starting
   `eservice-` (the shipped entry's id after it, when there is one), with
   the service and page in the scope and the date in the source. This needs
   no new mechanism: company knowledge already has dated sources, revisions
   and retirement, `SOUL.md` already tells the agent to read it first, and
   it is the right place for a local override, because the next session on
   this machine needs the finding before any release carries it.
2. **Offer the contribution.** The agent tells the user that the finding is
   general and drafts a GitHub issue from a new issue form,
   *E-service observation*: service and page, the entry id it adds to or
   contradicts, what was tried, what worked, the observed date and the
   aggountant version. The draft is synthetic: no names, business IDs,
   amounts, letter contents, screenshots of real data or identifying URLs.
   Publishing is an outward act, so the user reads and submits it; the agent
   never posts on its own.
3. **Review and merge.** The maintainer (or a maintainer's agent) mirrors an
   accepted report into `issues/` as usual and edits the reference file:
   checks it is synthetic, rewrites or rejects anything that turns the
   browser into a filing channel or bypasses an approval, merges it into the
   existing entry rather than appending a near-duplicate, and sets the
   dates.
4. **Ship and retire the local copy.** The next release carries the entry.
   When the agent sees that the installed skill now covers its local
   `eservice-` fact, it retires that fact with the reason "shipped in
   aggountant <version>", so the company store does not keep a second,
   aging copy.

### Contradicting observations

- An agent whose shipped entry fails tries the page as it now is, holds what
  works as a local fact that names the entry id, finishes the task, and
  offers the contribution. A local fact newer than the shipped entry's
  *Vahvistettu* wins on that machine.
- The maintainer replaces the entry with the newer behaviour when the report
  is specific enough to follow. Git keeps the old text; the entry does not
  carry a history.
- When two recent reports disagree and both are credible (a service rolling
  out a change gradually, a difference between user roles), the entry keeps
  both, each with its own date and the condition under which it holds,
  until one goes away.

## Consequences

- The installer must ship skills with more than one file. That replaces the
  `SOUL.md` special case with a per-skill file list, so it simplifies the
  installer rather than adding a path. `skill install vero-fi` also installs
  `e-services`, as it already pulls in `aggountant-init`.
- Agents start from shared, dated guidance instead of rediscovering it, and
  stale guidance is visible as stale rather than silently wrong.
- The contribution loop depends on users submitting issues. If nobody does,
  the references age into leads, which the freshness rule already covers;
  nothing breaks.
- `skills/vero-fi/SKILL.md`'s "ADR-9" citation is replaced by a reference to
  the rule as stated in `e-services` (slice 1).
- When the slices land, the rules the code cannot show (knowledge is not
  authority, the freshness threshold, the entry header) live in
  `skills/e-services/SKILL.md` and a short `skills/` note in
  `crates/AGENTS.md`, and this page is removed per `docs/AGENTS.md`.

## Simplicity

- **Reused:** the skill bundling and install path, the relative cross-skill
  links, `knowledge company` as the local holding place, the GitHub issue
  intake and `issues/` mirroring, the packaging tests.
- **Removed:** the installer's `SOUL.md` special case (folded into the file
  list).
- **Deliberately left out:** an entry schema in code, a bundled knowledge
  store, a CLI read surface, run-time fetching, an automatic submission of
  observations, age-based CI failures, and placeholder files for services
  nobody has observed yet.

## Implementation slices

1. **Multi-file bundled skills and the `e-services` skill** (spinoff).
   Generalise `BundledSkill` to a file list and drop the `SOUL.md` special
   case; add `skills/e-services/` with `SKILL.md`, `omavero.md`,
   `tulorekisteri.md` and `selain.md` seeded from the synthetic material
   above; have `skill install vero-fi` include it; link `vero-fi`'s portal
   steps to the references and replace its stale "ADR-9" citation; extend
   the packaging test with the entry-header check; update the user guide's
   skill list. Touches `crates/cli/src/skill.rs`, so it collides with
   `skill-install-pi` (lane `cli-canon`) and with `skill-cli-contract-tests`
   on `skills/`.
2. **The accumulation loop** (spinoff, after 1). The recording and
   contribution sections of `e-services/SKILL.md`, a one-line pointer from
   `SOUL.md` to them, the *E-service observation* issue form under
   `.github/ISSUE_TEMPLATE/`, a maintainer note on reviewing such reports in
   `CONTRIBUTING.md`, and a short user-guide section (fi and en) on what
   contributing an observation means and what it never contains.
