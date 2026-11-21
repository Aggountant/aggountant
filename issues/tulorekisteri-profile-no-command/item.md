---
created: 2026-11-21
updated: 2026-11-21
type: improvement
status: open
priority: normal
provenance: other
provenance_detail: product gap found by taskfleet worker while rebuilding the vero-fi skill
source_ref: taskfleet:01m7yj1dwxfz2vr4z5h867tt8t/observed:tulorekisteri-profile-no-command
originating_run: 01m7yj1dwxfz2vr4z5h867tt8t
originating_run_kind: spinoff
lane: agent-surface
lane_seq: 7
preparation: ready
---

# Incomes Register profile has no command, no help and no itemised readiness

## Description

## Problem

The Incomes Register profile `.aggountant/tulorekisteri.json` has no command
and no description in the program, and readiness is not itemised.

- No command writes `schema_version`, `mode`, `business_id`,
  `reporting_permission_active` and `credentials_file`. Only the
  `employer_report` block has a writer (`file-wage employer-report configure`),
  and it requires the file to exist already.
- No `--help` shows the file's shape. The only description an agent has is the
  JSON example in `skills/tulorekisteri/kayttoonotto.md`.
- When something is missing, `check_readiness` in
  `crates/broker/src/tulorekisteri.rs` answers with one sentence that lists
  every requirement, whichever one failed. `vero-fi status` by contrast lists
  each blocker with a code and an owner, offline.
- `aggountant explain TULOREKISTERI_NOT_READY` has no entry.

## Impact

Every company that takes the Incomes Register channel into use has its agent
hand-write a JSON file from a skill example, including the user's attestation
that the reporting permission is granted. A typo or a wrong field is found only
when a report is about to be sent, from a message that does not say which
requirement failed. `skills/AGENTS.md` names this situation as a product gap: a
skill carrying an input format means the program is missing something.

## Direction

A `configure` for the profile with an INPUT section, merging like
`vero-fi configure`, and an offline readiness command (or `vero-fi status`
extended) that lists blockers one by one with owners. Then remove from
`skills/tulorekisteri/kayttoonotto.md` the "Profiili" section's JSON example
and the paragraph beginning "Ohjelma ei erittele, mikä valmiudesta puuttuu",
and the "no command" lines in the guide (`reference/status.md`,
`tasks/report-travel-allowances.md`, both languages).
