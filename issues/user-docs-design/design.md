# User documentation design

This is the design for rewriting the user guide in `docs/user/`. It covers who
reads the guide, what they are trying to do, which pages serve that, a template
for each page type, how Finnish and English stay in step, where every part of
the current guide goes, and the format. Three Finnish sample pages sit beside
this file:

- [`naytesivu-aloitus-harjoittele-ensimmainen-kuukausi.md`](naytesivu-aloitus-harjoittele-ensimmainen-kuukausi.md)
  is the getting-started tutorial.
- [`naytesivu-tehtava-sulje-kuukausi.md`](naytesivu-tehtava-sulje-kuukausi.md)
  is a task guide.
- [`naytesivu-selitys-sinetointi-ja-hyvaksynta.md`](naytesivu-selitys-sinetointi-ja-hyvaksynta.md)
  is a concept page.

Every command and figure in the samples was checked against the 0.1.0 binary
in a throwaway home with the fictional Esimerkki Oy (see "How the samples were
checked" at the end).

## What is wrong with the current guide

The guide is accurate and honest about limitations. Its problem is shape. The
nine chapters follow the CLI's command groups (installation, skills, monthly
cycle, VAT, payroll, year end, errors, glossary), so each chapter mixes four
different things: a procedure, an explanation of why the tool behaves as it
does, reference tables (error codes, Vero.fi blockers, home layout) and
warnings about what does not work yet. Someone who wants to close March has to
read chapter 4 from the bank statement onwards, and the step that closes the
month sits between paragraphs about obligations and a warning about rejected
closes. Someone deciding whether to use aggountant at all has no page written
for them. There is no safe place to learn: the first real voucher a new user
seals is permanent.

What already works and should be kept:

- Every task says what the user does, what the agent does and what the user
  approves and why. This is the guide's most valuable habit and becomes part
  of the task template.
- Limitations are stated where the user meets them, with what to do instead.
- One fictional company and synthetic figures throughout.
- Finnish statutory and service names stay Finnish in the English guide.

## Readers

aggountant is used by one trusted person per company, with an AI agent. Three
kinds of people open the guide. The agent itself is not a reader to design for:
its operating manual is the skills, and the guide is for the human who
supervises it.

**1. The owner who keeps their own books** (primary reader). Runs a small
Finnish limited company, is legally responsible for its bookkeeping, knows
roughly what a voucher, VAT and a balance sheet are, and has chosen to do the
books with an agent instead of an accounting firm or a SaaS product. Is
comfortable enough to start a terminal agent such as Claude Code, but does not
want to learn aggountant's commands. Wants to know: what do I say to the agent,
what do I check before approving, is it done, and what do I do when something
looks wrong. Reads a page when a task is due, not front to back.

