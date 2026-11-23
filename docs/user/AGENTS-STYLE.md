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
  where nothing implied it would), restatements of the previous sentence,
  explanations of what any reader already knows, summaries of what the page
  just said, and announcements of what the page is about to say.
- **Defensive detail.** Edge cases and internal mechanisms the user never meets
  on screen. If it matters to a few users, one sentence and a link to its home
  page.
- **AI patterns** (see the humanizer list): em and en dashes used as
  punctuation, lists whose items start with a bold label and a colon,
  forced groups of three, "ei vain X vaan Y" / "not just X but Y", inflated
  significance ("keskeinen", "olennainen", "varmistaa, että"), and generic
  closing lines.
- **Nominal Finnish.** "suorittaa tarkistuksen" → "tarkistaa", "tehdä
  hyväksyntä" → "hyväksyä", "mahdollistaa" → say who can do what. Prefer verbs
  and an actor: "agentti tarkistaa", "sinä hyväksyt".
- **Bold for emphasis.** Keep bold for the few words a scanning reader must
  not miss, not for every term.

## Shape

- Open with what the reader is here for: the task, or the question the page
  answers. No warm-up sentence after a heading.
- Shorter is usually better; a page commonly comes out a third shorter
  without losing a fact. Do not compress steps into a paragraph if the user
  follows them one by one.
- Vary sentence length. A guide can have a short sentence; it should not have
  a run of them for effect.

## Lessons from cleaned pages

*Added by the conductor as pages are cleaned up. Each entry says what was
found and what to do instead.*

- (none yet)
