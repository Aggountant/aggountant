# Round 2: the Pääkirja identity applied to the site

Jari chose direction A, "Pääkirja", on 12.11.2026 and asked to keep its logo
as aggountant's logo. This round puts that identity into `site/` so it can be
seen on real pages. The guide's Markdown in `docs/user/` is unchanged; the
theme is CSS and two small component overrides.

## What changed in `site/`

- `src/styles/ledger.css` (via `customCss`) holds the theme. It overrides
  Starlight's colour variables for light (paper) and dark ("ledger at
  night"). The reading column has faint ruled lines. The header has a heavy
  ink rule below it. The sidebar's edge is the red margin line, and on the
  landing page, which has no sidebar, the margin line is drawn left of the
  column. Headings and body text use a book serif (Iowan Old Style /
  Palatino / Charter / Georgia), the interface (header, sidebar, table of
  contents, buttons, pagination, `<summary>`) uses the system sans, and code
  and figures use monospace. Links are ink with a red underline. Tables are
  ledger pages: a card, an ink rule under the head, dotted row rules and
  tabular figures. The `<details>` command blocks are cards whose left edge
  turns seal green when open. Blockquotes (the guide's example prompts) are
  italic with a red margin bar. Asides and buttons are square.
- `astro.config.mjs` sets Expressive Code to the `github-light` and
  `github-dark-dimmed` themes on paper-coloured backgrounds with square
  frames and no shadow.
- `public/logo.svg` is the logo from prototype A, and `public/favicon.svg` is
  the same mark on paper (it follows the browser's dark mode).
  `src/components/SiteTitle.astro` puts the logo in the header, inlined so
  its ink follows the theme, next to the name in monospace.
- `src/components/ThemeProvider.astro`: on a first visit the site opens in
  light (paper) instead of following the system. The theme menu still offers
  dark, light and auto. To do this, the first visit writes `light` to
  Starlight's own `starlight-theme` localStorage key.
- `src/components/Landing.astro` (fi and en) rebuilds prototype A on
  Starlight's splash template: the promise with "keskustelemalla" in seal
  green, a sealed voucher (the README's €124.00 licence) as the hero image,
  § 1 Miksi (philosophy), § 2 the month as journal rows (Yhdessä / Agentti /
  Sinä), the approval guarantee, § 3 honest status (ready / not yet, linked
  to the status page), § 4 the early-users call, and a footer with links to
  the guide in both languages and to GitHub. The content follows the current
  README and the guide's front page; Esimerkki Oy and the figures are made
  up. Prototype A's small kicker line was dropped because Starlight's hero
  has no place for it.

## Factual fix: approval is per voucher

The month now says that the agent shows each proposed voucher and you approve
it ("yksi kerrallaan" / "one at a time"). Batch approval is only the open
issue `month-batch-approval`. The round-1 prototypes were corrected the same
way: A's step 3, B's "Hyväksy kaikki 38" (now "Hyväksy tosite 47") and its
thread text, and D's figure. The round-1 README records the correction.

## Checks

- `bash scripts/check-user-guide.sh` and `scripts/ci-local.sh` both pass
  (`CARGO_TARGET_DIR` under `/home`).
- No external fonts or CDNs. Headless Chromium recorded no requests outside
  the local site, and no page has horizontal overflow at 390 or 1440 px.

## Screenshots

Headless Chromium (Playwright 1.65) loaded the built site from a local
server. The images are compressed JPEG: phone shots are 390 px wide and cut
at 2600 px, desktop shots were taken at 1440 px and saved at half scale
(720 px wide), cut at 1500 px. `<details>` blocks are opened for the shots.

| File | Page |
| --- | --- |
| `landing-fi-1440.jpg`, `landing-fi-390.jpg` | Landing page, Finnish |
| `landing-en-1440.jpg` | Landing page, English |
| `landing-fi-dark-1440.jpg` | Landing page in the dark theme |
| `guide-fi-1440.jpg`, `guide-fi-390.jpg` | Guide front page (`/guide/`) |
| `close-month-fi-1440.jpg`, `close-month-fi-390.jpg` | Task page `/tasks/close-month/` with the command block open |
| `glossary-fi-1440.jpg`, `glossary-fi-390.jpg` | Glossary `/reference/glossary/` |

The screenshots render with Linux fallback fonts (DejaVu/Liberation). On a
Mac the serif is Iowan Old Style and the monospace SF Mono, which look
finer.

## Open points for Jari

- Light by default: the site ignores the system's dark setting until the
  visitor picks "auto" or "dark" from the menu. Following the system again
  is a one-file revert (`ThemeProvider.astro`).
- The glossary is a long run of bold-led paragraphs. The theme cannot make
  it more scannable without changing the Markdown (for example to a
  definition list), so it was left as it is.
- Code blocks keep Expressive Code's terminal frame (three dots and a copy
  button), restyled to paper. It could be reduced to a plain frame.

The issue stays open: the acceptance item "applied to the site theme and
landing page in fi and en" is now implemented, waiting for Jari's comments
on the live pages.