**2. The bookkeeper** (secondary). Two variants: someone who keeps one
company's books on the owner's behalf (a partner, an employee, a freelance
bookkeeper working inside the owner's setup), and the accounting firm that
receives aggountant's material for the financial statements. The second is a
real reader in year one, because `close-year` cannot yet be used and the
financial statements are made elsewhere. Knows bookkeeping better than the
owner; does not know aggountant. Wants to know: how does this meet
kirjanpitolaki (numbering, audit trail, retention, corrections), what material
do I get and in what form, where are the limits. aggountant does not support
filing on behalf of another company, so the guide must not imply
accounting-firm use.

**3. The evaluator.** Deciding whether to try aggountant: an owner comparing it
with Procountor or an accounting firm, a developer-entrepreneur, a curious
bookkeeper. Arrives from the README. Wants to know in five minutes: what it
does, what it needs from me (an agent, a terminal, a Vero API registration of
my own), what it does not do, how mature it is, and who carries the risk.
Leaves if the answer is buried in installation details.

## What they are trying to do

| When | Owner | Bookkeeper | Evaluator |
|---|---|---|---|
| Before starting | understand the division of work; decide | check that the controls meet kirjanpitolaki | decide whether to try it |
| Day one | install, learn safely, set up the company home, opening balances, describe where receipts come from | review the opening balances against the old system | try the practice month |
| First month | first bank statement, first receipts, first close, first VAT return (by hand); start the Vero API registration because it has several steps | check the first month's report | — |
| Every month | book the month, close it, file VAT, glance at the report | the same, on the owner's behalf | — |
| Now and then | report travel allowances, correct a voucher, renew the certificate, upgrade, restore a backup | the same | — |
| Year end | hand the year's material to the accountant; the statements are made outside aggountant for now | make the financial statements from aggountant's reports and vouchers | — |
| Something went wrong | understand an error, recover an interrupted run, find "missing" vouchers, filing not visible in OmaVero | the same | — |

The month and its VAT return are where users spend most of their time, so task
guides for that cycle are the core of the guide. Day one is where users are
most likely to give up or make a permanent mistake, so it gets a tutorial in a
practice home.

## What to borrow from documentation people like

- **Diátaxis** ([diataxis.fr](https://diataxis.fr/start-here/)): four kinds of
  documentation serving four needs, written differently. A *tutorial* is a
  lesson in which the teacher is responsible for the learner's success; a
  *how-to guide* serves a competent user doing real work and is "written from
  the perspective of the user, not of the machinery"
  ([how-to guides](https://diataxis.fr/how-to-guides/)); *reference* is neutral
  fact; *explanation* is understanding. Borrow the separation. The current guide
  mixes all four on every page, and that is what makes it read like a manual.
- **GitHub Docs content model**
  ([about the content model](https://docs.github.com/en/contributing/style-guide-and-content-model/about-the-content-model)):
  a small fixed set of article types (concept, how-to, reference,
  troubleshooting, quickstart, tutorial), each with its own template; how-to
  titles name the task; a *troubleshooting* type for errors users will meet,
  and *known issues* as a subset for problems users *will* encounter
  ([troubleshooting type](https://docs.github.com/en/contributing/style-guide-and-content-model/troubleshooting-content-type)).
  Borrow the per-type template, task-named titles, and a single known-issues
  page for what is not usable yet.
- **hledger** ([hledger.org](https://hledger.org/),
  [docs](https://hledger.org/doc.html)): a plain-text accounting CLI with a
  quick example on its front page, a gentle book for beginners ("hledger by
  example"), a cookbook of task recipes grouped by subject, and the complete
  manual available from the binary (`hledger help`). Borrow the split between
  a gentle path and task recipes, and leave flag-level detail to `--help`
  instead of copying it into the guide.
- **Beancount** ([docs index](https://beancount.github.io/docs/)): user docs
  open with "Command Line Accounting in Context" and "The Double Entry Counting
  Method" before syntax. Borrow the idea that a reader new to the approach needs
  the *why* of the tool early; for aggountant that is the division of work and
  sealing.
- **The Good Docs Project** ([templates](https://www.thegooddocsproject.dev/template)):
  a core pack of concept, how-to, reference, troubleshooting and tutorial
  templates. It confirms the same five types and that a template per type is
  normal practice for keeping many pages consistent.
- **Procountor** ([tuki](https://www.procountor.fi/tuki/)): a Finnish accounting
  product with separate handbooks for accounting firms and for entrepreneurs.
  aggountant does not need two handbooks, but it confirms that the bookkeeper
  reads differently from the owner; one orientation page for bookkeepers covers
  that.

One thing none of these have and aggountant needs: in aggountant the user does
not run the commands. Every task guide therefore says what the user says, what
the agent does, and what the user checks before approving. Commands are shown
so the user recognises them, folded below the steps.

## Structure and navigation

Folders follow page type, so a writer knows which template applies. The front
page follows the reader's calendar, so a reader finds a task by when it is due.
Each language has the same tree.

```
docs/user/
  AGENTS.md
  fi/                                  en/ (same paths)
    README.md                          front page and the full page list
    start/
      is-it-for-you.md                 Onko aggountant sinulle?
      install.md                       Asenna aggountant ja skillit
      practice-month.md                Harjoittele ensimmäinen kuukausi   (sample)
      set-up-company.md                Ota yrityksesi kirjanpito käyttöön
    tasks/
      monthly-routine.md               Kuukauden kirjanpito alusta loppuun
      import-bank-statement.md         Tuo pankin tiliote
      record-receipts.md               Kirjaa kuitit ja ostolaskut
      record-other-entries.md          Kirjaa myyntilasku, palkka tai muu erikoiskirjaus
      monthly-report.md                Katso, miten kuukausi meni
      close-month.md                   Sulje kuukausi                      (sample)
      file-vat-omavero.md              Tee ALV-ilmoitus OmaVerossa
      file-vat-direct.md               Lähetä ALV-ilmoitus suoraan Verohallinnolle
      set-up-vero-api.md               Ota suora ALV-ilmoittaminen käyttöön (and certificate renewal)
      report-travel-allowances.md      Ilmoita matkakorvaukset Tulorekisteriin (setup included)
      correct-voucher.md               Korjaa virheellinen tosite
      back-up-and-restore.md           Varmuuskopioi ja palauta kirjanpito
      upgrade.md                       Päivitä aggountant
      year-end.md                      Valmistele tilinpäätös
    concepts/
      division-of-work.md              Sinä, agentti ja aggountant
      sealing-and-approval.md          Sinetöinti ja hyväksyntä            (sample)
      periods.md                       Kaudet, sulku ja tilikausi
      vat-treatment.md                 Miten ALV-käsittely päätetään
      filings.md                       Viranomaisilmoitukset ja epävarma lopputulos
      company-home.md                  Yrityskoti: mitä tallentuu minne
      for-bookkeepers.md               Kirjanpitäjälle
    troubleshooting.md                 Kun jokin menee vikaan
    reference/
      status.md                        Mitä on valmista ja mitä ei
      error-codes.md                   Virhekoodit
      vero-fi-blockers.md              Vero.fi-valmiuden esteet
      commands.md                      Komennot lyhyesti
      glossary.md                      Sanasto
  practice/                            practice material for the tutorial (shared by both languages)
    README.md
    tiliote-2026-03.xml                synthetic camt.053 of Esimerkki Oy
    lasku-pilvipalvelu-2026-03.pdf
    kuitti-toimistotarvike-2026-03.pdf
```

That is 31 pages per language, against 10 today. The count comes from tasks,
not from the types: each page answers one question a reader arrives with, and
most are 40–120 lines. The total length stays near today's. What keeps the
larger number maintainable is one rule: **every fact has one home page**. The
error table lives only in `reference/error-codes.md`; the explanation of why a
sealed voucher cannot be edited lives only in `concepts/sealing-and-approval.md`;
the detail of each limitation lives only in `reference/status.md`. Other pages
link there with a sentence. Today the same facts (rejected close, Vero.fi
blockers, what needs approval) are repeated in two or three chapters.

Deliberately left out:

- **No page per command.** `--help` is the command reference; the guide has one
  map from command groups to tasks (`reference/commands.md`), no flags.
- **No separate "known issues" page per area.** One status page.
- **No reference pages for states and home files.** The period and run states
  are in `concepts/periods.md` and the glossary; the home layout table sits at
  the end of `concepts/company-home.md`. Splitting them out would add pages a
  reader has to hop between for little gain.
- **No second handbook for bookkeepers.** One orientation page.

### Front page

Each language's `README.md` is the entry point and the only table of contents.
Sketch:

```markdown
# aggountant — käyttäjän opas

Pidät yrityksesi kirjanpitoa tekoälyagentin kanssa: agentti valmistelee
tositteet tiliotteista ja kuiteista, sinä hyväksyt, ja aggountant tallentaa
kirjanpidon pysyvästi ja tekee ilmoitukset. Tämä opas kertoo, mitä sinä teet,
mitä agentti tekee ja mitä hyväksyt.

aggountant on varhaisessa vaiheessa (0.1). [Mitä on valmista ja mitä ei](reference/status.md).

## Aloita tästä
1. [Onko aggountant sinulle?](start/is-it-for-you.md)
2. [Asenna aggountant ja skillit](start/install.md)
3. [Harjoittele ensimmäinen kuukausi](start/practice-month.md)
4. [Ota yrityksesi kirjanpito käyttöön](start/set-up-company.md)

## Joka kuukausi
- [Kuukauden kirjanpito alusta loppuun](tasks/monthly-routine.md) – tästä linkit kaikkiin vaiheisiin
- [Tuo pankin tiliote](tasks/import-bank-statement.md) · [Kirjaa kuitit](tasks/record-receipts.md) · …
- [Sulje kuukausi](tasks/close-month.md) · [Tee ALV-ilmoitus OmaVerossa](tasks/file-vat-omavero.md) · …

## Kerran tai harvoin
…

## Vuoden lopussa
- [Valmistele tilinpäätös](tasks/year-end.md)

## Kun jokin menee vikaan
- [Kun jokin menee vikaan](troubleshooting.md) · [Virhekoodit](reference/error-codes.md)

## Näin aggountant toimii
- [Sinä, agentti ja aggountant](concepts/division-of-work.md) · [Sinetöinti ja hyväksyntä](concepts/sealing-and-approval.md) · …
- [Kirjanpitäjälle](concepts/for-bookkeepers.md)

## Hakuteos
- [Komennot lyhyesti](reference/commands.md) · [Vero.fi-esteet](reference/vero-fi-blockers.md) · [Sanasto](reference/glossary.md)
```

### Reading paths

- **Evaluator:** README (English, owned by issue `readme-plain-intro`) →
  `start/is-it-for-you.md` → optionally `start/practice-month.md`. The README
  says what aggountant is in one screen and links here; `is-it-for-you` carries
  the detail (what you need, what it costs you in effort, what it does not do,
  who is responsible). The two must not repeat each other beyond one paragraph.
- **New owner:** the four `start/` pages in order, then `tasks/monthly-routine.md`
  every month.
- **Bookkeeper:** `concepts/for-bookkeepers.md` → `concepts/sealing-and-approval.md`
  → `tasks/year-end.md`.
- **Someone in trouble:** `troubleshooting.md` by symptom, or
  `reference/error-codes.md` by the code on screen.

Every page ends with a short "Katso myös" of at most three links, so no page is
a dead end.

## Page-type templates

Five types plus the front page. The templates are written in Finnish because
`fi/` is the original; `en/` uses the same headings in English. Headings in
square brackets are optional.

### Aloitus (tutorial)

Purpose: a newcomer learns by doing something meaningful in a place where
nothing can go wrong. Only `start/practice-month.md` is a full tutorial;
`install.md` and `set-up-company.md` follow the task template but sit on the
same path.

Rules:

- The teacher carries the responsibility. Every step says what the reader will
  see, so they know they are on track. No choices along the way: one path.
- Works in a practice home with the bundled practice material, never in the
  real books, and says so at the top. Learning sealing and closing is the point,
  and both are permanent.
- Explains only what the step needs and links to concept pages for the rest.
- Ends with what the reader can now do and where real work starts.

```markdown
# <Tee jotain konkreettista>

<Kaksi tai kolme virkettä: mitä teet, mitä opit, kauanko se kestää.>
<Missä harjoitellaan ja miksi se on turvallista.>

## Ennen kuin aloitat
- <mitä pitää olla valmiina, linkki asennukseen>

## 1. <Vaihe>
<Mitä sanot agentille (lainauksena).>
<Mitä agentti tekee ja mitä näet. Tarkka tuloste, jos se on vakaa.>
<Mitä opit tästä: yksi tai kaksi virkettä, linkki selitykseen.>

## 2. …

## Mitä opit
- <taito tai käsite>

## Seuraavaksi
<Linkki oikean kirjanpidon käyttöönottoon.>
```

### Tehtävä (how-to)

Purpose: a reader who knows what they want gets it done correctly. Most pages
are of this type.

Rules:

- The title is the task in the imperative, in the user's words: "Sulje
  kuukausi", not "Kauden sulkuajo" or "close-period".
- One task per page. A page that needs "jos haluat X, …, jos haluat Y, …" is two
  pages.
- The core block is **Sinä / agentti / sinä hyväksyt**: what to ask, what the
  agent does on its own, and what the user checks before each approval, with
  the reason.
- Commands go in a folded `<details>` block below the steps. The reader does not
  type them; they are there so the reader recognises what the agent ran.
- "Valmis, kun" gives a check the user can do (a state, a balance, a receipt
  number), not a feeling.
- A limitation gets one sentence and a link to `reference/status.md`.
- Short "Jos jokin menee pieleen" with links, no long troubleshooting.

```markdown
# <Tehtävä käskymuodossa>

<Yksi tai kaksi virkettä: milloin tämä tehdään ja mitä siitä seuraa.>

## Ennen kuin aloitat
- <ehto, jonka käyttäjä voi tarkistaa>

## Näin teet sen
1. **Pyydä agenttia.** <esimerkkipyyntö lainauksena>
2. **Agentti** <mitä se tekee itse, ilman lupaa>.
3. **Tarkista ja hyväksy.** <mitä katsot ennen hyväksyntää ja miksi>
4. …

<details>
<summary>Komennot, joita agentti ajaa</summary>

<komennot esimerkkiarvoin, ei kaikkia valitsimia>
</details>

## Valmis, kun
- <tarkistettava lopputila>

## [Rajoitukset]
- <yksi virke ja linkki tilasivulle>

## [Jos jokin menee pieleen]
- <oire> → <linkki vianetsintään tai virhekoodiin>

## Katso myös
- <enintään kolme linkkiä>
```

### Selitys (concept)

Purpose: the reader understands why aggountant behaves as it does, so its
refusals make sense and they can judge the agent's work.

Rules:

- No steps and no commands except as an example of something the reader sees.
- Starts from what the reader notices ("agentti pyytää hyväksyntää ennen
  jokaista tallennusta") and explains the reason behind it.
- States the legal or practical reason where there is one (kirjanpitolaki,
  an authority's rules) without claiming legal advice.
- Says what this means for the user in practice, in a closing section.
- Architecture vocabulary stays out; name a mechanism only when the user sees
  it on screen.

```markdown
# <Käsite>

<Mitä käyttäjä huomaa, ja kysymys, johon sivu vastaa.>

## <Miksi …>
<Selitys arkikielellä, esimerkki Esimerkki Oy:stä.>

## <Mitä siitä seuraa / rajat>

## Mitä tämä tarkoittaa sinulle
- <käytännön seuraus>

## Katso myös
```

### Vianetsintä (troubleshooting)

Purpose: someone who sees something unexpected finds out what happened and
what to do, without making it worse.

Rules:

- One page, organised by **symptom as the user describes it** ("Ilmoitus ei näy
  OmaVerossa", "Tositteet näyttävät kadonneen"), not by cause or command.
- Each symptom: what probably happened, what to check, what to do, and **what
  not to do** where a wrong reaction is irreversible (resending a filing,
  editing the database, rejecting a close).
- Error codes are listed in `reference/error-codes.md`; this page links to them.
- Split into a folder only when the page passes about 300 lines.

```markdown
## <Oire käyttäjän sanoin>

**Mitä todennäköisesti tapahtui.** <…>

**Tarkista.** <…>

**Tee näin.** <…>

**Älä** <peruuttamaton väärä reaktio ja miksi>.
```

### Hakuteos (reference)

Purpose: look up a fact while working.

Rules:

- Neutral, complete for its scope, tables where the content is tabular, no
  narrative.
- Mirrors the thing described: error codes alphabetically, Vero.fi blockers
  grouped by owner (`human` / `agent`), commands by group as in `--help`.
- Never copies flags or input schemas from `--help`; links to the task page
  where the code or command matters.
- `reference/status.md` is the one place where each limitation is described in
  full: what does not work, why, what to do instead, and the issue that tracks
  it. When a limitation is removed, this page and the affected task pages change
  in the same commit as the code.

```markdown
# <Aihe>

<Yksi virke: mitä tästä löytyy ja mistä tieto on peräisin.>

| <avain> | <merkitys> | <mitä tehdään / missä> |
|---|---|---|
```

### Writing rules for every page

These keep the existing rules in `docs/user/AGENTS.md` and add a few:

- Fictional Esimerkki Oy, Business ID 1234567-1, Maija Meikäläinen, made-up
  figures that add up.
- Describe the 0.1 tool as it is. A command that exists but cannot be used for
  real work is described as such.
- Say "sinä" to the reader. Prompts to the agent are examples, not magic words;
  say that once on the front page.
- Agent output is not deterministic. Quote aggountant's own output exactly
  where it is stable (states, error codes, report headings); describe the
  agent's replies in general terms.
- One fact, one home; link instead of repeating.

## Finnish and English in step

Today the two languages pair by chapter number with names in each language. The
new structure has task-named files in folders, so pairing by number no longer
works. Proposal:

1. **Identical paths in both languages.** `fi/tasks/close-month.md` and
   `en/tasks/close-month.md`. Paths use English slugs: the code, issues and
   branch names are English, and ASCII avoids escaping in URLs. The page title
   is in the page's language. (Question 3 below.)
2. **Same headings in the same order**, translated. A reader can compare the
   two pages section by section.
3. **Same commit, every language**, as today. Finnish is written first; the
   English page follows the rules already in `docs/user/AGENTS.md` (natural
   English, Finnish statutory names kept and explained).
4. **A mechanical check in `scripts/ci-local.sh`** (a later issue, not part of
   the rewrite itself): both trees have the same set of files; each pair has the
   same number of headings; every `aggountant <subcommand> … --flag` mentioned
   in either language exists in the binary's `--help`; both languages mention
   the same set of commands, error codes and Vero.fi blocker codes on each page.
   This catches the two drifts that matter (a page or section missing in one
   language, a command renamed in code but not in the guide) without trying to
   judge translation quality. Commands and codes are language-neutral, which is
   what makes the comparison possible.

## Format: Markdown in the repository, kept ready for a site

**Recommendation: stay with plain Markdown in `docs/user/`, read on GitHub, as
the approved `OSS-RELEASE.md` already says (`docs_site: none`).** Shape the files
so that a static site can be added later without rewriting them.

Why now:

- The guide changes in the same commit as the code. A site adds a build and a
  deployment that can lag behind or break, for a project with one user.
- GitHub renders everything the templates use: tables, relative links, folded
  `<details>` blocks, folder `README.md` files as indexes.
- No new tool, no hosting, no domain.

What it costs: no search across the guide without a GitHub login, no sidebar,
no language switcher. The front page as the full table of contents and the
"Katso myös" links make up for most of it at 31 pages.

Keeping the door open costs nothing if the pages follow these rules: one H1
per page, only relative links to `.md` files, no GitHub-only syntax beyond
`<details>`, practice material in its own folder. With those rules
[mdBook](https://rust-lang.github.io/mdBook/) (the generator hledger's site is
built with; it suits a Rust project's toolchain) could render the same files
from a `SUMMARY.md` and publish to GitHub Pages from CI.

**When to revisit:** when aggountant has outside users who ask for search, or
when the guide outgrows a one-page table of contents. Changing to a site is an
`OSS-RELEASE.md` contract change (`/shipshape-init`), not part of this
rewrite.

## Migration map from the current guide

Every section of the current Finnish guide, and the English counterpart in the
same position, goes to a page in the new structure. "Status" means the detail of
the limitation goes to `reference/status.md` and the task page keeps one
sentence.

| Current (fi) | Section | New home |
|---|---|---|
| `README.md` | Johdanto | `README.md` (front page) |
| `README.md` | Sisällys | `README.md`, rebuilt by moment |
| `README.md` | Mitä on valmista ja mitä ei | `reference/status.md`; one line on the front page |
| `01-mika-aggountant-on.md` | Johdanto, työnjako | `concepts/division-of-work.md` |
| 01 | Kenelle | `start/is-it-for-you.md` |
| 01 | Mitä aggountant ei tee | `start/is-it-for-you.md` |
| 01 | Miksi tositetta ei voi muokata | `concepts/sealing-and-approval.md` |
| `02-asennus-ja-yrityskoti.md` | Ohjelman asennus | `start/install.md` |
| 02 | Mikä yrityskoti on (table) | `concepts/company-home.md` (table at the end) |
| 02 | Kodin perustaminen | `start/set-up-company.md` (practice home: `start/practice-month.md`) |
| 02 | Miten ohjelma löytää kodin | `concepts/company-home.md`; the "wrong path creates an empty home" part to `troubleshooting.md` and status |
| 02 | Git ja varmuuskopio | why: `concepts/company-home.md`; how: `tasks/back-up-and-restore.md` |
| 02 | Käyttöönotto aiemmasta järjestelmästä | `start/set-up-company.md` |
| 02 | Tilikausi ja kaudet | `concepts/periods.md`; non-calendar fiscal year to status |
| `03-agentti-ja-skillit.md` | Skillien asennus | `start/install.md`; reinstall after upgrade to `tasks/upgrade.md` |
| 03 | SOUL.md, agentin rooli | `concepts/division-of-work.md` |
| 03 | Mitä agentti tekee itse | `concepts/division-of-work.md` |
| 03 | Mitä sinä hyväksyt (table) | `concepts/sealing-and-approval.md` |
| 03 | Hyväksyntä on sidottu näytettyyn | `concepts/sealing-and-approval.md` |
| 03 | Hyvä hyväksyntäpyyntö | `concepts/sealing-and-approval.md`; one line in each task's approval step |
| 03 | Yrityksen tietovarasto | `concepts/company-home.md` |
| 03 | Kodin AGENTS.md | `start/set-up-company.md` (writing it); `concepts/company-home.md` (what it is) |
| 03 | Salasanat eivät kuulu kotiin | `concepts/company-home.md`; repeated as one line in `set-up-vero-api.md` |
| `04-kuukausikierto.md` | Johdanto, järjestys | `tasks/monthly-routine.md` |
| 04 | Pankin tiliote | `tasks/import-bank-statement.md` |
| 04 | Kuitit ja laskut | `tasks/record-receipts.md`; VAT choices to `concepts/vat-treatment.md` |
| 04 | Erikoiskirjaukset | `tasks/record-other-entries.md` |
| 04 | Poikkeukselliset kuitit ja konsultointi | status; one line in `record-receipts.md` |
| 04 | Puuttuvat kuitit, `hunt-receipts` | `record-receipts.md` (what to do); status (`hunt-receipts`) |
| 04 | Velvoitteet | `concepts/periods.md` (why they block a close); `close-month.md` (check) |
| 04 | Kauden sulku | `tasks/close-month.md` |
| 04 | Rejected close stays `closing` | status; `troubleshooting.md`; warning in `close-month.md` |
| `05-alv-ja-vero-fi.md` | Johdanto, two ways to file | `tasks/file-vat-omavero.md`, `tasks/file-vat-direct.md` |
| 05 | Ilmoitettavat luvut | both VAT task pages (preview step) |
| 05 | Preview text tells to file by hand | status |
| 05 | Vero.fi-käyttöönotto, mitä tarvitset | `tasks/set-up-vero-api.md` |
| 05 | Tunnukset koneellesi | `tasks/set-up-vero-api.md` |
| 05 | Valmiuden tarkistus (blocker table) | `reference/vero-fi-blockers.md` |
| 05 | Käyttöönoton vaiheet | `tasks/set-up-vero-api.md` |
| 05 | Varmenteen uusiminen | `tasks/set-up-vero-api.md`, section "Kun varmenne vaihtuu" |
| 05 | Suora ALV-ilmoitus | `tasks/file-vat-direct.md` |
| 05 | Maksu (`ApprovePayment` never completes) | status; one line in `file-vat-direct.md` |
| 05 | Ilmoitus ei näy OmaVerossa | `troubleshooting.md`; why in `concepts/filings.md` |
| 05 | Epävarma lopputulos | `concepts/filings.md` (why); `troubleshooting.md` (what to do) |
| 05 | Ilmoitus käsin OmaVerossa | `tasks/file-vat-omavero.md` |
| 05 | Kuukausi- ja neljännesvuosikaudet | `concepts/periods.md` |
| `06-palkat-ja-tulorekisteri.md` | Mitä voi tehdä; payroll not usable | status; `tasks/record-other-entries.md` (booking a payroll voucher) |
| 06 | Työntekijän tiedot | `tasks/report-travel-allowances.md` (before you start); plaintext store to status |
| 06 | Matkakorvaukset, käyttöönotto | `tasks/report-travel-allowances.md` |
| 06 | Ilmoituksen teko | `tasks/report-travel-allowances.md` |
| 06 | Epävarma lopputulos ja varapolku | `concepts/filings.md`; `troubleshooting.md` |
| `07-tilinpaatos-ja-raportit.md` | Kuukausiraportti | `tasks/monthly-report.md` |
| 07 | Kausien ja tilikauden tila | `concepts/periods.md`; `reference/commands.md` |
| 07 | Ketjun tarkistus, tositteiden haku | `concepts/sealing-and-approval.md`; `reference/commands.md` |
| 07 | Tilinpäätös, `close-year` not usable | `tasks/year-end.md`; detail to status |
| `08-virhetilanteet.md` | Miten virhe näkyy | `reference/error-codes.md` (intro) |
| 08 | Tavallisimmat virheet | `reference/error-codes.md` |
| 08 | Keskeytynyt ajo | `troubleshooting.md` |
| 08 | Väärin sinetöity tosite | `tasks/correct-voucher.md` |
| 08 | Ketjun tarkistus epäonnistuu | `troubleshooting.md`; restore in `tasks/back-up-and-restore.md` |
| 08 | Tositteet näyttävät kadonneen | `troubleshooting.md` |
| 08 | Kausi jäi suljettavaksi | `troubleshooting.md` |
| 08 | Ohjelman päivitys | `tasks/upgrade.md` |
| 08 | Testiympäristön muuttuja | `troubleshooting.md` and `reference/status.md` |
| `09-sanasto.md` | all | `reference/glossary.md` |
| — | new | `start/is-it-for-you.md` (most of it), `start/practice-month.md`, `concepts/for-bookkeepers.md`, `reference/commands.md`, `docs/user/practice/` |

### Facts to settle before or during the rewrite

Checking the samples against the binary turned up statements in the current
guide or README that the rewrite cannot copy as they are:

1. **Bank row for an already recorded receipt.** Chapter 4 says a bank row that
   pays an already recorded receipt "is not recorded again". `import camt053
   seal` refuses a batch in which any booked row lacks an assignment, so the row
   cannot be left out. The guide has to say what the row is assigned to. The
   usual answer is that the receipt is credited to a liability (ostovelat, or a
   debt to the owner) and the bank row clears it, but that is a bookkeeping
   convention the skill and guide should state, not something to invent in the
   rewrite. Filed from this run as the unlaned intake issue
   `bank-row-for-recorded-receipt`.
2. **Installation.** The README's install region describes prebuilt binaries
   and an installer script from the latest GitHub Release; the repository has
   no release yet, so that path fails today. The guide's build-from-source path
   is the true one. `start/install.md` should describe what works on the day it
   is written, and the README issue (`readme-plain-intro`) should do the same.
3. **Pending bank entries.** The import skips entries that are not yet booked at
   the bank (status `PDNG`). Not in the guide; belongs in
   `import-bank-statement.md`.
4. **`voucher show`.** Without `--json` it prints only the voucher number and
   hash, not the postings. The guide's "you can see a single voucher by number"
   is true for the agent, not for a user reading the terminal. Task pages should
   say "ask the agent to show voucher 12".

## Practice material

The tutorial needs a practice home and material that cannot be confused with
real books. Proposal: `docs/user/practice/` holds a synthetic camt.053 for
Esimerkki Oy for March 2026 (two booked rows, one pending), one purchase
invoice and one receipt as small PDFs, all with fictional counterparties
(Pilvipalvelu Oy, Toimistotarvike Oy, the owner Maija Meikäläinen paying in
share capital) and fictional IBANs. The statement opens at 0,00 € as for a new
company, so the bank balance and the balance sheet agree without opening
balances. The sample tutorial uses exactly the figures the binary produced in
the check below. The test fixture `crates/import/tests/fixtures/holvi-camt053-sample.xml`
has the right shape but names a real company as a counterparty, so the practice
statement should be its own file.

## Decisions that need Jari

1. **Is the practice-home tutorial the right first experience?** It needs the
   practice material folder and keeps it working as the CLI changes.
   *Recommendation: yes.* Sealing and closing are permanent; the only safe way to
   learn them is somewhere disposable, and a 20-minute practice month shows an
   evaluator what aggountant feels like better than any description. A check in
   CI that the practice material still imports would keep it honest (part of the
   check in "Finnish and English in step").
2. **Commands visible or folded?** *Recommendation: folded.* Steps say what to
   ask and what to check; commands sit in a `<details>` block under them. The
   current rule in `docs/user/AGENTS.md` (commands are included so the user
   recognises them) still holds, but they no longer interrupt the steps.
3. **File names: identical English paths in both languages, or names in each
   language?** *Recommendation: identical English paths.* It makes pairing
   obvious and checkable. It reverses the current rule "file names are in the
   chapter's own language", so `docs/user/AGENTS.md` changes with the rewrite.
4. **Markdown on GitHub or a docs site?** *Recommendation: Markdown on GitHub
   now, mdBook-compatible files, revisit when outside users ask for search.*
   This matches the approved contract.
5. **One rewrite or page by page?** *Recommendation: one rewrite per language
   pair, in one or two commits.* Mixing old chapters and new pages would leave
   two tables of contents and duplicated facts for weeks. The migration map makes
   a single rewrite tractable; it is about the size of the current guide plus
   the tutorial.
6. **Should the bookkeeper orientation page exist before an accounting firm has
   used aggountant's material?** *Recommendation: yes, short.* The first year's
   financial statements are made outside aggountant, so an accountant will
   receive this material; one page saves them reverse-engineering it.
7. **Should the guide-vs-binary check be built before the rewrite?**
   *Recommendation: right after the rewrite, as its own issue.* Built first, it
   would be tuned to pages that are about to disappear.

## How the samples were checked

In a throwaway home under a scratch directory, with the 0.1.0 binary built from
this branch: `init` (98 accounts, `fi-general-2026`); `import camt053 parse` of
a March 2026 Esimerkki Oy statement with two booked rows and one pending row
(two transactions parsed); `import camt053 seal` with a domestic purchase of
124,00 € to 7690 and an incoming 2 480,00 € to 1700 ("sealed 2 vouchers");
`artifact attach` and `capture-receipt record --dry-run` for a 31,00 € receipt
to 7700 against 2740, giving 24,70 € + 6,30 € VAT; the sealed record (voucher
#3) and an identical replay (`replayed=true`); `voucher verify` ("chain ok — 3
vouchers"); `report statements` to HTML; `run start close-period` (run
`awaiting_gate`, period `closing`); `file-vat preview` refusing with
`PERIOD_NOT_SEALED` while closing; `run answer … --gate ApproveClose --answer
approve` (run `completed`, period `sealed`); `file-vat preview` giving field
307 = 31,50 € and 308 = −31,50 €, payment 0,00 €; `file-vat complete-manual`
(period `vat_filed true`).

The tutorial's figures come from a second throwaway home with the practice
statement described above (opening balance 0,00 €; Pilvipalvelu Oy −124,00 €
with invoice date 11.3.; share capital +2 500,00 € to 2000; a pending −50,00 €
row skipped): re-importing the same statement created no new vouchers; the
receipt sealed as #3 and replayed; the report showed a month result of
−123,50 €, Pankkitili 2 376,00 € and both balance-sheet totals 2 407,50 €;
after the close, sealing a new receipt into March was refused with
`PERIOD_SEALED` although its dry-run preview still reported valid.
