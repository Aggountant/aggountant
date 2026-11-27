---
created: 2026-11-27
updated: 2026-11-27
type: improvement
status: untriaged
priority: normal
provenance: ai-review
source_ref: taskfleet:01m8ehz1eezcc03sc8dzm8xy55/review-finding:municipality-free-text
review_source: ai-review
originating_run: 01m8ehz1eezcc03sc8dzm8xy55
originating_run_kind: spinoff
review_target: claimant-workplace-register 9808ba7..f21b193
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_severity: must-fix
review_confidence: high
labels:
- ai-review-model:gpt-6-astra
- ai-review-model:claude-fable-5-2
---

# Claimant register compares municipality names as free text; the secondary-workplace check can pass for one municipality

## Description

## Problem

The claimant register (`crates/domain/src/workplace.rs`, `validate_workplace`)
refuses a secondary workplace (toissijainen työpaikka) in the same
municipality as the primary regular workplace, as the Tax Administration's
guidance VH/630/00.01.00/2026 section 4.1 requires ("paikkakunnalla
tarkoitetaan kuntaa"). The comparison, `same_municipality`, only trims and
lowercases the free-text `municipality` field of the two places.

So two spellings of one municipality pass as different: "Helsinki" and
"Helsingfors" (the Finnish and Swedish names of the same municipality),
"Tampere" and "Tampereen kaupunki", or a typo. A second office in the same
municipality can then be recorded as a secondary workplace. The check fails
open in the direction of tax-free treatment: a secondary workplace earns tax-free
travel and lodging costs, while another regular workplace in the same
municipality earns nothing (guidance 3.2, 3.9). The only safeguard today is
that the user confirms the entry.

## Expected

Places carry an official municipality identity (Statistics Finland's
municipality code), validated against a bundled classification that accepts
both the Finnish and the Swedish name, and the secondary-workplace check
compares codes. An unknown name is refused rather than taken as different.
Two things need a decision while designing it: places abroad (a secondary
workplace may be in another country, guidance 4.1), and the yearly update of
the classification when municipalities merge (the guidance's example 21 shows
a merger changing a place's nature).

## Source

Review of the `claimant-workplace-register` slice (finder gpt-6-astra,
verified by claude-fable-5-2): finding F3 of the first round, confirmed
against the code and deferred because it needs bundled reference data.
Synthetic example: Esimerkki Oy's managing director with a primary office
recorded in "Helsinki" and a second office recorded in "Helsingfors".
