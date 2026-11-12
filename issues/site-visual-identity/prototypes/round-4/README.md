# Round 4: riff on Laatija, the agent's name on the voucher

Jari (12.11.2026) picked round 3's Laatija direction, where the agent's name
sits on the voucher's LAATINUT (prepared by) line. Its weakness was that the
name was too small to notice, and on a phone the voucher was below the
first screen. This round follows the repository's `prototype` skill
("riff Laatija"). It has three variants, each giving a different answer to
"how does the agent's name read clearly at a glance".

Open `index.html` in a browser. It is one self-contained file (inline CSS
and JS, system fonts) showing the Pääkirja header, the hero with the static
title "Hoida firmasi kirjanpito omalla agentillasi.", the voucher, and the
start of § 1 below. One variant is shown at a time at full size. Switch with
the picker at the bottom or with keys `1`–`3` or `←`/`→`. `R` (or ↻)
replays the animation. The choice is kept in the URL (`?v=2`). The picker
is copied verbatim from the skill's `PICKER.md`. `site/` is unchanged.

| # | Variant | Axis | When it's the right choice | Its cost |
| --- | --- | --- | --- | --- |
| 1 | Allekirjoitus | Scale: the LAATINUT and HYVÄKSYNYT lines become a signature block at signature size, the agent's name in 24–30 px typewriter and "Sinä" in italic serif, as two signatures on a paper form | The voucher should stay a calm paper document and the name should read as a signature, not as a feature | The name is at the foot of the voucher, so on a phone it is the last thing on the voucher; the larger block makes the voucher taller |
| 2 | Lipuke | Placement: a dark label clipped to the top of the voucher, "LAATINUT Claude Code", is the first thing the eye meets; "HYVÄKSYNYT Sinä" closes the slip at the bottom | The name must be seen first, including on a phone, where the label sits right under the title | The only dark block on the page, so it competes with the title and the primary button; prepared-by and approved-by are far apart, so the pairing reads less as a signature |
| 3 | Vuorot | Motion: the agent's name is typed onto the LAATINUT line, then "HYVÄKSYNYT Sinä · 2.4.2026" appears, then the SINETÖITY stamp lands; it holds, clears, and the next agent repeats the cycle | The division of work should read as a sequence: the agent prepares, you approve, the voucher is sealed | It is motion again, the same family as the rotor Jari found silly, though slower and inside the example; the approval line and stamp are hidden for part of each cycle |

The names cycle through Claude Code, Codex, OpenCode, Pi and Gemini CLI.
Allekirjoitus and Lipuke swap the name with a 200 ms ease-out fade every
3.2 s. Vuorot types each name, and its approval line and stamp enter with
220 ms ease-out (the stamp scales from 1.25 to 1). No name is coloured.

## Shared behaviour and checks

- **Phone:** at 390 px the voucher moves up between the title and the
  tagline in every variant, so the agent's name is on the first screen.
  The bottom of the name measured at 489 (Allekirjoitus), 203 (Lipuke) and
  465 (Vuorot) px of an 844 px screen. On desktop the voucher stays beside
  the title as on the live site.
- **No layout jump:** all names share one grid cell, so the slot is as wide
  as the longest name. Vuorot's hidden lines keep their space. Headless
  Chromium sampled the layout every 0.5 s for 12 s per variant at 390 and
  1440 px, with and without reduced motion, and positions never changed.
- **Reduced motion:** nothing cycles. Allekirjoitus and Lipuke show Claude
  Code. Vuorot shows the final state (Claude Code, approved, sealed), which
  is also what the markup shows without JavaScript.
- **Screen readers** hear one sentence: "agentti, esimerkiksi Claude Code,
  Codex, OpenCode, Pi tai Gemini CLI".
- There were no console errors and no network requests. The only links are
  the guide (fi/en), the early-users page and GitHub. Picker keys, wrap-around
  and the `?v=` parameter were checked.

Applying the chosen variant to `site/src/components/Landing.astro` (fi and
en) is a later round. The phone order (voucher above the tagline) carries
over whichever variant wins. The issue stays open.
