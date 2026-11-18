# Assessment of the employer-report-history review

Assessed against `13d05de` (the review fixes are in that commit).

| # | Finding | Conf | Like | Read | Arch | Confidence | Recommendation |
|---|---|---|---|---|---|---|---|
| F1 | Import time decided which transcript was newer | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | HIGH | FIX (Rule 5: confirmed, real impact) — applied |
| F2 | Reception-time-filtered transcript erased evidence | CONFIRMED | OCCASIONAL | NEUTRAL | MINOR | LOW | FIX (Rule 5) — applied with F1 |

FIX: 2   FIX (with care): 0   SPIN-OFF: 0   DISCUSS: 0   DROP: 0

- F1: the code ranked imports by `imported_at`; the CSV has no generation
  time, and the conflict was returned without gating anything. Both failure
  directions are data integrity (a duplicate report to the register, or a
  hidden late month). Fixed by not ranking at all: any positive count blocks a
  new send, and disagreement is a `transcript_conflict` month that `missing`
  lists and preview refuses until e-service evidence is recorded. Tested in
  both import orders.
- F2: §4.6.1.1 lets a 630 order filter employer's separate reports by
  reception time, and the file does not say so. Same fix as F1, plus the
  guide, the skill and `--help` say to leave the reception range empty.
  Likelihood is OCCASIONAL with low confidence: the subscription schema has the
  field; whether the e-service web form shows it was not verified. A filtered
  transcript imported alone can still show a reported month as missing, which
  is the pre-slice behaviour, not a regression.

No confirmed data-integrity finding remains open. No follow-up issue was
filed: both findings are fixed in this slice.

## Simplicity

- Reused: the existing month model in `file_employer_report.rs` (one new state
  derived like the others, no second status path), the ledger's artifact store
  for the CSV, `complete-manual` as the one way to settle a month from
  e-service evidence, `ReportMonth`, `BusinessId`, the broker's existing
  `reqwest` dependency.
- Repetition: the rule "what makes a month filed outside" lives in one place,
  `TranscriptImports::evidence`; status, `missing`, preview and the import's
  conflict report all read it. The import record is the parsed transcript plus
  the artifact hash, not a copy of per-month decisions, so a later import never
  has to rewrite an earlier one.
- Left out on purpose: English and Swedish transcripts (their documentation is
  incomplete, so the header could not be pinned); a generation-time or
  "unrestricted" flag supplied by the user (the safe merge above makes
  ordering irrelevant); stopping `missing` at a register end date; writing
  `registered_from` from YTJ automatically (the issue wants it on the user's
  say-so); a Run or journal for the import (it is evidence, not a workflow).
