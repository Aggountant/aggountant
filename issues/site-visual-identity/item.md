---
created: 2026-11-12
updated: 2026-11-13
type: feature
reporter: jari
status: open
priority: normal
preparation: ready
lane: site-design
lane_seq: 1
collision: [site]
---

# Visual identity for aggountant.dev, starting with front-page prototypes

## Description

aggountant.dev (Astro Starlight, `site/`) uses Starlight's default look and a
plain landing page (`site/src/components/Landing.astro`). It needs a visual
identity that fits the product: Finnish bookkeeping kept with an AI agent,
calm and trustworthy rather than a generic AI startup page, with the
philosophy up front (in the age of AI agents nobody should pay for a basic
service like bookkeeping; the agent does it, you approve).

First deliver front-page prototypes for Jari to choose from: three or four
distinctly different visual directions (typography, colour, layout, how the
"you and the agent" month is shown), each a self-contained static HTML file
with inlined CSS and assets, Finnish content, working at phone and desktop
widths. After Jari picks and comments, iterate, and finally carry the chosen
direction into the Starlight theme (landing page, guide pages, fi and en).

## Acceptance Criteria

- [ ] Prototype round 1: 3–4 self-contained HTML directions under `issues/site-visual-identity/prototypes/round-1/`, each with a short rationale.
- [ ] Iteration rounds on Jari's feedback.
- [ ] Chosen direction applied to the site theme and landing page in fi and en; `scripts/check-user-guide.sh` passes.

## Comments

### 2026-11-12T13:12:56Z · @conductor

Jari 12.11.2026, copy for the landing page's 'why' (to use in a later round):
- Cost angle: a bookkeeper costs about 150 €/month; an AI agent subscription about 20 €/month.
- You get better reports and a better picture of where the company stands, which matters most when starting a company.
- The agent is patient and exact and sorts out bigger messes without complaint: for example parking receipts that also contain EV charging fees get split correctly, and a missing receipt is something the agent goes and finds, instead of a bookkeeper's reprimand.

### 2026-11-13T07:59:45Z · @conductor

Copy for the landing page's "Miksi" section as worded on 2026-11-12, awaiting Jari's approval before a design round:

Miksi agentti eikä tilitoimisto — small two-row table: Tilitoimisto noin 150 € / Agentti noin 20 € per month.
1. **Halvempi.** Kuukausimaksu on tekoälytilauksen hinta, ei tilitoimiston.
2. **Kysy kirjanpidoltasi mitä vain.** Koko kirjanpitosi on omalla koneellasi, ja agenttisi lukee sen tosite tositteelta. Samalla se tietää yrityksestäsi kaiken muunkin, mitä olet sille kertonut. Siksi saat vastauksen mihin tahansa kysymykseen saman tien ja tarkkoja, perusteltuja näkemyksiä, joita kirjanpitäjältä on vaikea saada, koska kirjanpitäjä on sinun ja tietojesi välissä.
3. **Tarkka eikä kyllästy.** Agentti erottelee parkkikuitista sähköauton latausmaksut ja selvittää isommatkin sotkut mutisematta.
4. **Kadonnut kuitti on agentin tehtävä.** Agentti etsii puuttuvan tositteen, eikä kukaan anna sinulle sapiskaa.

English: Cheaper — you pay for an AI subscription, not an accounting firm. Ask your books anything — your whole ledger is on your own computer and your agent reads every voucher; it also knows everything else you have told it about your company, so you get answers on the spot and precise, reasoned insight that is hard to get from a bookkeeper, because a bookkeeper stands between you and your own figures. Exact, and never tired of it — splits EV charging fees out of a parking receipt and sorts out bigger messes without complaint. A missing receipt is the agent's job — it goes and finds it, and nobody scolds you.

Round 5 (Vuorot voucher sequence) is live. Open points for Jari: light-first default, heavy bold glossary paragraphs, terminal frame on command blocks.
