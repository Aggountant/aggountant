# Report travel allowances to Tulorekisteri

When the company pays tax-free travel allowances to an employee or to you,
they are reported to Tulorekisteri, the Finnish Incomes Register. aggountant
reports mileage allowances and per diems for one income earner and one
payment at a time, over the real Tulorekisteri production connection. The
first part of this page is a one-time setup.

Cash wages cannot yet be paid or reported with aggountant
([What is ready and what is not](../reference/status.md#payroll)).

## Before you start

- **API permission.** In the Tax Administration's certificate service, apply
  with a replacement application for your production certificate to also
  cover the Tulorekisteri wage reporting interface (purpose: reporting income
  I pay myself). Keep the Vero API selected if you use it. The same
  certificate works for both once the permission is granted. A separate
  certificate is also possible.
- **Credentials.** By default the same credential file is used as for the VAT
  return ([Set up direct VAT filing](set-up-vero-api.md)). The path of a
  separate file is given in the profile. The credential file must be outside
  the home.
- **The travel or expense claim** from which the allowance was paid, and the
  payment date.

## Set up reporting (once)

1. **Ask the agent to write the Tulorekisteri profile.**

   > Let's set up Tulorekisteri travel allowance reports. The API permission
   > has been granted.

   The agent writes the home file `.aggountant/tulorekisteri.json` with
   production mode, the Business ID and your confirmation that the reporting
   permission has been granted. There is no command for this file, so the
   agent writes it as the `vero-fi` skill instructs. **Confirm the permission
   only once it has been granted**, not when the application has been sent.

2. **Add the income earner to the employee register.** The income earner's
   details are an identifier, name, personal identity code (henkilötunnus),
   bank account number and the withholding percentage from their tax card.
   The withholding percentage is currently required even for a person who is
   only reported travel allowances. It does not affect the travel allowance
   report.

   **Do not type the personal identity code into the conversation with the
   agent.** Write the input to a file yourself, give the agent only the file
   name, and delete the file after the person has been added. There is not yet
   a safe way of entering the details that would keep them off the screen.

## Make a report

1. **Ask the agent.**

   > Report the allowances from Maija Meikäläinen's October travel claim to
   > Tulorekisteri. The payment date was 24 October 2026.

2. **The agent prepares the report** from the travel or expense claim. There
   are two kinds of row:
   - **311 mileage allowance** (kilometrikorvaus): amount and total kilometres
   - **331 per diem** (päiväraha): amount and the kind of per diem (1 meal
     allowance, 2 partial per diem, 3 full per diem, 4 foreign per diem).

   Receipt-based costs, such as a hotel or parking, do not belong on these
   rows. The report also has the payment date, the start and end of the
   earning period, whether the earner is a shareholder (pääomaosakkuus) and a
   contact person. The agent does not write the personal identity code into
   the report; the program takes it from the employee register.

3. **The agent previews the report.** The preview makes no network
   connection. It shows the rows, the period, the payment date, the
   shareholder information, the contact person and the deadline. aggountant
   uses a cautious deadline: the payment date plus 5 calendar days. It also
   gives the report's checksum.

4. **Check and approve** the rows, the period, the shareholder information and
   the checksum. Shareholding is always stated explicitly. aggountant does not
   infer it.

   > I approve the report.

5. **The agent sends the report** with exactly the checksum you approved.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant employee add < employee.json
aggountant employee list
aggountant file-wage preview --timestamp 2026-10-25T12:00:00Z < report.json
aggountant file-wage start   --timestamp 2026-10-25T12:00:00Z < report.json
aggountant file-wage approve wr-… --record-hash <checksum>
```

In the employee input the identifier is `employee_id` and the withholding
percentage `tax_card.withholding_pct`. The preview gives the checksum in the
field `record_hash`. The preview and the start get the same timestamp.
`aggountant employee show --employee <identifier>` shows the personal identity
code and bank account number in plain text, so the agent does not run it
needlessly.
</details>

## Done when

The agent tells you that Tulorekisteri has received the report. You can also
check it in Tulorekisteri's e-service.

## Limitations

- The employer's separate report, replacement reports and cancellations are
  not made with aggountant. They are made in Tulorekisteri's e-service.
- The employee register is not yet encrypted, even though the command's help
  says so. Keep the home directory and its backups for your own use only.

More detail: [What is ready and what is not](../reference/status.md#tulorekisteri-and-employees).

## If something goes wrong

- **The result of sending was left unclear** (a timeout, a service error):
  repeating the approval does not send again.
  [When something goes wrong](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain).
- **Tulorekisteri rejected the report, or it needs correcting.** Do it in the
  e-service:
  [When something goes wrong](../troubleshooting.md#tulorekisteri-rejected-the-report).
- **Do not change the payment identifier or the payment date** to get around
  a rejection or an uncertainty. That would quietly create a new report.

## See also

- [Filings and uncertain outcomes](../concepts/filings.md)
- [The company home: what is stored where](../concepts/company-home.md)
