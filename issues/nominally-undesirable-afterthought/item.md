---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: open
priority: normal
provenance: ai-review
source_ref: taskfleet:01m7yfttmj4acrkhe2exge2c7b/review-finding:masterdata-corrupt-names-no-remedy
review_source: ai-review
originating_run: 01m7yfttmj4acrkhe2exge2c7b
originating_run_kind: spinoff
related: ['@skill-init-rebuild']
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_target: skills/aggountant-init/SKILL.md (worker fact-check against the built binary, not a model finding)
lane: agent-surface
lane_seq: 5
preparation: ready
---

# MASTERDATA_CORRUPT for a missing bundled file does not name home upgrade

## Description

## Problem

A home written by an older `init` lacks masterdata a newer binary needs. Every
command that opens masterdata then fails with `MASTERDATA_CORRUPT`, and the
message names the missing file but not what settles it:

```
MASTERDATA_CORRUPT: active chart has no readable VAT role metadata at
<home>/masterdata/reference/vat-account-roles.json: No such file or directory
```

Reproduced on a synthetic home: `init`, delete
`masterdata/reference/vat-account-roles.json`, run `chart list` or
`voucher verify`. `home upgrade --dry-run` on the same home plans to add the
file, so the program knows the remedy.

## Why it matters

"Corrupt" plus a missing-file path invites the agent to restore or hand-write
the file. The remedy (`home upgrade`) is known only from the skill and the
guide's error table. Per `skills/AGENTS.md` an error message should say what to
do; until it does, `skills/aggountant-init/SKILL.md` keeps the sentence naming
`MASTERDATA_CORRUPT` as a trigger for `home upgrade`.

## Wanted

When the cause is a missing bundled reference file that `home upgrade` would
add, the message (or a `next_command`) names `aggountant home upgrade
--dry-run`. Then remove the `MASTERDATA_CORRUPT` clause from the last
paragraph of `skills/aggountant-init/SKILL.md` ("Myöhemmin").
