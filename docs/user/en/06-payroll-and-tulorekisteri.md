# 6. Payroll and Tulorekisteri

Tulorekisteri is the Finnish Incomes Register, the national register to which
employers report wages, fees and allowances.

## What you can do today

With aggountant you can report **tax-free travel allowances** (mileage
allowances and per diems) to Tulorekisteri, for one income earner at a time
(`file-wage`). This works over the real Tulorekisteri production connection.

**Cash wages cannot yet be paid or reported with aggountant.**
The payroll run (`run-payroll`) is in the program, but its contribution tables
(earnings-related pension, health insurance and unemployment insurance
contributions) are provisional values that have not been checked against
official sources. Tax card handling is simplified, and there is no real
payment or Tulorekisteri connection behind the run. Do not use it for real
wages. For now, calculate, pay and report wages with a payroll service or in
Tulorekisteri's e-service, and book the payroll voucher in aggountant as a
special entry (chapter 4).

## Employee details

The income earner's personal data is stored in the home's personal data store
(`masterdata/pii.sqlite`), which is excluded from git:

```sh
aggountant employee add < employee.json
aggountant employee list
aggountant employee show --employee employee-1
```

The input contains an identifier (`employee_id`), the name, the Finnish
personal identity code (henkilötunnus), the bank account number, and the tax
card's withholding percentage (`tax_card.withholding_pct`). The withholding
percentage is currently required even for a person who is only reported travel
allowances. It has no effect on the travel allowance report.

Keep these limitations in mind:

- **The store is not encrypted yet.** The command's help describes an encrypted
  store, but in this version the data is stored in plain text. The only
  protection is that the file is kept out of git. Keep the home directory and
  its backups for your own use only.
- **`employee show` displays the personal identity code and the bank account
  number in plain text.**
- **Do not type the personal identity code into the conversation with the
  agent.** Prepare the input yourself in a file or on the command line, and
  delete the file afterwards. There is no safe way of entering it yet that
  would keep it off the screen.

## Tax-free travel allowances to Tulorekisteri

A report covers one income earner and one payment, and it can contain two kinds
of line:

- **311 mileage allowance**: the amount and the total kilometres
- **331 per diem**: the amount and the type of per diem (1 meal allowance,
  2 partial per diem, 3 full per diem, 4 foreign per diem).

Expenses reimbursed against receipts, such as a hotel or parking, do not belong
on these lines. The employer's separate report, replacement reports and
cancellations are not made through this route. They are made in
Tulorekisteri's e-service.

### Onboarding

1. **Interface permission.** In the Tax Administration's certificate service,
   use a replacement application to add the Tulorekisteri wage-reporting
   interface to your production certificate (purpose: I report income I have
   paid myself). Keep Vero API selected if you use it. Once the permission is
   granted, the same certificate works for both. A separate certificate is also
   possible.
2. **Profile.** The home's file `.aggountant/tulorekisteri.json` holds the
   production mode, the Business ID and your confirmation that the reporting
   permission has been granted. There is no command for this file, so the
   agent writes it following the `vero-fi` skill. Confirm the permission only
   once it has been granted, not when the application has been sent.
3. **Credentials.** By default the same credential file is used as for the ALV
   return (chapter 5). A separate file can be named in the profile. The
   credential file must be outside the home.
4. **The income earner** is added to the employee register (above).

The home's `.aggountant` directory also marks the home: when you work in the
home directory or one of its subdirectories, the program finds the home without
the `--home` option (chapter 2).

### Making the report

The agent prepares the report from an expense claim or travel claim. The input
contains the Business ID, a permanent identifier for the payment, the income
earner's identifier, the payment date, the start and end of the earning period,
whether the earner is a part-owner of the company, a contact person and the
lines. The personal identity code is not in the input; the program fetches it
from the employee register.

```sh
aggountant file-wage preview --timestamp 2026-10-25T12:00:00Z < report.json
aggountant file-wage start   --timestamp 2026-10-25T12:00:00Z < report.json
```

The preview and the start open no network connection. Both are given the same
timestamp. The preview shows the lines, the period, the payment date, the
ownership information, the contact person and the due date. aggountant uses a
cautious due date: the payment date plus 5 calendar days. It also gives the
report's checksum (`record_hash`).

**You approve** the lines, the period, the ownership information and the
checksum. Only then does the agent send it:

```sh
aggountant file-wage approve wr-… --record-hash <checksum>
```

The checksum is also derived from the personal identity code, so it is treated
as sensitive: it is not stored in git, in notes or in a conversation log.

Never change the payment identifier or the payment date to get around a
rejection or an uncertain outcome. That would silently create a new report.

### Uncertain outcome and the fallback route

If the outcome of a submission is unclear (a timeout, a service error or a
contradictory answer), repeating the approve command **does not send again**.
The agent asks Tulorekisteri for the status:

```sh
aggountant file-wage reconcile wr-…
```

If the report is still being processed, ask again later. If Tulorekisteri has
rejected the report, the run stays in a state that needs attention. You can see
the exact errors in Tulorekisteri's e-service, because the program does not
show the authority's error text, which may contain the personal identity code.

When the interface route does not work, or a report needs correcting, make the
report in Tulorekisteri's e-service. **First check whether the uncertain
interface submission has already arrived**, so you do not make a second
original report. Then record the identifiers the e-service gives you in
aggountant:

```sh
aggountant file-wage complete-manual wr-… \
  --ir-delivery-id <delivery identifier> --ir-report-id <report identifier>
```
