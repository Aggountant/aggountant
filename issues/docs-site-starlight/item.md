---
created: 2026-11-12
updated: 2026-11-12
type: feature
reporter: jari
status: open
priority: normal
related: ['@user-docs-design']
preparation: ready
lane: user-guide
lane_seq: 4
collision: [docs/user, .github/workflows]
---

# Publish the user guide at aggountant.dev with Astro Starlight

## Description

Publish the user guide as a website at https://aggountant.dev with Astro
Starlight (an early user recommended it; it fits better than mdBook because
it has a Finnish/English language switcher, search and a sidebar built in).
This replaces design decision 4 in `issues/user-docs-design/design.md`
("Markdown on GitHub now") with a site now (Jari 12.11.2026).

The Markdown under `docs/user/{fi,en}/` stays the single source: GitHub keeps
rendering it, and the site builds from the same files (no second copy). The
site root is a short landing page in the README's voice (what aggountant does,
the philosophy, links to the guide in both languages and to the early-users
page); the guide follows the existing tree and the front page's calendar
order. Finnish is the default locale, English at `/en/`.

Deploy with GitHub Pages from a workflow on pushes to `main` (custom domain
`aggountant.dev`, `CNAME` in the build output). The site build also runs in
pull-request CI so a broken link or page fails before merge. Keep the existing
`scripts/check-user-guide.sh` gate working; extend it rather than duplicate
it.

## Acceptance Criteria

- [ ] `npm ci && npm run build` in the site directory produces the site from `docs/user/` without copying the Markdown; fi default, en switchable, search works locally.
- [ ] A Pages deploy workflow and a build step in CI; the site build fails on a broken internal link.
- [ ] Links between guide pages work both on GitHub and on the site.
- [ ] README links to https://aggountant.dev; `docs/user/AGENTS.md` describes how the site is built.
