# docs/user/ — the user guide

This directory is the user guide: how a person runs their own company's
bookkeeping with aggountant and an AI agent. `fi/` is the Finnish original
and `en/` its English counterpart; both describe the same tool with the same
facts. `practice/` holds the synthetic material for the practice-month
tutorial and is shared by both languages. The guide is read in two places from
the same files: on GitHub as plain Markdown, and as the website
https://aggountant.dev, which `site/` builds from these files in place (see
"The site" below). It is not a glasspad space.

The readers are, in order: the owner who keeps their own company's books
(primary), a bookkeeper who keeps them on the owner's behalf or receives the
material for the financial statements, and someone deciding whether to try
aggountant. The agent is not a reader; its manual is the skills.

## The guide changes with the code

The guide is a permanent document and describes the tool as it is now. When a
change alters a command, a flag, an input field, an error code the guide names,
or the behaviour a page describes, the same commit updates the guide in every
language that exists, and `practice/` if the practice month is affected. A
guide that lags one release behind teaches users commands that fail, and they
have no way to tell which half is stale.

Before changing a statement, check it against the code and
`aggountant <command> --help`. Describe only what exists. Where a step is still
manual, or a command exists but cannot yet be used for real work (for example
because no live connection is behind it), say so plainly instead of leaving it
out: the user needs to know what they still do themselves.

## Structure

Folders follow page type, so a writer knows which template applies; the front
page follows the reader's calendar, so a reader finds a task by when it is due.

| Path | Page type | Holds |
|---|---|---|
| `README.md` | front page | the only table of contents, grouped by moment: aloita, joka kuukausi, harvoin, vuoden lopussa, kun jokin menee vikaan, näin aggountant toimii, hakuteos |
| `start/` | tutorial and getting-started tasks | `is-it-for-you`, `install`, `practice-month` (the one full tutorial, run in a practice home), `set-up-company` |
| `tasks/` | how-to | one task per page, titled with the task in the imperative |
| `concepts/` | explanation | why aggountant behaves as it does; `for-bookkeepers` is the one orientation page for bookkeepers |
| `troubleshooting.md` | troubleshooting | one page organised by symptom; split into a folder only past about 300 lines |
| `reference/` | reference | `status`, `error-codes`, `vero-fi-blockers`, `commands`, `glossary` |

The templates for the five page types are in
[`AGENTS-TEMPLATES.md`](AGENTS-TEMPLATES.md). A new page takes the template of
its folder and goes onto the front page in the same commit. Every page ends
with "Katso myös" of at most three links, so no page is a dead end.

There is no page per command (`--help` is the command reference;
`reference/commands.md` maps command groups to tasks, without flags), no
separate known-issues page per area, and no second handbook for bookkeepers.

## One fact, one home

Every fact has one home page; other pages link to it with a sentence instead
of repeating it. The fixed homes:

- each limitation in full (what does not work, why, what to do instead, the
  issue that tracks it): `reference/status.md`. A task page gives the
  limitation one sentence and links there. When a limitation is removed,
  `status.md` and the affected task pages change in the same commit as the
  code.
- error codes: `reference/error-codes.md`; Vero.fi readiness blockers and
  warnings: `reference/vero-fi-blockers.md`.
- why a sealed voucher cannot be edited and what needs approval:
  `concepts/sealing-and-approval.md`; period and run states:
  `concepts/periods.md`; the company-home layout and home resolution:
  `concepts/company-home.md`.
- terms a user actually meets: `reference/glossary.md`.

## Finnish and English in step

Both languages have the same file paths (English slugs: `fi/tasks/close-month.md`
and `en/tasks/close-month.md`), and each pair has the same headings in the same
order, translated; the page title is in the page's language. A new page, or a
change to a fact, a command or a limitation, goes into every language in the
same commit. Finnish is written first.

## Format

Plain Markdown that renders the same on GitHub and on the site: no
frontmatter, one H1 per page (the site uses it as the page title), only
relative links to `.md` files, anchors that are the heading's own slug, and no
GitHub-only syntax beyond `<details>`. Write links for GitHub; the site
rewrites them when it builds.

## The site

