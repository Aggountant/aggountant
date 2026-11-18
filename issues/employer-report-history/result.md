# Result: employer-report-history

Two commands let an agent find the employer's separate report months that are
actually missing, without guessing:

```
aggountant file-wage employer-report registration [--as-of DATE]
aggountant file-wage employer-report import-transcript <file.csv> [--dry-run]
```

`missing` stays offline and now leaves out months an imported transcript shows
reported, listing them under `filed_outside` with their evidence.

## What was built

- `crates/broker/src/ytj.rs`: read-only lookup of PRH/YTJ open data
  (`/opendata-ytj-api/v3/companies?businessId=`), behind the `YtjLookup` trait.
  `HttpYtj` is the production client (blocking reqwest, rustls, HTTPS only,
  20 s timeout, generic user agent). Any connection failure or non-200 is
  `Unavailable`; a 200 that is not the documented shape is `InvalidResponse`;
  an empty result is "company unknown". Nothing is sent but the Business ID.
- `crates/process/src/employer_report_history.rs`:
  - `employer_registration`: takes register `7` entries, reads `REK_KDI`
    types `41`/`42` as registered and ignores `99` (an erroneous line, shown
    under `ignored_entries`); any other type is refused
    (`YTJ_UNKNOWN_ENTRY_TYPE`) rather than guessed. Reports the current entry,
    every period (current, ended, future), and a comparison with
    `registered_from`: `match`, `profile_later_than_register` (with
    `hidden_months`), `profile_earlier_than_register`, `register_ended`,
    `not_in_register`, `profile_not_set`. `implied_registered_from` is the
    current entry's first month, never before 2019-01. Writes nothing.
  - `import_employer_transcript`: parses the CSV, stores it through the
    ledger's artifact store and writes
    `process/employer-report-transcripts/<sha256>.json`. Reports `recorded`,
    `already_held` (aggountant's own or `complete-manual` receipt, not
    overwritten), `not_reported`, and `conflicts`
    (`accepted_in_aggountant_none_in_transcript`, `transcripts_disagree`,
    `in_doubt_send_transcript_counts_report`). Re-importing the same file is a
    no-op (`already_imported: true`).
  - `TranscriptImports::evidence`: the one rule. A month has transcript
    evidence when any imported transcript counts at least one report for it;
    imports are never ranked against each other.
- `crates/process/src/transcript_630.rs`: the strict parser (below).
- `crates/process/src/file_employer_report.rs`: two new derived month states.
  `filed_outside`: no own accepted report, a transcript counts one, no
  transcript contradicts it. `transcript_conflict`: transcripts disagree.
  `missing` hides `filed_outside` months (listed in `filed_outside` with
  source `incomes_register_transcript_630`, artifact, counts and totals) and
  lists `transcript_conflict` months. Preview refuses
  `EMPLOYER_REPORT_FILED_OUTSIDE` (new report), `EMPLOYER_REPORT_NO_REFERENCE`
  (`--replace`, pointing to the e-service) and
  `EMPLOYER_REPORT_TRANSCRIPT_CONFLICT`. `complete-manual` with the e-service
  reference turns any of them into an ordinary accepted month.
- `crates/cli/src/wage.rs`: the two subcommands with examples in `--help`;
  `YTJ_UNAVAILABLE` and `YTJ_RESPONSE_INVALID` are `system` errors (exit 2),
  every other new code is `user` (exit 1).
- Guide (fi, en): `tasks/file-employer-report.md` sections "Tarkista
  rekisteröinnin alku YTJ:stä" / "Check the registration start in YTJ" and
  "Kun aiempi palkanlaskija on antanut ilmoituksia" / "When an earlier payroll
  service filed reports"; error codes; command map.
- Skill: `skills/vero-fi/SKILL.md`, the order registration → transcript 630
  from the e-service (the one step that needs it) → import → `missing` →
  oldest first; the `--replace` refusal for transcript months.

## The transcript 630 layout: what the documents pin and what they do not

Source: *Raportit ja aineistot tulorekisteristä 2026*, version 1.3,
§4.6 (Finnish edition,
<https://www.vero.fi/globalassets/tulorekisteri/dokumentaatio-2026/raportit-ja-aineistot-tulorekisterista-2026.pdf>,
SHA-256 `7102b58f…f3c8`, downloaded 2026-11-18), checked against the English
*Transcripts and records from the Incomes Register 2026* v1.3 (`3a51e9db…5c48`)
and 2027 (`9df07f42…69a4`) and the Swedish edition (`ea59d9ee…b53562d`).

