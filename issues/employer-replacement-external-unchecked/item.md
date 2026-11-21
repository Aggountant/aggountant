---
created: 2026-11-21
updated: 2026-11-21
type: bug
status: untriaged
priority: normal
provenance: ai-review
source_ref: taskfleet:01m7yj1dwxfz2vr4z5h867tt8t/review-finding:sha1:e313eabcf2ecf22e7463489110cf417d17184fe2
review_source: ai-review
originating_run: 01m7yj1dwxfz2vr4z5h867tt8t
originating_run_kind: spinoff
labels:
- ai-review-model:gpt-6-astra
assessment_classification: CONFIRMED
assessment_outcome: SPIN_OFF
review_target: skills/vero-fi and skills/tulorekisteri (skill-vero-split draft)
review_confidence: MED
review_severity: Medium
---

# employer-report: replacing an externally filed month skips the pension-insurance check

## Description

## Problem

`file-wage employer-report submit --replace` for a month that was recorded
with `complete-manual` (a report given outside aggountant) builds the
replacement from the current profile and skips the check that a replacement
repeats the previous version's pension insurances.

In `crates/process/src/file_employer_report.rs` the planner takes
`previous = current.and_then(|r| r.report.as_ref())`. For `Filing::External`
there is no stored report, so `previous` is `None`: the pension insurances come
from `profile.pension_insurances`, and both the `EMPLOYER_REPORT_UNCHANGED` and
the `EMPLOYER_REPORT_REPLACEMENT_NOT_ALLOWED` comparisons are skipped. The same
code's comment states the register's rule: in a replacement the pension
insurances match the previous version, and a changed provider or policy is
corrected by cancel-and-new, never by a replacement.

## When it happens

After catching up history: a month an earlier payroll provider reported is
imported from transcript 630, the user fetches its report reference, the agent
records it with `complete-manual`, and a correction is then sent with
`--replace`. The guide's hint for `EMPLOYER_REPORT_FILED_OUTSIDE` and
`EMPLOYER_REPORT_NO_REFERENCE` describes exactly this path. Example (Esimerkki
Oy, fictional): the earlier provider reported March 2026 with pension policy
12-3456789; the profile now holds `--no-pension-insurance`; the replacement is
sent with an empty list.

## Impact

Either the register refuses the replacement (not tested here), or it accepts a
report whose insurance details changed without anyone deciding to change them.
The preview does show the insurances, but nothing tells the user they must equal
the original's.

## Direction

When the previous version's content is unknown, refuse the replacement unless
the caller states the original report's pension insurances (then compare as
usual), or say in the preview that the comparison could not be made.

Until then `skills/tulorekisteri/kiinnikurominen.md` asks the agent to have the
user compare the original report's insurances with the profile ("Ohjelma ei
tunne alkuperäisen ilmoituksen sisältöä…"); remove that when the program checks.
