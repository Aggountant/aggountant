# Result

`aggountant skill install vero-fi` now installs `e-services` alongside
`aggountant-init`; `aggountant skill install` installs all four skills.

- **Installer.** Each bundled skill has a file list (`SKILL.md` first) and a
  `requires` list, resolved transitively. The `SOUL.md` special case and the
  per-name dependency `match` are gone. Install and refresh write every listed
  file and report each file's action; nothing is deleted.
- **`skills/e-services/`.** A short `SKILL.md` that opens with the
  knowledge-is-not-authority boundary and explains the entry format and the
  180-day freshness rule, and three reference files seeded from ADR-10's
  synthetic material: `omavero.md` (letter download, letter-list ordering,
  form-field input, submit confirmation), `tulorekisteri.md` (ordering and
  downloading report 630) and `selain.md` (pop-ups and multiple downloads must
  be allowed). One entry per thing an agent needs to know about a page; the
  ADR's single letter example was split into download and list ordering.
- **`vero-fi`.** The stale "ADR-9" citation is replaced by a pointer to the
  e-services boundary; the report 630 step keeps its parameters and links to
  `tulorekisteri.md` for working the page; the manual OmaVero route links to
  `omavero.md` and forbids filling a period with an unresolved API
  submission.
- **Rules the code cannot show** are in `skills/AGENTS.md` (new, with the
  `CLAUDE.md` symlink): bundling and links, and the maintainer's rules for
  e-services entries.
- **Tests.** `e_services_entries_have_well_formed_headers` checks every entry's
  id, the four header lines, `Lähde` values and the dates' form and order,
  never against today. `relative_links_resolve_within_what_the_install_ships`
  checks every relative link in every bundled file. `skill_cli` covers
  installing vero-fi and refreshing a changed and a missing reference file.
- **User guide.** The install page (fi, en) lists four skills and says
  `vero-fi` brings `e-services`.

The ADR-10 page stays until slice 2 (`eservice-observation-loop`) lands.

## For the decider

The seed entries' dates are not separate witness dates: the letter entries
use the dates of ADR-10's own example, the form, submit and browser entries
the ADR's date (2026-11-19), and the report 630 entries the date its page
steps entered `vero-fi` (2026-11-18). They are within weeks of the production
observations they come from, so the 180-day rule is not misled, but slice 2's
"a local fact newer than *Vahvistettu* wins" makes the exact dates matter. If
you have the production agent's notes, replacing the dates with the real ones
before a release is the clean fix; otherwise this belongs with the
granularity review (`eservice-knowledge-granularity`).

Review and assessment: `review.md`, `assessment.md`.
