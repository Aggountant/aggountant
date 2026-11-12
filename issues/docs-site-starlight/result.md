# Result: docs-site-starlight

The guide builds into a Starlight site from `docs/user/{fi,en}/` in place.
`site/guide.mjs` wraps Astro's glob loader (ids and titles), carries one
Sätteri mdast plugin (drops the H1, rewrites relative links, gives the links
validator the page URL) and generates the sidebar from the front pages.
`scripts/check-user-guide.sh` gained a site-build step, so `ci-local.sh` and
the CI `user-guide` job (now with `setup-node`) build the site;
`.github/workflows/pages.yml` deploys `site/dist` with `CNAME` to Pages.

Verified: `npm ci && npm run build` builds 69 pages (33 per language plus the
two landing pages and 404); the language switcher maps every page to its
pair; Pagefind indexes 34 pages in each language and is served by
`npm run preview`; an injected broken link (`../tasks/nope.md`) and broken
anchor (`close-month.md#ei-ole`) fail the build with
starlight-links-validator's report; `scripts/ci-local.sh` passed in full.

## Decisions for later review

- **Astro pinned to 7.4.1, not the latest 7.4.11.** From 7.4.2 Astro refuses
  to run on Node older than 22.23.4, and the host has 22.23.2. 7.4.1 needs
  22.22, which `site/package.json` `engines` records. Bump Astro (and
  `@astrojs/markdown-satteri` to the version it depends on) once the hosts'
  Node is 22.23.4 or newer. Starlight 0.42.10 and starlight-links-validator
  0.27.0 are the latest.
- **No frontmatter in the guide.** Starlight requires a `title`; the loader
  takes it from the page's H1 and the mdast plugin drops the H1 from the body.
  So the sources are unchanged, GitHub shows no frontmatter table, and the H1
  and title cannot disagree.
- **URLs.** Finnish at the root and English under `/en/`, with the guide's
  paths minus `.md`. The front page `README.md` is `/guide/` (`/en/guide/`)
  because `/` is the landing page.
- **Sidebar generated from the Finnish front page**, not written by hand, so
  `README.md` stays the only table of contents. English group names come from
  the English front page's `##` headings in the same position (the gate keeps
  heading structure in step). Pages linked from more than one section (error
  codes) appear in each, as on the front page. The first entry is labelled
  "Oppaan etusivu" / "Guide home", since the front page's H1 is too long for
  a sidebar.
- **Links outside the guide** (`practice/`, `CONTRIBUTING.md`) go to the file
  on GitHub `main`.
- **Landing page** is `site/src/components/Landing.astro` (Starlight splash
  page, both languages): tagline, the philosophy and the monthly cycle in
  three paragraphs taken from the README, buttons to the guide and the
  early-users page, and links to status, "is it for you" and the other
  language. It is the only site text outside `docs/user/`.
- **The site gate is part of `check-user-guide.sh`** (step 3, before the
  practice month) rather than a separate script or CI job; it needs `npm` and
  shows the build log only on failure. `npm ci` runs every time for a
  reproducible install.
- **Pages workflow** runs on pushes to `main` that touch `docs/user/`,
  `site/` or the workflow itself, plus manual dispatch, with the same
  repository guard as CI.

## For the conductor

- `OSS-RELEASE.md` still says `docs_site: none`, and
  `issues/user-docs-design/design.md` decision 4 still says "Markdown on
  GitHub now". Both were outside this run's file set; `OSS-RELEASE.md` should
  become a Starlight `docs_site` (via `/shipshape-init`), and the design page
  can be updated or retired.
- `.github/workflows/ci.yml` is marked as managed by `/shipshape-ci`; the
  `user-guide` job (already hand-added before this change) now has a
  `setup-node` step that a regeneration would drop.
- Enabling Pages (source: GitHub Actions) and the DNS for `aggountant.dev`
  is still to do, and the workflow changes need pushing from a host that can.
