# File the employer's separate report

When your company is in the Tax Administration's employer register, an
employer's separate report (työnantajan erillisilmoitus) goes to Tulorekisteri,
the Finnish Incomes Register, every month, including months in which you paid
nothing. The deadline is the 5th day of the following month. aggountant sends
the report over the same Tulorekisteri production connection as travel
allowances and tells you which months are missing.

The separate report is a different report from the earnings payment report for
travel allowances. A travel allowance report does not replace it.

## Before you start

- **The Tulorekisteri API permission and profile** are in place
  ([Report travel allowances to Tulorekisteri](report-travel-allowances.md#set-up-reporting-once)).
  The separate report uses the same permission and credential file.
- **The first month of your employer registration.** You find it in OmaVero or
  in the Tax Administration's decision. A company that pays wages only
  occasionally and is not in the register does not file the separate report
  for months in which it paid nothing.
- **Earnings-related pension insurance (TyEL)**, if the company has one: the
  pension company and the policy number. If the only insured person is an
  owner with self-employed pension insurance (YEL), none is needed.

## Set up the separate report (once)

1. **Ask the agent to add the separate report settings.**

   > The company has been in the employer register since August 2026. Let's
   > add the employer's separate report. The contact person is Maija
   > Meikäläinen, phone 040 123 4567. There is no TyEL insurance; the owner
   > has YEL.

2. **The agent adds** the first registration month, the contact person and the
   pension insurances to the Tulorekisteri profile. Check them: they go into
   the report, and from the first month on a report is expected for every
   month.

## How to do it

1. **Ask the agent.**

   > Let's file the employer's separate report for October.

2. **The agent finds out what was paid in the month** and asks you if the
   books do not show it. There are three cases:
   - **No payments at all**: the report says "No wages payable" (Ei
     palkanmaksua).
   - **Only tax-free travel allowances** (mileage allowances, per diems): the
     report gives the employer's health insurance contribution as €0.00.
     According to Tulorekisteri's guidance "No wages payable" is not correct
     then, because payments were made.
   - **Wages**: aggountant does not yet compute the health insurance
     contribution, so it does not file such a month. File it in
     Tulorekisteri's e-service
     ([What is ready and what is not](../reference/status.md#tulorekisteri-and-employees)).

   aggountant refuses "No wages payable" for a month that has a travel
   allowance report.

3. **The agent previews the report.** The preview uses no network connection
   and saves nothing. It shows the month, the content, the contact person, any
   pension insurances, the deadline and the report checksum.

4. **Check and approve** the month and the content. The report is final once
   Tulorekisteri has accepted it; a correction is a replacement report.

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
timestamp also becomes the report's reporting date. The profile section is
`employer_report` (`registered_from`, `contact_name`, `contact_phone`,
`pension_insurances`).
</details>

## Missing months

Ask the agent to check whether reports are missing:

> Are any employer's separate reports missing?

aggountant lists the months from the start of the registration up to the
previous month for which Tulorekisteri has no report that aggountant knows of.
Each shows its deadline and whether the deadline has passed. The agent goes
through the missing months from the oldest, and you approve each month
separately. A late report can lead to a late-filing fee, so file late months
right away.

If you have already filed a month in the e-service, do not send it again. Give
the agent the report reference from the e-service, and the month is marked as
filed.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant file-wage employer-report missing
aggountant file-wage employer-report complete-manual --month 2026-08 \
  --ir-report-id <report reference>
```
</details>

## Done when

The month's state is `accepted` and the agent shows Tulorekisteri's report
reference. The month is no longer in the list of missing months. You can also
check the report in Tulorekisteri's e-service.

## If something goes wrong

- **The outcome of the sending is unclear**: nothing is sent again for the
  month until its state is settled. The agent asks Tulorekisteri for the state
  ([When something goes wrong](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain)).
- **The report needs correcting**: ask for a replacement report, for example
  when travel allowances turn out to have been paid in a month reported as "No
  wages payable". A replacement repeats the pension insurance details of the
  report it replaces. Correcting the pension insurance details themselves, and
  cancelling a report, are done in the e-service.
- **Tulorekisteri rejected the report**:
  [When something goes wrong](../troubleshooting.md#tulorekisteri-rejected-the-report).

## See also

- [Report travel allowances to Tulorekisteri](report-travel-allowances.md)
- [Authority filings and uncertain outcomes](../concepts/filings.md)
- [Error codes](../reference/error-codes.md)
