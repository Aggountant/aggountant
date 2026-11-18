# Review: employer-report-history

- Skill: `/llm-review`, rigor **critical**: the change decides which months
  count as already filed with the Incomes Register. Treating a month as filed
  wrongly hides a late report; treating it as missing wrongly invites a second,
  irreversible report.
- Scope: commit `352cbaf` against `f90f5f0` (`crates/broker/src/ytj.rs`,
  `crates/process/src/employer_report_history.rs`,
  `crates/process/src/transcript_630.rs`,
  `crates/process/src/file_employer_report.rs`, `crates/cli/src/wage.rs`,
  tests, guide, skill).
- Finder: `gpt-6-astra`, one call (diff plus the issue and
  `file_employer_report.rs`). Verifier: `claude-fable-5-2`, one call, asked to
  refute both surviving candidates with the code and an excerpt of the official
  document (*Raportit ja aineistot tulorekisteristä 2026* §4.6.1.1, §9.2.1).
  Two model calls in total.
- Risk class: authority filing state (duplicate send / hidden late month).

## F1 (must-fix): import time decided which transcript was newer

- Location: `crates/process/src/employer_report_history.rs`,
  `TranscriptImports::evidence` (the max-by-`imported_at` selection) and
  `import_employer_transcript` (`imported_at = now`).
- Claim: a transcript states the reports valid when it was generated, but the
  CSV carries no generation time; ranking imports by import time lets an older
  snapshot imported later override a newer one. Two failure directions: a
  zero-count older transcript erases positive evidence, so the month becomes
  missing and a duplicate report may be sent; a positive older transcript
  imported after a cancellation hides a month that is missing again.
- Evidence: local read; the parser's accepted header has no time column
  (`transcript_630.rs` `HEADER_FI`); the conflict was returned but never gated
  the write. Verifier: survives, and noted the positive-after-zero direction
  raised no conflict at all.
- Fix: imports are no longer ranked. Any transcript counting a report blocks a
  new send; transcripts that disagree on a month make it `transcript_conflict`,
  which `missing` lists and preview refuses
  (`EMPLOYER_REPORT_TRANSCRIPT_CONFLICT`) until e-service evidence is recorded
  with `complete-manual`. Test
  `transcripts_that_disagree_make_a_visible_conflict_whatever_the_import_order`
  runs both orders.

## F2 (must-fix): a reception-time-filtered transcript erased evidence

- Location: same as F1.
- Claim: transcript 630 can be ordered with an optional "Vastaanottohetken
  aikaväli" (reception-time range) that also filters employer's separate
  reports (§4.6.1.1); the CSV does not show the filter, so a filtered
  transcript reads a month as having no report.
- Evidence: §4.6.1.1 lines quoted by the verifier; the "Aikaväli" column is the
  reporting-period range only. Verifier: survives, with one unverified link:
  whether the e-service web form exposes the reception range (the subscription
  schema does).
- Fix: the F1 change (a zero never cancels a positive count), plus the guide,
  the skill and `--help` tell the user to leave the reception range empty.
  Residual: a filtered transcript imported alone still shows a reported month
  as missing. That is no worse than having no transcript at all (the month was
  missing before this slice too), and the instruction covers it.

## Dropped

None. Both candidates survived local checking and the verifier.

## Coverage

Read: the whole diff, the issue, the official 630 sections in fi/en/sv
editions, the YTJ v3 OpenAPI schema and `REK_KDI` code list. Tested: workspace
tests and clippy (`scripts/ci-local.sh`, stable and 1.89), live YTJ calls in a
scratch home (unknown ID → `YTJ_COMPANY_NOT_FOUND`; a real public company
parsed). Outside scope: a real transcript 630 file (none available; the layout
is from the document), and whether the register's real CSV uses the
documented column names verbatim.