`site/` is an Astro Starlight project that reads `docs/user/fi/` and
`docs/user/en/` where they are; nothing is copied, so a guide edit needs no
site change. `site/guide.mjs` holds everything that adapts the guide to the
site:

- page URLs: Finnish is the root locale and English is under `/en/`; the path
  minus `.md` is the URL (`fi/tasks/close-month.md` is `/tasks/close-month/`,
  `en/…` is `/en/tasks/close-month/`), and `README.md` is `/guide/`, because
  `/` is the landing page;
- the page title is the H1, which is dropped from the body so it is not shown
  twice;
- relative links are rewritten to site URLs at build time; a link outside
  `fi/` and `en/` (the practice material, `CONTRIBUTING.md`) goes to the file
  on GitHub;
- the sidebar is generated from the Finnish front page: one group per `##`
  section in its order, with the pages that section links to. The group names
  in English come from the English front page's headings in the same position.
  A page appears in the sidebar by being linked from the front page, which
  every page already must be.

The landing page at `/` and `/en/` is `site/src/components/Landing.astro`, a
short front door in the README's voice that points into the guide; it is the
only site text that is not in `docs/user/`, so keep it to what the README
already says. Build locally with `npm ci && npm run build` in `site/` (Node.js
22.22 or newer); `npm run preview` serves the result with search working.
Versions are pinned in `site/package.json` and `site/package-lock.json`.
`.github/workflows/pages.yml` publishes `site/dist` to GitHub Pages on every
push to `main` that touches the guide or the site; `site/public/CNAME` sets
the domain.

## Writing

Write to the owner as "sinä", plain Finnish bookkeeping language, and call the
assistant "agentti". For each task say what the user asks, what the agent does
on its own, and what the user checks before each approval and why. Prompts to
the agent are examples, not magic words. Agent output varies: quote
aggountant's own output exactly where it is stable (states, error codes,
`sealed …` lines, report headings) and describe the agent's replies in general
terms. Commands sit in a folded `<details>` block under the steps, so the user
recognises what the agent ran without the steps being interrupted. Name
internal mechanisms only when the user meets them on screen (an error code, a
gate name); architecture vocabulary stays out.

Natural English in `en/`, not a literal translation: Finnish statutory and
service names (ALV, OmaVero, Tulorekisteri, tilinpäätös, Y-tunnus, Ilmoitin.fi)
and the Finnish account names of the chart stay in Finnish with an English
explanation where they first appear, because that is what the user sees in the
authorities' services and in aggountant's output. Account names, VAT return
field names and the Finnish words of the report are quoted as they appear,
with the English in parentheses: "1910 Pankkitili (bank account)". The tax
itself is "VAT" in English prose; "ALV" is explained on the front page and in
the glossary. Figures and dates use English notation (€2,376.00, 20 March);
aggountant's own output is quoted exactly as it prints. In `en/` the owner is
"you", the assistant "the agent", and `erikoiskirjaus` is a "manual entry".

## Examples, practice material and privacy

Every example uses a fictional company and synthetic figures: Esimerkki Oy,
Business ID 1234567-1, contact Maija Meikäläinen, fictional IBANs and amounts
that add up. Never use a real company's data, figures, counterparties or
personal data, and no real machine names or home-directory paths. Fictional
counterparties carry no business ID, since a made-up one with a valid check
digit can belong to a real company. The repository is public.

`practice/` stays synthetic and in text formats (camt XML, receipts as text)
so the publication scanner can read it. `practice/run-practice-month.sh` runs
the practice month against a binary in a throwaway home and checks the figures
`start/practice-month.md` quotes; when a command, input shape or output the
tutorial uses changes, run it and update the script, the material and the page
together.

## The gate

`scripts/check-user-guide.sh` runs in `scripts/ci-local.sh` and hosted CI. It
fails when a `fi/` page has no `en/` page at the same path or the other way
round, when a pair's heading levels differ in number or order, when a relative
link or its `#anchor` does not resolve, when the site does not build or a link
or `#anchor` does not resolve on the site (starlight-links-validator checks
the built pages), or when the practice month fails against the freshly built
binary. The site build needs `npm`. It does not check wording, translated
heading text, or external URLs.
