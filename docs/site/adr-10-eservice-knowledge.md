# ADR-10 — E-service operating knowledge: a bundled skill, fed through company knowledge and public issues

- **Status:** Accepted 2026-11-21 by Jari Mustonen
- **Date:** 2026-11-19
- **Authors:** technical-decision worker for issue `eservice-knowledge`,
  from the existing skills, the `skill install` and `knowledge` code,
  `SOUL.md` and the user guide. No panel. Decider: Jari Mustonen.
- **Refines:** no ADR page. It restates the earlier decision that browser
  automation is not a filing channel, which `skills/vero-fi/SKILL.md` still
  cites as "ADR-9", a number that now belongs to the date-time ADR. It is
  bound by the "Company knowledge is prose the agent judges" rule in
  `crates/AGENTS.md` and by the publication rule in the root `AGENTS.md`.

---

## Context

The user's agent keeps the books, and many tasks take it into the
authorities' and banks' web services in the user's browser: OmaVero, the
Incomes Register e-service (report 630), the bank's web app, pension
insurers. Working these pages takes know-how that no API document gives. For
example (synthetic, but of the kind a production agent observed):

- An OmaVero letter is downloaded through a `./Retrieve/…` link that
  appears when the letter is opened; it has to be fetched as binary in the
  same session. The letter list reorders after opening, so letters are
  picked by name, not position.
- OmaVero form fields accept a value only through
  `document.execCommand('insertText')`, and every submit asks for a
  separate OK.
- These services open pop-ups and hand out several files at once, so the
  browser must allow both for their domains.

None of this is about one company. Today each company's agent rediscovers it
and writes it into that company's private notes, where it helps nobody else
and goes stale unseen.

The constraints from the issue: the knowledge is public and synthetic; every
item shows where it comes from, which service and page it applies to, and
when it was last seen to work; it never becomes authority to act; and
general service operation, company practice and machine configuration stay
apart.

The options weighed:

1. **A bundled skill for e-services**, a short `SKILL.md` with one reference
   file per service that the agent opens only when it needs it.
2. **A new bundled knowledge kind** served by `aggountant knowledge`:
   structured entries compiled into the binary and readable offline.
3. **User-guide pages only**, linked from the skills.
4. **Fetched at run time** from the public repository.

## Decision

**Option 1.** E-service operating knowledge is a bundled skill,
`skills/e-services/`: a short `SKILL.md` and one reference file per service
(`omavero.md`, `tulorekisteri.md`, a `selain.md` for browser prerequisites,
more as observations arrive). A company agent holds a new observation in
`knowledge company` until it ships; it reaches the product through a
synthetic public issue that the user submits and the maintainer merges into
the reference file; the next binary carries it.

### Why a skill

The agent needs this knowledge exactly when it is about to work a page, and
that is what a skill gives: the description routes the agent in, and it
opens only the one service file it needs. Skills already ship with the
binary, install offline and are versioned with the commands beside them, so
the guidance cannot drift from the binary the user runs.

A knowledge kind (option 2) would add an entry schema, a parser, a bundled
store, read commands and their tests, all to hand prose to an agent that
reads Markdown anyway. The structure it would enforce is a few header lines
a packaging test can check. And `aggountant knowledge` is the company's own
store; putting product-wide content behind it blurs the line between product
and company that the constraints draw.

Guide pages (option 3) are written for people, in two languages, and are
not installed beside the agent. A page that says "call
`execCommand('insertText')`" serves no human reader. The guide gets one
short section on what the skill is for and what contributing an observation
means.

Run-time fetching (option 4) needs the network, ties the agent's guidance to
a branch instead of the binary it runs, and gives a web page the standing of
bundled instructions.

### What lives where

- **What to obtain and why** stays in the task skill. `vero-fi` keeps the
  report 630 parameters, because they decide whether the import is correct,
  and links to the `e-services` reference for how to work the page.
- **How to work a service's pages** is in `e-services`.
- **This company's practice** (which bank account, who signs in, which
  insurer) is in `knowledge company`.
- **Machine configuration** is the user's. `selain.md` says what the
  browser must allow, not how a machine is set up to allow it.
