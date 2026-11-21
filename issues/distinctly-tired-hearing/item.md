---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: done
priority: normal
provenance: ai-review
source_ref: taskfleet:01m7yfttmj4acrkhe2exge2c7b/review-finding:chart-retention-no-path
review_source: ai-review
originating_run: 01m7yfttmj4acrkhe2exge2c7b
originating_run_kind: spinoff
labels:
- ai-review-model:gpt-6-astra
related: ['@skill-init-rebuild']
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_target: skills/aggountant-init/SKILL.md
lane: chart-decisions
lane_seq: 0
preparation: ready
closed: 2026-11-21
---

# Set-up offers keeping the previous chart of accounts, but no path installs one

## Description

## Problem

Set-up asks the user whether the company continues its previous chart of
accounts or moves to the default chart: `skills/aggountant-init/SKILL.md`
("tilikarttapäätös: jatketaanko aiempaa tilikarttaa vai siirrytäänkö
oletustilikarttaan") and the guide's `start/set-up-company.md` step 3. `init`
always writes the default chart (`fi-general-2026`), and `chart` has only
`list`: there is no command, help text or guide page for bringing a company's
own chart into a home. The guide's `tasks/upgrade.md` mentions "a company's
own chart" as something that can exist, without saying how it got there.

## Why it matters

An agent offers the user a choice it has no documented way to carry out. If
the user answers "keep our chart", the agent either hand-edits
`masterdata/reference/` (the skill's own upgrade paragraph says not to repair
those files by hand) or silently maps to the default chart after recording the
opposite decision.

## Wanted

A decision on what v1 supports, then one of: a documented path for a company
chart (command or guide page the skill can point to), or the question removed
from the skill and the guide so set-up maps source accounts to the default
chart and says which distinctions that loses.

## Decision (2026-11-21, Jari Mustonen)

Remove the choice. Set-up states that aggountant uses its default chart and
that the previous system's accounts are mapped onto it in the opening voucher
(as the first production company did). The skill (`aggountant-init`), the
guide's `start/set-up-company.md` (fi, en) and `tasks/upgrade.md`'s mention of
"a company's own chart" change accordingly. Importing a company's own chart
stays with `sie4-chart-import` for when a user actually needs it.

## Resolution

### 2026-11-21T10:21:44Z · @issuectl

Choice removed from set-up; skill and guide say the default chart is used and old accounts are mapped in the opening voucher. See result.md.
