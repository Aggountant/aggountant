---
created: 2026-11-12
updated: 2026-11-12
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