- **Executable helpers** for migrating from other accounting programs stay
  outside this skill, as proposed in `computer-use-helpers`.

### Knowledge is not authority

The skill opens with this boundary, and every entry is written under it. An
entry says how a page behaves; it never authorises an action. Where
aggountant has a command for the task, the entry names it and the agent uses
it: a VAT return goes through `file-vat`, an employer report through
`file-wage`, each with its approval. The browser is never a fallback channel
for a filing aggountant can make.

A form aggountant does not produce, such as a manual return or a correction,
may be filled by the agent on the user's explicit instruction, but the
final submit is the user's act, and the result is recorded through
aggountant's manual-filing path.

Entries never describe extracting cookies, tokens or session identifiers,
and paths are written as patterns (`./Retrieve/…`), never as URLs carrying
an identifier.

### Entry format

An entry is a level-two heading with a stable id, four header lines and
prose:

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

- **Lähde** says whether the behaviour was seen working (`havaittu`), is
  in the service's public documentation (`dokumentoitu`, with a link), or
  both.
- **Havaittu** is when the behaviour was first seen; **Vahvistettu** the
  latest date someone saw it still work. Changed behaviour is a rewritten
  entry with a new *Havaittu*.
- **Rajapinta** names the aggountant command or authority API that does the
  task, or says there is none. It is the line that keeps the browser from
  becoming a channel.
- The id is what local facts, contributions and contradictions refer to,
  so it stays when the text changes.

### Freshness

An entry verified within the last 180 days is the expected behaviour. An
older one is a lead: the agent tries it, expects the page may have changed,
and tells the user it is working from old guidance. 180 days is roughly one
change cycle of these services in the observed material.

A packaging test checks that entries have well-formed headers and dates, but
it does not fail on age: a test that turns red with the calendar would block
unrelated releases, and only someone at the page can verify an entry. Dates
move only on evidence, a contribution confirming the entry or the
maintainer's own use.

### How knowledge accumulates

1. **Hold it locally.** When a page behaves differently from the shipped
   entry, or has no entry, the agent records what works in
   `knowledge company`, naming the entry id, the service and page, and the
   date. Company knowledge already has dated sources, revisions and
   retirement, and the next session on this machine needs the finding
   before any release carries it. A local fact newer than the shipped
   entry's *Vahvistettu* wins on that machine.
2. **Offer it to the product.** The agent tells the user the finding is
   general and drafts a synthetic GitHub issue from an *E-service
   observation* issue form. Publishing is an outward act, so the user reads
   and submits it; the agent does not post on its own.
3. **Review and merge.** The maintainer accepts the report as usual and
   edits the reference file: checks it is synthetic, rejects anything that
   would turn the browser into a filing channel or bypass an approval,
   merges it into the existing entry and sets the dates. Git keeps the old
   text.
4. **Ship and retire.** The next release carries the entry. When the agent
   sees the installed skill now covers its local fact, it retires the fact
   as shipped in that version, so the company store does not keep a second,
   aging copy.

## Consequences

- The installer has to ship skills with more than one file. A per-skill
  file list replaces today's `SOUL.md` special case, so the installer gets
  simpler rather than gaining a path. Installing `vero-fi` also installs
  `e-services`, as it already pulls in `aggountant-init`.
- Agents start from shared, dated guidance instead of rediscovering it, and
  stale guidance shows as stale rather than being silently wrong.
- The loop depends on users submitting issues. If nobody does, entries age
  into leads, which the freshness rule already covers.
- Once the skill is in use, the maintainer watches whether what agents
  record and contribute has the right granularity: one entry per thing an
  agent needs to know about a page, neither whole-page walkthroughs nor
  one-off session detail. The first contributions are reviewed for this
  before the format is considered settled (issue `eservice-knowledge-granularity`).
- Implementation is two slices: first multi-file skills and the seeded
  `e-services` skill (with `vero-fi` linked to it and its stale "ADR-9"
  citation replaced), then the accumulation loop (recording and contribution
  guidance, a pointer from `SOUL.md`, the issue form, a maintainer note and
  the user-guide section).
