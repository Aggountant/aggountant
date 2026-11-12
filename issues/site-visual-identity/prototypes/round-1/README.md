# Front-page prototypes, round 1

Four visual directions for the aggountant.dev front page. All four carry the
same Finnish content, so only the look differs. Open `index.html` to see them
side by side (each at 1440 px and 390 px), or open a direction file on its own.

Every file is self-contained: inline CSS and SVG, system font stacks, no
external fonts, images or scripts. The only URLs are links to the guide on
aggountant.dev (`/guide/`, `/en/guide/`, `/start/early-users/`,
`/reference/status/`) and to the GitHub repository. `index.html` has a
short inline script that scales the desktop frames. The example company
is Esimerkki Oy and every figure is made up.

## Content used in every direction

- Promise: "Pidä osakeyhtiösi kirjanpito keskustelemalla tekoälyagentin
  kanssa."
- Philosophy: nobody should pay for a basic service like bookkeeping in the
  age of agents; the agent does it, you approve. The goal is a small company
  with no accounting firm, every statutory filing included, a few
  quarter-hours a month.
- The month (Esimerkki Oy, March), showing who acts at each step: gather
  together (38 transactions, 21 receipts) → the agent prepares (38 proposals,
  2 questions, e.g. "Oliko 64,50 €:n ravintolalasku asiakastapaaminen?") →
  you approve (the README's €124.00 licence = €98.80 IT expenses + €25.20
  VAT; vouchers 40–77 sealed) → month closed (bank balance €12 480,15
  matches the statement) → VAT filed (€1 342,60 payable, directly to Vero or
  by hand in OmaVero). The rule "Mitään ei tallenneta eikä lähetetä ilman
  hyväksyntääsi" appears next to it.
- Honest status: version 0.1, already keeping a few companies' real books.
  Ready: bank statement import, purchase receipts with VAT, month close and
  reports, the VAT return, travel allowances to Tulorekisteri. Not yet: wages,
  the built-in receipt hunt, the financial statements (tilinpäätös). Links to
  the status page.
- The early-users call, plus links to the guide in Finnish and English and
  to GitHub.

## A · Pääkirja (`a-paakirja.html`)

A calm Nordic ledger. The page is ruled paper with a red margin line.
Headlines use a book serif (Iowan Old Style / Palatino / Charter / Georgia)
and figures use monospace tabular numerals. The hero shows a sealed voucher
(the licence entry with debit and credit columns and a "SINETÖITY" stamp).
The month is a journal: one ruled row per step with a timestamp, a role tag
(Yhdessä / Agentti / Sinä) and the figure in the right-hand column. The
palette is paper beige, ink blue-black, a muted red for the margin and a deep
green for the seal. What it says: this is real bookkeeping, careful and
permanent, and familiar to an accountant. Risk: it can look old-fashioned,
and the agent stays in the background.

## B · Keskustelu (`b-keskustelu.html`)

Conversation first. The hero is a chat window where "Sinä" says "Tehdään
maaliskuu", the agent answers with what it found and one question with
answer chips, and then shows a proposed voucher with a "Hyväksy kaikki 38"
button. The month is a thread with two sides: agent steps on the left, your
steps on the right (dark green), shared steps across the middle; at phone
width it folds into one numbered timeline. Typography is the system sans
throughout, bold and tight in the headings; the palette is a light grey-green
with one deep green and a teal accent. What it says: you do not learn a
bookkeeping program, you talk. It is the most direct picture of the product.
Risk: it sits closest to generic chat-product pages, and the chat could
promise more fluency than the agent shows on a given day.

## C · Toimitus (`c-toimitus.html`)

Bold editorial. A newspaper masthead ("Versio 0.1 · Marraskuu 2026"), the
promise set in a heavy grotesque at up to 148 px with "tekoälyagentin" in
red, a two-column body text with a drop cap under the pull quote
"Kirjanpidosta ei pitäisi joutua maksamaan." The month is five columns with
huge numerals; your decisions (3 and 4) are red and the agent's are black, a
code the legend states. Status leads with a giant "0.1". Black, warm white
and one vermilion; Helvetica/Arial for display, Charter/Georgia for the body.
What it says: an opinionated project with a manifesto. The philosophy reads
loudest here. Risk: it can feel more like a campaign than a calm tool for
the books, and the heavy grotesque depends on Helvetica or Arial being
installed.

## D · Pienyritys (`d-pienyritys.html`)

Warm and friendly, for a small-business owner rather than a developer.
Headings use a rounded font stack (ui-rounded / SF Pro Rounded / Nunito,
falling back to the system sans), on cream with terracotta, forest green and
mustard, with chunky buttons that have offset shadows. An inline illustration
shows a shoebox of March receipts, a laptop with "Tehdään maaliskuu" and
"Hyväksyn ✓" bubbles, and a coffee cup. The philosophy sits in a green block
with a "0 €" coin. The month is five icon cards; the two where you decide
have a terracotta border and shadow. Role badges ("Sinä + agentti", "Agentti",
"Sinä") mark who acts. What it says: approachable, you get back to your own
work. Risk: it reads as less serious about accuracy than A, and the rounded
font only shows on Apple systems or where Nunito is installed.

## Checks

- Screenshots were taken with headless Chromium (Playwright 1.65) at
  390 × 844 and 1440 × 900, full page. Neither width has horizontal
  overflow, and Chromium recorded no network requests other than the local
  files. The PNGs (about 3 MB) are not committed. To regenerate, load each
  file in a headless browser at both widths.
- Rendering on Linux used DejaVu/Liberation fallbacks: Iowan, Charter, SF and
  ui-rounded are not installed there, so A, C and D will look somewhat
  finer on a Mac than in those screenshots.
- `grep` for URLs finds only `https://aggountant.dev/{guide,en/guide,start/early-users,reference/status}/`
  and `https://github.com/aggountant/aggountant`.

## For the next round

Jari picks one direction, or a combination (for example B's chat hero with
A's ledger month), and comments. The next round iterates on that choice. The
content and figures above can be reused as they are. Carrying the result into
`site/` (the Starlight theme and `Landing.astro`, fi and en) is a later round.
