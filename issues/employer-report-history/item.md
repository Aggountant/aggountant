---
created: 2026-11-18
updated: 2026-11-18
type: feature
status: done
priority: high
lane: wage-reporting
lane_seq: 0
collision: [crates/cli/src/wage.rs, crates/process/src/file_employer_report.rs, skills/vero-fi/SKILL.md]
preparation: ready
closed: 2026-11-18
closed_by: claude-opus-5-5
---

# employer-report: read the employer-register start from YTJ and import already-filed months from the Incomes Register transcript

## Description

`file-wage employer-report missing` lists every month from the profile's
`registered_from` to last month that has no accepted report in aggountant's
own records. Two facts behind that list come from the user today, and the
user often does not know either of them:

1. **When the company entered the employer register.** `registered_from` is
   typed by hand. A company that registered years ago and forgot about it (a
   common case for small owner-run companies) sets it to a recent month and
   silently hides years of months from `missing`; one that guesses too early
   gets months it never owed.
2. **Which months were already reported outside aggountant.** A previous
   payroll service or accountant may have given employer's separate reports
   for earlier months. aggountant cannot see them, so `missing` lists them as
   `not_filed`, and the only way out is `complete-manual`, one month at a time,
   with an Incomes Register report ID the user has to dig up.

Example (Esimerkki Oy, fictional, Business ID 1234567-1): registered as an
employer in February 2020; a payroll service reported most months until
June 2026; aggountant has run the books since July 2026. With
`registered_from: "2020-02"` `missing` shows some 80 months, nearly all of them
already reported. With `registered_from: "2026-07"` it shows four, and any gap
before July stays invisible.

### What the authorities offer

- **Employer register entry: public, no credentials.** PRH/YTJ open data
  (`https://avoindata.prh.fi/opendata-ytj-api/v3/companies?businessId=<id>`,
  CC BY 4.0) returns `registeredEntries`; register code `7` is
  *Työnantajarekisteri* (code list: `/v3/description?code=REK&lang=fi`), with
  `registrationDate`, optional `endDate`, and a status in `type` /
  `descriptions` (for example "Rekisterissä"). Several entries per register
  are possible (history); treat the one covering today, and report past
  periods too.
- **The payer's own submitted employer's separate reports: no technical-
  interface query for a payer.** The real-time query
  `GetPayerSummaryReportsOnePayer` (record type 403) requires a *data user's*
  data access profile (Transcripts and records from the Incomes Register 2026,
  §7.5). What a payer can get is transcript **630 "Summary of payer's earnings
  payment data (csv)"** (same document §4.6): it can be ordered in the
  e-service (or via the interface) but is delivered only to the e-service, and
  it contains, per reporting period, the *number of employer's separate
  reports* and their health insurance contribution and deduction totals
  (§4.6.2.2, `PayerSummaryReportCount`, `HealthInsContribution`,
  `HealthInsContributionDeductions`). It has no report IDs.

## Wanted

1. **Registration from YTJ.** A read-only command, e.g.
   `file-wage employer-report registration`, that fetches the employer
   register entries for the profile's `business_id`, prints the current entry
   (start, end, status) and any earlier periods, and compares the start month
   with `registered_from`: match, profile later than register (months hidden),
   profile earlier than register, register ended, or not in the register. It
   writes nothing; the JSON output says which `registered_from` the register
   implies so an agent can update the profile on the user's say-so. A network
   failure is its own error, never "not registered". `missing` stays offline.
2. **Import transcript 630.** A command, e.g.
   `file-wage employer-report import-transcript <file.csv>`, that reads the
   e-service CSV, stores the file as an artifact, and records each reporting
   month with at least one employer's separate report as filed outside
   aggountant, with the transcript as evidence (count and totals per month).
   `missing` then excludes those months and says where the evidence came
   from. A month with no reports in the transcript stays missing. A month
   aggountant already holds (its own accepted report or `complete-manual`
   evidence) is not overwritten; a conflict (aggountant accepted, transcript
   shows none, for a month the transcript covers) is reported. Re-importing
   the same file is a no-op. Months recorded this way have no IR report ID, so
   a later `--replace` for them is refused with a message pointing to the
   e-service; say this in the skill.
3. **Skill.** `skills/vero-fi/SKILL.md`, *Työnantajan erillisilmoitus*:
   the order is registration check → order and download transcript 630 in
   the e-service for the period the register covers (the e-service query
   range limits apply) → import → `missing` → preview, approve and submit
   oldest first. Tell the user plainly that the transcript is the one step
   that needs the e-service.

The CSV layout must come from the official document (column names and order,
encoding, separator, how reporting periods are written); build synthetic
fixtures from it, and make the parser fail loudly on an unknown header rather
than guess. If the document leaves the layout open, say so in the result and
keep the parser strict on what it does specify.

Sources: vero.fi Incomes Register documentation page
(`/en/incomes-register/software-developers/documentation/`): *Transcripts
and records from the Incomes Register 2026* (§4.6, §7.5), *Application
instructions for the employer's separate report 2027*, *Technical interface –
Application instructions … 2026* (PayerSummaryReportQueryService).

## Resolution

### 2026-11-18T14:41:22Z · @claude-opus-5-5

Both commands landed with docs, skill and review; see result.md. The transcript 630 CSV layout is pinned from the Finnish documentation only and has not been seen in a real file yet.
