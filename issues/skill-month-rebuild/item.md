---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: done
priority: high
preparation: ready
lane: skills-rebuild
lane_seq: 3
collision: [skills/capture-receipt/SKILL.md, crates/cli/src/skill.rs]
blocked_by: ['@skill-soul-rebuild', '@cli-stdin-input-help']
closed: 2026-11-21
---

# Skill rebuild: the month's bookkeeping skill, with the bank import and the month close

## Description

Skill rebuild (after `skill-soul-rebuild` and `cli-stdin-input-help`).
`capture-receipt` promises "the month's transactions", but no skill covers the
month: the bank import is a side clause, the month close
(`run start close-period`) and the move on to filings are nowhere, although
the guide tells users to say "close March" and that the skills are the
agent's manual. The contra-account section (receipt against 2700/2740, the
bank row clears it) is the most valuable single part of all skills and stays.

## Wanted

`capture-receipt` becomes the month's bookkeeping skill, about 1,100 words, in
the order the work happens: material, receipts, bank statement, reconciliation,
close, hand-over to the filings. The contra-account section as it is; special
entries' VAT marking as its own section (today a 150-word bullet); what to do
when `eu-goods` refuses on the default chart; the bank import's assignments
explained by pointing to `import camt053 seal --help`. House-rule copies go
(SOUL has them). Decide whether the skill is renamed and say why; if renamed,
handle existing installs (stale copies are never deleted, `skills/AGENTS.md`).

## Resolution

### 2026-11-21T07:39:41Z · @issuectl

capture-receipt rebuilt as the month's skill (1,501 -> 1,220 words), name kept; review, assessment and result are beside the issue. ci-local passes; five gaps filed unlaned.
