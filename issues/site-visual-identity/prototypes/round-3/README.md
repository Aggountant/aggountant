# Round 3: how the agent's name appears in the hero

The live hero (since commit 38696f8) reads "Hoida firmasi kirjanpito ___",
where a fading, sliding rotor fills the blank with Claude Code, Codex,
OpenCode, Pi and Gemini CLI, in ink with a green underline. Jari's verdict
on 12.11.2026 was that it "näyttää vähän hönöltä" (looks a bit silly), and a
coloured highlight word adds nothing. These prototypes show three other ways
to name the agents. Each is only the Pääkirja hero: title, tagline, buttons
and the sealed voucher, in Finnish. Open `index.html` to see all three at
1440 px and 390 px, or open a file on its own. The JPEGs beside them are
screenshots at those widths, taken at a random point in the animation.

None of the options uses colour for the name. Two type the name instead of
fading it in, so the change looks like filling in a ledger rather than an
advertising slider. Every file is self-contained (inline CSS, system
fonts). The only URLs are the guide, the early-users page and GitHub.

## A · Lomakekenttä (`a-lomakekentta.html`)

The headline has a blank to fill in: "Hoida firmasi kirjanpito
<kenttä>". The field is a fixed-width monospace slot on a dotted line, with
a small "AGENTTI" label underneath like a form field. Each name is typed in
letter by letter with a blinking caret, held for about two seconds, erased
quickly, and the next one is typed. The field is as wide as the longest name,
so nothing reflows. Measured over eight seconds at both widths, the heading
and tagline heights never changed. This keeps the idea Jari asked for (the
agents people already use, named in the promise) but makes it look like
paperwork. The risk is that typing in a headline is still motion, and a
reader may wait for it to finish.

## B · Hiljainen (`b-hiljainen.html`)

No animation. The headline says "Hoida firmasi kirjanpito omalla
agentillasi." Under it, between two thin ink rules like a line on a form,
it says in small monospace: "ESIMERKIKSI Claude Code Codex OpenCode Pi Gemini
CLI tai muu". All five names are visible at once and readable to a screen
reader as plain text, and the headline does not depend on any one of them.
It is the calmest option and needs no script. The risk is that the names
become small print, and the headline alone no longer says "Claude Code" in
large type.

## C · Laatija tositteella (`c-laatija.html`)

The headline is static: "Hoida firmasi kirjanpito omalla agentillasi." The
change moves into the sealed voucher. Below the entries are two signature
lines: "LAATINUT" (prepared by), where the agent's name is typed in and
erased one name after another, and "HYVÄKSYNYT Sinä · 2.4.2026". This shows
the division of work on the voucher itself: any of these agents prepares it,
you approve it, and the voucher is the same. The motion is small and inside
the example, not in the promise. The risk is on phones: the voucher is below
the fold at 390 px, so the agent names are not visible on a phone's first
screen at all (see `c-laatija-390.jpg`).

## Motion and layout

- A and C use a small inline script. Without JavaScript, or with
  `prefers-reduced-motion: reduce`, the first name ("Claude Code") stays
  still and the caret does not blink. B has no motion.
- The visible name is `aria-hidden`. A screen reader reads one sentence
  instead: "omalla agentillasi, esimerkiksi Claude Code, Codex, OpenCode, Pi
  tai Gemini CLI".
- Headless Chromium (Playwright 1.65) checked each file at 390 and 1440 px,
  with and without reduced motion, sampling the layout every 0.5 s for 8 s.
  Heading, tagline and voucher positions stayed constant, no page overflowed
  sideways, and no request left the local file.

## Recommendation

My pick is B. Jari's two complaints are about motion and colour drawing
attention, and B removes both while still naming all five agents. If the
name should still change, C is the more Pääkirja-like way to do it on
desktop: the agent signs the voucher and you approve it, which is exactly
the product's division of work. A is the closest to the current rotor and
the least likely to fix "hönö".

Applying the chosen option to `site/src/components/Landing.astro` (fi and
en) is the next round. The issue stays open.