**Pinned by the document, and enforced:** the data items and their order
(§4.6.2: 33 items from "Ilmoituksen tyyppi" to "Työnantajan
sairausvakuutusmaksusta tehtävät vähennykset"), the values of "Ilmoituksen
tyyppi" and "Ajanjakso", that the employer's separate report part is per
reporting period ("Kohdekauden alkupäivä", the first day of the month) with a
range total, and the count rule (§9.2.1: valid reports at generation time, a
report and its replacements once, cancelled ones not at all). The parser
requires the header to be exactly those 33 names in that order, every row to
have 33 cells, known type and period values, one row per reporting period, all
periods inside the range, year and month agreeing with the start date, and the
range total to equal the sum of the periods.

**Not pinned, so accepted only where the file verifies it:** separator (`;`,
`,` or tab, whichever splits the first line into exactly the documented
header), encoding (UTF-8 with or without BOM, otherwise ISO-8859-1; the header
must still match), quoting (RFC 4180 style), date format (`D.M.YYYY` or
`YYYY-MM-DD`), amount format (decimal comma or point, at most two decimals, no
thousands grouping; an empty amount is zero), and the "Aikaväli" format (two
such dates joined by a dash). Anything else is refused with a specific code.

**Only Finnish.** The English edition omits six columns (period, range, year,
month, payment date, reporting-period start) and the Swedish edition omits a
column and the employer's separate report type, so their headers cannot be
pinned. An English or Swedish file is refused with a message to order the
transcript in Finnish.

**Not verified against a real file.** No real transcript 630 was available.
The column names are the document's data item names; if the e-service writes
different header text, the first real import fails with
`TRANSCRIPT_UNKNOWN_HEADER` and shows the line it found, and the fix is the
`HEADER_FI` constant. Nothing is stored on a refusal.

## Acceptance

- Registration, several register-7 entries, an ended entry, a company not in
  register 7, an unknown company, network failure ≠ not registered, profile not
  set: `crates/process/tests/employer_report_history.rs`
  (`registration_compares_the_current_employer_entry_with_the_profile`,
  `a_network_failure_is_never_read_as_not_registered`) with recorded synthetic
  YTJ JSON in `crates/process/tests/fixtures/employer_report/`; unit tests in
  `employer_report_history.rs` and `ytj.rs`. No test touches the network.
- Import records months with reports, keeps held months, reports the conflict,
  leaves zero months missing, `missing` shows the evidence source, re-import
  is a no-op, dry run writes nothing:
  `transcript_months_leave_missing_with_their_evidence_and_held_months_are_kept`.
- No resend and no `--replace` for a transcript month; `complete-manual`
  lifts both: `a_month_known_only_from_a_transcript_is_neither_resent_nor_replaced`.
- Disagreeing transcripts in either import order:
  `transcripts_that_disagree_make_a_visible_conflict_whatever_the_import_order`.
- Unknown header refused, nothing written:
  `an_unknown_header_is_refused_and_nothing_is_written`, parser unit tests,
  and the CLI test `crates/cli/tests/employer_report_history_cli.rs`, which
  also checks exit codes, JSON output and `--help`.
- Live smoke test in a scratch home (not in the repository): an unknown
  Business ID gave `YTJ_COMPANY_NOT_FOUND`; a real public company's answer
  parsed as `profile_not_set` with an entry from before 2019 implying 2019-01.

## Decisions for later review

- **Transcripts are never ranked.** The CSV has no generation time, and an
  order may be limited by reception time without the file saying so. A
  positive count therefore always blocks a new send, and disagreement is a
  visible conflict settled only through the e-service and `complete-manual`
  (review F1, F2). Cost: a report genuinely cancelled after an earlier
  transcript leaves the month in conflict instead of plainly missing, and it
  is then reported through the e-service.
- **Evidence is derived when read**, from stored transcript records, not
  written per month at import. A later own accepted report or
  `complete-manual` takes precedence without any record being rewritten.
- **An in-doubt send stays `outcome_unknown`** even when a transcript counts a
  report: the transcript may be that very send, and the existing rule says
  only `status` or e-service evidence settles it. The import reports it.
- **The import is not a Run.** It is evidence like company knowledge, with no
  external effect; the file is an artifact and the record names its hash.
- **The payer is not checked.** Transcript 630 does not contain the payer's
  Business ID; the output warns and the skill tells the agent to confirm with
  the user.
- **`register_ended` does not shorten `missing`**; the listing still runs to
  last month. Left out until a company whose entry has ended uses it.
- New modules in `crates/process` (`employer_report_history.rs`,
  `transcript_630.rs`) and in `crates/broker` (`ytj.rs`) instead of growing
  `file_employer_report.rs`; `crates/process/src/lib.rs` and
  `crates/broker/src/lib.rs` declare them.

## Gates

`scripts/ci-local.sh` green (fmt, clippy, cargo-deny, build and test on stable
and 1.89, user guide) both before and after the review fixes.

## Review

`review.md` (`/llm-review`, critical; `gpt-6-astra` finder, `claude-fable-5-2`
verifier) and `assessment.md`: two confirmed must-fix findings, both fixed in
this slice. No follow-up issue was filed.
