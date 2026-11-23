# Writing the user guide so a person wrote it

Much of the guide was written by agents, and it reads that way: dense,
defensive, padded with denials and restatements. This file collects what makes
a page read as written by a person, and it grows as pages are cleaned up. The
rules in `AGENTS.md` (structure, one fact one home, Finnish and English in
step, format) still hold; this file is about the prose.

The reader is a small-company owner who wants to get a task done or
understand why aggountant behaves as it does. They read a page once, looking
for what to do and what to check. Everything that does not help them with that
is noise.

## What to keep

- Every fact, command, flag, error code and quoted output that is true. A
  cleanup does not change what the guide says the tool does. If a statement
  looks wrong, check it against the code and `aggountant <command> --help`;
  fix it when certain, otherwise leave it and report it.
- The heading structure the gate checks: the same heading levels in the same
  order in both languages. A heading's text may change, but its anchor changes
  with it, so fix the links that point to it.
- Instructions that a user actually needs, even when they are plain. Plain is
  the correct voice for a guide.

## What to take out

- **Steam.** Denials of things nobody suggested ("aggountant ei koskaan …"
  where nothing implied it would; but "aggountant ei sisällä tekoälyä" stays,
  because a reader of an AI bookkeeping tool does assume the opposite),
  restatements of the previous sentence,
  explanations of what any reader already knows, summaries of what the page
  just said, and announcements of what the page is about to say.
