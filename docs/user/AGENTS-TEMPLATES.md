# Page-type templates for the user guide

The guide has five page types, one per folder (see [`AGENTS.md`](AGENTS.md)).
The templates are in Finnish because `fi/` is the original; `en/` uses the same
headings in English. Headings in square brackets are optional. The pages
`fi/start/practice-month.md`, `fi/tasks/close-month.md` and
`fi/concepts/sealing-and-approval.md` are worked examples of the first three.
## Aloitus (tutorial)

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

## Tehtävä (how-to)

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

## Selitys (concept)

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

## Vianetsintä (troubleshooting)

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

## Hakuteos (reference)

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

