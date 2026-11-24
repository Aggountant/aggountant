# Report travel allowances to Tulorekisteri

When the company pays tax-free travel allowances to an employee or to you,
they are reported to Tulorekisteri, the Finnish Incomes Register. aggountant
reports mileage allowances and per diems for one income earner and one
payment at a time, over the Tulorekisteri production connection. Cash wages
[cannot yet be paid or reported](../reference/status.md#payroll) with it.

If the company is in the employer register, it also files
[the employer's separate report](file-employer-report.md) with Tulorekisteri
every month. For a month in which you paid only travel allowances, that
report gives a health insurance contribution of €0.

## Before you start

- The company has been granted the Tulorekisteri API permission. Apply for it
  in the Tax Administration's certificate service: Tuotantovarmenteet → your
  organisation → Näytä tiedot → Tee korvaava hakemus. The Tulorekisteri
  interfaces are Palkkatiedot and Etuustiedot, each as SFTP or Web service.
  Choose **Palkkatiedot (Web service)** and the purpose "ilmoitan itse
  maksamiani tuloja". Keep Vero API ticked if you use it. You do not need a new certificate, because the same
  one works for both, but you can also use a separate one. Once the
  permission is granted, the confirmation lists "Palkkatiedot tuottaja (Web
  service)" and, if you kept Vero API, "Vero API (Web service)".
- The credential file is outside the home. By default aggountant uses the
  same file as for [direct VAT filing](set-up-vero-api.md). If you want a
  separate file for Tulorekisteri, the agent writes its path to the profile.
- You have the travel or expense claim from which the allowance was paid, and
  you know the payment date.

## Set up reporting (once)

1. **Ask the agent to write the Tulorekisteri profile.**

   > Let's set up Tulorekisteri travel allowance reports. The API permission
   > has been granted.

   The agent writes production mode, the company's Business ID and your
   confirmation that the reporting permission has been granted to the
   profile. Confirm the permission only once it has been granted. Having sent
   the application is not enough.

   The program writes production mode only once the connection is ready.
   Otherwise it lists what is missing: the code of each missing requirement,
   whether it is yours (`human`) or the agent's (`agent`), and what to do.

2. **Write the income earner's details to a file**, and the agent adds them
   to the employee register. The details are an identifier, name, personal
   identity code (henkilötunnus), bank account number and the withholding
   percentage from their tax card. The withholding percentage is currently
   required even for a person who is only reported travel allowances, although
   it does not affect the travel allowance report.

   **Do not type the personal identity code into the conversation with the
   agent.** Give the agent only the file name, and delete the file once the
   income earner has been added. There is not yet a way of entering the
   details that keeps them off the screen.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant tulorekisteri configure < profile.json
aggountant tulorekisteri status
aggountant employee add < employee.json
aggountant employee list
```

`tulorekisteri configure` writes the profile to the home file
`.aggountant/tulorekisteri.json`.

`tulorekisteri status` shows what the connection still lacks, later too and
without a network connection.

In the employee input the identifier is `employee_id` and the withholding
percentage `tax_card.withholding_pct`.

`aggountant employee show --employee <identifier>` shows the personal identity
code and bank account number in plain text, so the agent does not run it
needlessly.
</details>

## Make a report

1. **Ask the agent.**

   > Report the allowances from Maija Meikäläinen's October travel claim to
   > Tulorekisteri. The payment date was 24 October 2026.

2. **The agent prepares the report** from the travel or expense claim. A
   mileage allowance (kilometrikorvaus) row (311) has the amount and the total
   kilometres. A per diem (päiväraha) row (331) has the amount and the kind of
   per diem: 1 meal allowance, 2 partial per diem, 3 full per diem or
   4 foreign per diem. Receipt-based costs, such as a hotel or parking, do not
   belong on these rows.

   The report also has the payment date, the start and end of the earning
   period, the shareholding (pääomaosakkuus) and a contact person. In
   Tulorekisteri's e-service the shareholding is called "Osakasasemassa
   maksajaan" (joint owner with the payer). It is not the partial ownership
   (osaomistaja) of unemployment security law. aggountant infers neither the
   earning period nor the shareholding, so the agent asks you about the
   shareholding, and about the earning period if the travel claim does not
   show it. The
   agent does not write the personal identity code into the report; the
   program takes it from the employee register.

3. **The agent previews the report.** The preview makes no network
   connection. It shows the rows, the period, the payment date, the
   shareholding, the contact person and the deadline. As the deadline
   aggountant shows a cautious one, five calendar days after the payment date.

4. **Check and approve** the rows, the period, the payment date and the
   shareholding.

   > I approve the report.

5. **The agent sends the report.** The preview gives a checksum, and sending
   succeeds only with it, so exactly the report you saw goes to Tulorekisteri.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant --json file-wage preview --timestamp 2026-10-25T12:00:00Z < report.json \
  | tee >(jq -r .record_hash > HASH_FILE) | jq 'del(.record_hash)'
aggountant file-wage start   --timestamp 2026-10-25T12:00:00Z < report.json
aggountant file-wage approve wr-… --record-hash-file HASH_FILE
aggountant run show --run-id wr-…
```

The preview gives the checksum in the field `record_hash`. The checksum is
derived from the personal identity code, so it is sensitive: it is used only
for the approval and is not kept, logged or shared. The preview says the same
in the field `sensitive.record_hash`. On the command line the checksum would
end up in shell history and in the conversation. So the agent captures the
checksum of the preview you review into a private temporary file (made with
`mktemp`, outside the home) and shows you the preview without it.

`file-wage start` gets the same timestamp as the preview.

`file-wage approve` reads the checksum from the file (`--record-hash-file`),
and the agent deletes the file afterwards. Once Tulorekisteri has accepted
the report, the output carries its receipt on the `submit-wage` effect, in
`outcome.receipt.ir_delivery_id` and `outcome.receipt.ir_report_id`. If
`outcome.state` is `outcome_unknown`, `outcome.next_command` names
`file-wage reconcile`, which asks Tulorekisteri for the status.

`run show` shows the same receipt without a network connection.
</details>

## Done when

The agent tells you that Tulorekisteri has received the report. You can also
check it in Tulorekisteri's e-service.

## Correct an accepted report

When Tulorekisteri has already accepted a report and it has an error, correct
it with aggountant the same way you made the report: the agent previews the
correction, you approve it and the agent sends it once.

> The kilometres in October's travel claim were wrong; the right figure is
> 160 km and the allowance €96.00. Correct the Tulorekisteri report.

A replacement report gives the whole report again, corrected. It corrects the
rows, the contact person or the shareholding. It cannot change the payer, the
income earner, the payment date or the earning period, because Tulorekisteri
requires them to stay the same.

A cancellation removes the report. It is used when there was no payment after
all, or when something must change that a replacement cannot change. A new
report is then made in the usual way if one is needed.

The preview shows the report Tulorekisteri holds now and the corrected report
side by side. Before you approve, check that only what should change does
change. A correction is as final as the original report.

<details>
<summary>Commands the agent runs</summary>

```sh
aggountant file-wage correction preview wr-… --action replace --timestamp 2026-11-02T08:00:00Z < corrected.json
aggountant file-wage correction submit  wr-… --action replace --timestamp 2026-11-02T08:00:00Z --record-hash-file HASH_FILE < corrected.json
aggountant file-wage correction preview wr-… --action cancel --timestamp 2026-11-02T08:00:00Z
aggountant file-wage correction status  wr-…
```

`wr-…` is the run of the original report.

The input of a replacement is the whole report, corrected. Its checksum is
sensitive like the original's, so the agent captures it to a file the same
way.

A cancellation reads no input. After a cancellation a new report needs its
own `source_reference`.

`correction status` shows what Tulorekisteri holds now (`accepted`,
`replaced`, `cancelled`), the receipt in force and the report's whole
history. `run show` of the original run shows the state `superseded` once its
receipt is no longer the one in force.

If you have already made the replacement or the cancellation in Tulorekisteri's
e-service, the agent records it from the evidence:

```sh
aggountant file-wage correction complete-manual wr-… --action replace --ir-report-id <report reference> --report-version 2
```
</details>

## Limitations

The employee register is not yet encrypted. Keep the home directory and its
backups for your own use only. The other limitations are under
[Tulorekisteri and employees](../reference/status.md#tulorekisteri-and-employees).

## If something goes wrong

- If the result of sending was left unclear (a timeout, a service error),
  repeating the approval does not send the report again. See
  [The outcome of a filing is uncertain](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain).
- If Tulorekisteri rejected the report, see the reason in the e-service and
  read
  [Tulorekisteri rejected the report](../troubleshooting.md#tulorekisteri-rejected-the-report).
- If the result of a correction was left unclear, `file-wage correction
  status` asks Tulorekisteri for the status. The program does not send the
  correction again, and no new correction can be made before the status is
  settled.

Do not change the payment identifier or the payment date to get around a
rejection or an uncertainty, because that would quietly create a new report.

## See also

- [File the employer's separate report](file-employer-report.md)
- [Filings and uncertain outcomes](../concepts/filings.md)
- [The company home: what is stored where](../concepts/company-home.md)