- **Defensive detail.** Edge cases and internal mechanisms the user never meets
  on screen. If it matters to a few users, one sentence and a link to its home
  page. A reason only a developer cares about ("jotta aiemmat tavat toimivat
  edelleen") is defensive detail, not a fact to keep; the fact it explains
  stays.
- **AI patterns** (see the humanizer list): em and en dashes used as
  punctuation (aggountant's quoted output keeps its own), lists whose items
  start with a bold label and a colon or a full stop,
  forced groups of three, "ei vain X vaan Y" / "not just X but Y", inflated
  significance ("keskeinen", "olennainen", "varmistaa, että"), and generic
  closing lines.
- **Nominal Finnish.** "suorittaa tarkistuksen" → "tarkistaa", "tehdä
  hyväksyntä" → "hyväksyä", "mahdollistaa" → say who can do what. Prefer verbs
  and an actor: "agentti tarkistaa", "sinä hyväksyt".
- **Bold for emphasis.** Keep bold for the few words a scanning reader must
  not miss, not for every term. The bold lead-in of a numbered step
  is the how-to template's and stays, as a clause that says who does what
  ("3. **Agentti lukee tiliotteen**, …", "**Tarkista kuukausi.**"), not a
  noun label ("**Tiliote.**"). In a long reference list (limitations, error
  situations) one short bold term per item may stay as a scanning aid, never
  a bold sentence. Bold labels elsewhere, such as symptom labels, actor names
  opening a paragraph or a term where it is first defined, go.

## Shape

- Open with what the reader is here for: the task, or the question the page
  answers. No warm-up sentence after a heading.
- Shorter is usually better, but there is no target. A page with a stale
  fact can grow when the missing route is added. A page with real steam
  can lose a third; a task or overview page that already follows its template
  may barely shrink, and then the gain is order and shape. Never cut a fact to
  shorten a page. Do not compress steps into a paragraph if the user follows
  them one by one.
- Vary sentence length. A guide can have a short sentence; it should not have
  a run of them for effect.

## Lessons from cleaned pages

*Added by the conductor as pages are cleaned up. Each entry says what was
found and what to do instead.*

Wave 1 (division-of-work, close-month, is-it-for-you):

- **The page announces itself.** "Tämä sivu kertoo, …", "Tämä sivu auttaa
  päättämään, … Lukeminen vie viisi minuuttia." Cut it and open with the
  substance, for example the one-sentence definition from further down.
- **The heading restated as the first sentence.** Heading "Agentti toimii
  kirjanpitoa tuntevan työtoverin tavoin", body "… agentti toimii kuin
  kirjanpitoa tunteva työtoveri." Go straight to what follows from it.
- **Replacing a bold-label list.** Start each item with what the reader or
  the actor does ("Hankit ja käynnistät agentin itse."), or turn a short list
  of assumptions into a paragraph. If an item's first sentence is already the
  point ("Se ei maksa mitään."), dropping the bold is enough.
- **An announced count.** "Ohjelma olettaa kolme asiaa:" followed by three
  items. Drop the count.
- **Lists of -minen nouns.** "tilikartan lukeminen / tositteen esikatselu"
  under "Agentti ajaa ilman lupaa …" → "Se lukee tilikartan / esikatselee
  tositteen". Lists are sentences too.
- **The same fact or assurance in several sections.** Concept and overview
  pages restate the central promise (approval before every permanent act)
  section by section. Look across sections, keep it once where it is
  explained, and keep each other fact once on the page too.
- **Aphoristic balance and empty premises.** "Kun agentti ehdottaa ja
  aggountant tarkistaa, kumpikin tekee sitä, missä se on luotettava." →
  "Siksi agentti ehdottaa ja aggountant tarkistaa." "aggountant on työkalu:
  se laskee …" → "aggountant laskee …".
- **Normal path first, exceptions after.** A step that wedges edge cases
  between what to check and what to say makes the reader read past
  exceptions to reach the action. Order: check, the prompt, the alternative
  (reject), then the edge cases.
- **`<details>` command notes.** One short paragraph per command, in the order
  the commands are listed in the code block, not one wall.
- **Links.** Put a link on the words it explains, not in a trailing
  parenthesis. When the link goes to a section, the link text names the
  section ("Ajo keskeytyi"), not the page.
- **Hedged negatives and obvious purpose clauses.** "Agentti ei aloita sulkua,
  ennen kuin …" → "aloittaa sulun vasta, kun …"; "…, jotta hyväksyt sen,
  mitä …" → "…, ja hyväksyt sen, mitä …". "eikä agentin kuulu keskeyttää" →
  "joten agentti ei keskeytä".
- **Patch seams.** Hard line breaks in mid-sentence and over-long lines show
  where agents patched sentences. Rewrap a paragraph when you touch it.
- **Internal file names.** Name a file only when the user opens it or sees
  it on screen; "yhteinen roolikuvaus" is enough without `SOUL.md`.
- **Version numbers in prose go stale.** Say "varhainen 0.x-versio" in
  running text; `reference/status.md` is the one place that names the
  version.
- **"Katso myös" has at most three links** (`AGENTS.md`). Check it; drop the
  one the body already links.

Wave 2 (front page, install, status, early-users, monthly-routine,
sealing-and-approval):

- **"X ei ole Y. Se on Z."** and its cousins: "päivien, ei viikkojen",
  "esimerkkejä, eivät taikasanoja", "… koottuina, ei kuitti kerrallaan",
  "Sinetöinti ei ole aggountantin oma keksintö. Kirjanpitolaki …". Say the one
  true thing: "Hyväksyntä koskee täsmälleen sitä sisältöä, jonka agentti
  näytti", "Vaatimus tulee kirjanpitolaista".
- **A slogan label and its restatement.** "**Vastaamme nopeasti.** Kysymykset
  käsitellään yleensä päivien …" → "Vastaamme kysymyksiin yleensä muutaman
  päivän sisällä." Keep the body, drop the slogan.
- **Joining a former label to its body.** Once the bold goes, label and body
  are two disconnected sentences; join them with "koska" or "joten" when the
  second is the reason. A recurring inline label ("Tee näin:") becomes a plain
  imperative.
- **Warm-up clauses.** "Ennen kuin agentti voi pitää kirjanpitoa, koneellesi
  asennetaan …" → "Koneellesi asennetaan …". "Koska <the previous section's
  conclusion>, …" opening a section is the same move.
- **A paragraph that ends by restating its first sentence**, a linking
  sentence that only announces the next ("Sama koskee kokonaisia
  kuukausia."), a grand closing line ("aggountant tekee siitä teknisesti
  varman.") and a closing "what this means for you" section that recaps:
  cut. The concept template's closing section holds only what the reader
  does, not a summary.
- **Two rhetorical questions in a row** ("Tuletko toisesta ohjelmasta? …
  Haluatko mukaan? …") is the marketing pattern; write plain conditionals.
- **Routing pages** (monthly routine, front page): put the link on the step's
  own words (`kirjaa kuitit, jokaisen [omana tositteenaan](…)`), not
  `: [Page title](…)` after a colon. A page link may sit on a plain phrase.
- **Hypothetical conditional** where the present works: "Jos joku muuttaisi
  …, ketju ei enää täsmäisi" → "Jos tositetta muutetaan …, ketju ei enää
  täsmää".
- **A prompt that encodes the implementation** ("Asenna aggountant
  lähdekoodista GitHubista.") goes stale; "Asenna aggountant." survives.
- **"Valmis, kun" holds only checks.** A stray fact parked there moves to the
  step it belongs to.
- **Example output with a version number** goes stale like prose does;
  describe it ("näyttää versionumeron").
- **Mixed small things:** split sentences joined by a semicolon, make a
  two-item bullet list prose, show a link as words and never as a path
  (`../en/`), drop dashes from titles too ("aggountantin käyttäjän opas"),
  and check `en/` terms against `AGENTS.md` (`erikoiskirjaus` is "manual
  entry").
- **Invitation and about pages** keep their motive paragraph; it is content,
  not warm-up. Move it after what the reader can do.

Wave 3 (periods, filings, vat-treatment, company-home, for-bookkeepers,
record-receipts, import-bank-statement, correct-voucher, upgrade). Most
findings repeated earlier lessons; these were new or needed a ruling:

- **Error and limitation lists.** An error item opens with the code in
  backticks, not bold, followed by a plain sentence: "`CAPTURE_PREPARATION_CHANGED`:
  jokin muuttui …". One code per item. Other reference items may keep one
  short bold term.
- **A real warning may stay bold.** One sentence on a page that warns of data
  loss or leaked secrets ("git ei ole varmuuskopio") may be bold. Everything
  else is not a warning.
- **Name who acts, and whether the program enforces it.** An agentless
  passive hides whether the program refuses something or the agent is
  expected to do it. "Kuitti kirjataan …" → "Agentti kirjaa …" or "Ohjelma
  hylkää …".
- **One name per thing.** "maksutili", "vastatili" and "pankkitili" for the
  same role in one step is synonym cycling; pick the name the output uses.
- **Content in the wrong section.** An instruction under "Rajoitukset", an
  agent's action under "Ennen kuin aloitat", a conditional step ("Jos jokin
  ajo oli kuitenkin …") among the steps: move each where the reader looks for
  it, the last to "Jos jokin menee pieleen".
- **A step or bullet that carries a whole contract** (twenty lines of field
  names and conditions) is split: what the user sees and decides stays in
  the step, the command detail goes to the `<details>` notes.
- **Prose notation.** Write states in words ("`open`, sitten `closing` ja
  lopuksi `sealed`"), not arrows. In `en/`, dates and percentages use English
  notation (1 July 2026 to 30 June 2027, 25.5%).
- **Intensifiers and cross-page comparisons** ("Kirjanpitolaki edellyttää
  juuri tätä", "kuten kuittien kohdalla") add nothing unless the comparison is
  the point.

