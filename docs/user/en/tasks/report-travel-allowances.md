# Report travel allowances to Tulorekisteri

When the company pays tax-free travel allowances to an employee or to you,
they are reported to Tulorekisteri, the Finnish Incomes Register. aggountant
reports mileage allowances and per diems for one income earner and one
payment at a time, over the real Tulorekisteri production connection. The
first part of this page is a one-time setup.

Cash wages cannot yet be paid or reported with aggountant
([What is ready and what is not](../reference/status.md#payroll)).

If the company is in the employer register, an employer's separate report also
goes to Tulorekisteri every month. For a month in which you paid only travel
allowances it gives a health insurance contribution of €0:
[File the employer's separate report](file-employer-report.md).

## Before you start

- **API permission.** In the Tax Administration's certificate service, open
  Tuotantovarmenteet → your organisation → Näytä tiedot → Tee korvaava
  hakemus. Of the Tulorekisteri interfaces (Palkkatiedot and Etuustiedot, each
  as SFTP or Web service) choose **Palkkatiedot (Web service)**, purpose
  "ilmoitan itse maksamiani tuloja". Keep Vero API ticked if you use it. Once
  granted, the confirmation lists "Palkkatiedot tuottaja (Web service)" (and
  "Vero API (Web service)" if you kept it). The application can be approved
  at once, and no new certificate is needed: the same one works for both. A
  separate certificate is also possible.
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

   With `tulorekisteri configure` the agent writes the home file
   `.aggountant/tulorekisteri.json` with production mode, the Business ID and
   your confirmation that the reporting permission has been granted.
   **Confirm the permission only once it has been granted**, not when the
   application has been sent. Production mode is written only once the
   channel is ready; otherwise the command lists what is missing.
   `tulorekisteri status` shows the same offline later: the code of each
   missing requirement, whether it is yours (`human`) or the agent's
   (`agent`), and what to do.

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
aggountant --json file-wage preview --timestamp 2026-10-25T12:00:00Z < report.json \
  | jq -r .record_hash | aggountant file-wage approve wr-… --record-hash-file -
aggountant run show --run-id wr-…
```

In the employee input the identifier is `employee_id` and the withholding
percentage `tax_card.withholding_pct`. The preview gives the checksum in the
field `record_hash`. The checksum is derived from the personal identity code,
so it is sensitive: it is used only for the approval and is not stored, logged
or shared. The preview says the same in the field `sensitive.record_hash`.
That is why the approval reads it on stdin (`--record-hash-file -`), piped
from the same preview, instead of on the command line, where it would end up
in shell history and in the conversation.
The preview and the start get the same timestamp.
`aggountant employee show --employee <identifier>` shows the personal identity
code and bank account number in plain text, so the agent does not run it
needlessly.

Once Tulorekisteri has accepted the report, the output of `file-wage approve`
and `run show` carries its receipt on the `submit-wage` effect:
`outcome.receipt.ir_delivery_id` and `outcome.receipt.ir_report_id`.
`run show` reads it offline. If `outcome.state` is `outcome_unknown`,
`outcome.next_command` names `file-wage reconcile`, which asks Tulorekisteri.
</details>

## Done when

The agent tells you that Tulorekisteri has received the report. You can also
check it in Tulorekisteri's e-service.

## Correct an accepted report

When Tulorekisteri has already accepted a report and it has an error, the
correction is made with aggountant the same way as the report: the agent
previews the correction, you approve it and the agent sends it once.

> The kilometres in October's travel claim were wrong; the right figure is
> 160 km and the allowance €96.00. Correct the Tulorekisteri report.

There are two kinds of correction:

- **A replacement report** gives the whole report again, corrected. It
  corrects the lines, the contact person or the joint ownership detail. It
  cannot change the payer, the income earner, the payment date or the earning
  period, because Tulorekisteri requires them to stay the same.
- **A cancellation** removes the report. It is used when there was no payment
  after all, or when something must change that a replacement may not change.
  A new report is then made in the usual way if one is needed.

The preview shows the report Tulorekisteri holds now and the corrected report
side by side. Check that exactly what should change does change, and approve
only then. A correction is as final as the original report.

<details>
<summary>The commands the agent runs</summary>

```sh
aggountant file-wage correction preview wr-… --action replace --timestamp 2026-11-02T08:00:00Z < corrected.json
aggountant file-wage correction submit  wr-… --action replace --timestamp 2026-11-02T08:00:00Z \
  --record-hash-file <(aggountant --json file-wage correction preview wr-… --action replace \
  --timestamp 2026-11-02T08:00:00Z < corrected.json | jq -r .record_hash) < corrected.json
aggountant file-wage correction preview wr-… --action cancel --timestamp 2026-11-02T08:00:00Z
aggountant file-wage correction status  wr-…
```

`wr-…` is the run of the original report. The input of a replacement is the
whole report, corrected, and its checksum is sensitive like the original's.
Since stdin already carries the report, the checksum comes in through process
substitution (`--record-hash-file <(…)`). A cancellation reads no input. `correction status` shows what Tulorekisteri
holds now (`accepted`, `replaced`, `cancelled`), the receipt in force and the
report's whole history. `run show` of the original run shows the state
`superseded` once its receipt is no longer the one in force. After a
cancellation a new report needs its own `source_reference`.

If you have already made the replacement or the cancellation in Tulorekisteri's
e-service, the agent records it from the evidence:

```sh
aggountant file-wage correction complete-manual wr-… --action replace --ir-report-id <report reference> --report-version 2
```
</details>

## Limitations

- A replacement report cannot change the payer, the income earner, the payment
  date or the earning period. For those the report is cancelled and made again.
- The employee register is not yet encrypted. Keep the home directory and its
  backups for your own use only.

More detail: [What is ready and what is not](../reference/status.md#tulorekisteri-and-employees).

## If something goes wrong

- **The result of sending was left unclear** (a timeout, a service error):
  repeating the approval does not send again.
  [When something goes wrong](../troubleshooting.md#the-outcome-of-a-filing-is-uncertain).
- **Tulorekisteri rejected the report.** See the reason in the e-service:
  [When something goes wrong](../troubleshooting.md#tulorekisteri-rejected-the-report).
- **The result of a correction was left unclear.** `file-wage correction
  status` asks for the status; the correction is not sent again, and no new
  correction can be made before it is settled.
- **Do not change the payment identifier or the payment date** to get around
  a rejection or an uncertainty. That would quietly create a new report.

## See also

- [File the employer's separate report](file-employer-report.md)
- [Filings and uncertain outcomes](../concepts/filings.md)
- [The company home: what is stored where](../concepts/company-home.md)
