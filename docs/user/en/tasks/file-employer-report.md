# File the employer's separate report

When your company is in the Tax Administration's employer register, you file
an employer's separate report (työnantajan erillisilmoitus) with Tulorekisteri,
the Finnish Incomes Register, every month, including months in which you paid
nothing. The deadline is the 5th day of the following month. A company that
pays wages only occasionally and is not in the register does not file the
separate report for months in which it paid nothing.

aggountant sends the report over the same Tulorekisteri production connection
as travel allowances and tells you which months are missing. The earnings
payment report for travel allowances is a different report and does not
replace the separate report.

## Before you start

- [The Tulorekisteri API permission and profile](report-travel-allowances.md#set-up-reporting-once)
  are in place. The separate report uses the same permission and credential
  file as travel allowances.
- You know the month from which the company has been in the employer
  register. You find it in OmaVero or in the Tax Administration's decision,
  or the agent [checks it in YTJ](#check-the-registration-start-in-ytj).
- If the company has earnings-related pension insurance (TyEL), you know the
  pension company and the policy number. If the only insured person is an
  owner with self-employed pension insurance (YEL), these are not needed.

## Set up the separate report (once)

1. **Ask the agent to add the separate report settings.**

   > The company has been in the employer register since August 2026. Let's
   > add the employer's separate report. The contact person is Maija
   > Meikäläinen, phone 040 123 4567. There is no TyEL insurance; the owner
   > has YEL.

2. **The agent adds the settings to the Tulorekisteri profile:** the first
   registration month, the contact person and the pension insurances.
   aggountant checks that the phone number, the pension company codes and the
   policy numbers are in a form the report accepts, and compares the first
   month with
   [the employer register in YTJ](#check-the-registration-start-in-ytj).

3. **Check the settings the agent shows you.** They go into the report, and
   from the first month on a report is expected for every month.

If the register has a different first month from the one you gave, aggountant
does not save the settings. The agent shows you the comparison and saves your
month only if you confirm it. If YTJ does not answer, aggountant saves nothing
unless you and the agent agree to save without the check.

<details>
<summary>The command the agent runs</summary>

```sh
aggountant file-wage employer-report configure --registered-from 2026-08 \
  --contact-name "Maija Meikäläinen" --contact-phone "040 123 4567" \
  --no-pension-insurance --dry-run
```

A pension insurance is given as
`--pension-insurance PROVIDER_CODE:POLICY_NUMBER`, and the option can be
repeated. If there are none, that is stated with `--no-pension-insurance`. The
same settings can come as JSON on standard input.

`--accept-register-mismatch` saves a month YTJ disagrees with, and `--offline`
saves without the YTJ check.

The command changes only the `employer_report` section of the profile
(`registered_from`, `contact_name`, `contact_phone`, `pension_insurances`) and
shows it before and after.
</details>

### Check the registration start in YTJ

If you do not know when the company entered the employer register, ask the
agent to check:

> When did the company enter the employer register?

aggountant looks up the company's employer-register entries in PRH's YTJ open
data and compares them with the profile's first month. Only the Business ID is
sent, and no credentials are needed. The result says one of these:

- the first month matches
- the profile starts too late, so old months are left out of the missing list
- the profile starts too early
- the register entry has ended
- the company is not in the register.

Earlier register periods are shown too. Employer's separate reports exist from
January 2019, so an older registration means a first month of 2019-01.

The check does not change the profile. The agent changes the first month with
the setup command only once you approve it.

If YTJ does not answer, the result is "unknown", not "not registered". Try
again later or look the company up on ytj.fi.

<details>
<summary>The command the agent runs</summary>

```sh
aggountant file-wage employer-report registration
```
</details>

## How to do it

1. **Ask the agent.**

   > Let's file the employer's separate report for October.

2. **The agent finds out what was paid in the month** and asks you if the
   books do not show it.
   - If you paid nothing, the report says "No wages payable" (Ei
     palkanmaksua). aggountant refuses this choice for a month that has a
     travel allowance report.
   - If you paid only tax-free travel allowances (mileage allowances, per
     diems), the report gives the employer's health insurance contribution as
     €0.00. According to Tulorekisteri's guidance "No wages payable" is not
     correct then, because payments were made.
   - If you paid wages, file the report in Tulorekisteri's e-service.
     aggountant
     [does not yet compute the health insurance contribution](../reference/status.md#tulorekisteri-and-employees),
     so it does not file such a month.

3. **The agent previews the report.** The preview uses no network connection
   and saves nothing. It shows the month, the content, the contact person, any
   pension insurances, the deadline and the report checksum.

4. **Check and approve** the month and the content. The report is final once
   Tulorekisteri has accepted it, and after that it is corrected with a
   replacement report.

   > I approve the separate report for October.

5. **The agent sends the report** with exactly the checksum you approved and
   tells you Tulorekisteri's receipt.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant file-wage employer-report preview --month 2026-10 \
  --payments none --timestamp 2026-11-02T08:00:00Z
aggountant file-wage employer-report submit --month 2026-10 \
  --payments none --timestamp 2026-11-02T08:00:00Z --record-hash <checksum>
aggountant file-wage employer-report status --month 2026-10
```

`--payments` is `none`, `tax-free-reimbursements-only` or `wages`. The
preview and the submit get the same options and the same timestamp. The
timestamp also becomes the report's reporting date.
</details>

## Missing months

Ask the agent to check whether reports are missing:

> Are any employer's separate reports missing?

aggountant lists the months, from the start of the registration up to the
previous month, for which it knows of no report filed with Tulorekisteri. Each
shows its deadline and whether the deadline has passed. The agent goes through
the missing months from the oldest, and you approve each month separately. A
late report can lead to a late-filing fee, so file late months right away.

If you have already filed a month in the e-service, do not send it again. Give
the agent the report reference from the e-service, and the agent marks the
month as filed.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant file-wage employer-report missing
aggountant file-wage employer-report complete-manual --month 2026-08 \
  --ir-report-id <report reference>
```
</details>

### When an earlier payroll service filed reports

If an accounting firm or payroll service filed some months before aggountant,
there is no need to dig up the references one by one. Tulorekisteri's
transcript 630 shows how many separate reports each month has. You order the
transcript yourself in the Tulorekisteri e-service, because aggountant cannot
order it for you.

1. **Order the transcript in the e-service:** New subscription → *Summary of
   payer's earnings payment data (csv)* (630). Choose the reporting period
   range from the start of the registration, at most 24 months at a time, and
   **Finnish** (suomi) as the language. Leave the reception time range empty,
   because a limited transcript leaves out reports received at other times,
   and the file does not show the limit. Download the finished transcript from
   the *Subscriptions made* page. If the download is a ZIP file, unzip it.

2. **Give the file to the agent.** The transcript does not carry the payer's
   Business ID, so make sure it is your own company's.

   > Here is Tulorekisteri's transcript 630 for 2025.

3. **The agent imports the transcript.** aggountant stores the file and marks
   every month with at least one separate report as filed outside aggountant.
   A month without a report stays missing.

The import does not change a month aggountant already knows from its own
sending or from a recorded reference. If the transcript and aggountant's
records conflict, for example when aggountant has the month as filed but the
transcript has no report for it, the agent shows you the conflict. Importing
the same file again changes nothing.

In the missing-months list the transcript's months are shown separately, with
the transcript as their source. For such a month aggountant sends neither a
new report nor a replacement, because the transcript carries no report
reference. Correct the report in the e-service, or give the agent its
reference first.

Once the reference is recorded, aggountant still does not know which pension
insurance details the report filed outside it stated, and a replacement has to
repeat them. No transcript or interface returns them. So before a replacement,
open the original report under Submitted reports in the e-service and tell the
agent its pension insurances: the pension company code and the policy number,
or that there are none. Without them aggountant refuses the replacement
(`EMPLOYER_REPORT_PREVIOUS_UNKNOWN`). If they differ from your current
settings, aggountant refuses a "No wages payable" replacement, and you make
the correction in the e-service.

If two imported transcripts disagree about a month, one having a report and
the other none, the month is listed among the missing months as a conflict.
aggountant sends nothing for it until you check the month in the e-service and
the agent records the report reference.

aggountant reads only a Finnish-language transcript whose columns are those
the Tulorekisteri e-service writes. It refuses any other file
(`TRANSCRIPT_UNKNOWN_HEADER`) and stores nothing.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant file-wage employer-report import-transcript raportti-630.csv --dry-run
aggountant file-wage employer-report import-transcript raportti-630.csv
aggountant file-wage employer-report missing
aggountant file-wage employer-report complete-manual --month 2025-03 --ir-report-id REFERENCE
aggountant file-wage employer-report preview --month 2025-03 \
  --payments tax-free-reimbursements-only --timestamp 2026-11-21T08:00:00Z \
  --replace --previous-pension-insurance 46:12-3456789
```
</details>

## Done when

The month's state is `accepted` and the agent shows Tulorekisteri's report
reference. The month is no longer in the list of missing months. You can also
check the report in Tulorekisteri's e-service.

## If something goes wrong

- If the outcome of the sending is unclear, aggountant sends nothing again for
  the month until its state is settled. The agent asks Tulorekisteri for the
  state, as described in
  [The outcome of a filing is uncertain](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain).
- If the report needs correcting, ask the agent for a replacement report, for
  example when travel allowances turn out to have been paid in a month
  reported as "No wages payable". A replacement repeats the pension insurance
  details of the report it replaces. You correct the pension insurance details
  themselves, and cancel a report, in the e-service.
- If Tulorekisteri rejected the report, see
  [Tulorekisteri rejected the report](../troubleshooting.md#tulorekisteri-rejected-the-report).

## See also

- [Report travel allowances to Tulorekisteri](report-travel-allowances.md)
- [Authority filings and uncertain outcomes](../concepts/filings.md)
- [Error codes](../reference/error-codes.md)
