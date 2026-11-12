# Result: fiscal years that do not follow the calendar year

## What changed

A company can now define its fiscal year:

```sh
aggountant fiscal-year define --start 2026-07-01 --end 2027-06-30
aggountant fiscal-year list
aggountant fiscal-year show --year 2027        # or --date 2027-03-10
```

A defined year runs from the first day of a month to the last day of a month
and is at most 18 months long (kirjanpitolaki 1:4, cited from Finlex in
`crates/ledger/AGENTS.md`). Later years continue as 12-month years. Without a
definition every fiscal year is a calendar year, which is how every existing
home keeps reading. A fiscal year is identified by the calendar year it ends
in: 1.7.2026–30.6.2027 is fiscal year 2027.

Vouchers, year-to-date figures in the monthly report (the section title is now
"Tuloslaskelma, tilikauden alusta"), the year's close coverage (its own months),
opening balances from the previous fiscal year, adjustment attribution,
close-year dates and the statements (which state the dates of a non-calendar
year) all follow the defined year. VAT periods, monthly periods and the payroll
tax year are unchanged.

Refusals added (all audited in the rejection log where they concern recorded
state):

- `INVALID_FISCAL_YEAR`: a year longer than 18 months, not starting/ending on
  month boundaries, overlapping or not following the existing definitions, or
  two fiscal years ending in one calendar year.
- `FISCAL_YEAR_UNDEFINED`: a voucher dated before the first defined fiscal year
  (when that year does not start on 1 January), or an attribution to a label
  that names no fiscal year.
- `FISCAL_YEAR_CONFLICT`: a definition that would move a sealed voucher to
  another fiscal year, change or remove a year with a year-state row or
  attributed adjustments, or strand active quarter coverage.
- `PERIOD_TRANSITION_CONFLICT`: closing a quarter in a calendar year that is not
  wholly one calendar fiscal year (non-calendar years close by month).
- `CLOSE_YEAR_BLOCKED` (with `DEPRECIATION` in the message): close-year with
  assets in a fiscal year that is not 12 months; depreciation rates are annual
  and are not prorated.

## Acceptance criteria

- [x] A synthetic company with a fiscal year 1.7.–30.6. books, closes months,
  sees year-to-date figures for its own year and records opening balances:
  `crates/cli/tests/fiscal_year_cli.rs::a_july_to_june_company_books_closes_reports_and_opens`
  (mid-year move with an opening voucher, monthly closes, YTD from 1.7.), plus
  `crates/ledger/tests/fiscal_year_definition.rs` (opening balance of the next
  year from the sealed July year, close coverage July–June) and
  `crates/process/tests/close_year.rs::a_july_to_june_year_closes_into_july`.
- [x] A first fiscal year of up to 18 months is accepted; a longer one is
  rejected citing the rule: `first_year_length_follows_kirjanpitolaki` (CLI) and
  `an_eighteen_month_first_year_is_accepted_and_a_longer_one_refused` (ledger).
- [x] Existing calendar-year homes keep working without user steps: fiscal store
  migration v7 runs on open and adds an empty table;
  `a_home_written_by_the_main_binary_keeps_calendar_years` opens a committed home
  written by the `main` binary (`crates/cli/tests/fixtures/main-home-calendar-2026`),
  verifies, seals, closes and reports. Calendar-year statement bytes are
  unchanged (the new `period` field is omitted for calendar years).
- [x] The user guide drops the calendar-year limitation: the exact fi and en
  changes are below for the conductor to apply (this worker did not edit
  `docs/user/`).

## User guide changes to apply (fi and en)

### `docs/user/en/concepts/periods.md`, section "The fiscal year and periods"

Replace the first paragraph:

> aggountant treats the fiscal year as the calendar year. The year is closed
> period by period, either monthly (`2026-03`) or quarterly (`2026-Q1`), but
> you cannot use both in the same year. Monthly is the recommended and tried
> option. A fiscal year other than the calendar year is not yet supported.

with:

> The fiscal year is the calendar year unless you define another one. A
> company whose fiscal year runs, for example, from 1 July to 30 June defines
> it once when the books are set up, and later years follow it. A first fiscal
> year, or the year in which the fiscal year changes, can be shorter or longer
> than 12 months, at most 18 months (Accounting Act 1:4). A fiscal year is
> named by the calendar year it ends in: 1.7.2026–30.6.2027 is fiscal year
> 2027. Two fiscal years cannot end in the same calendar year; if you change
> your fiscal year, use one longer transition year instead.
>
> The year is closed period by period. A calendar fiscal year is closed either
> monthly (`2026-03`) or quarterly (`2026-Q1`), but you cannot use both in the
> same year; monthly is the recommended and tried option. A fiscal year that is
> not the calendar year is always closed monthly.

Section "Fiscal-year status": after "A fiscal year's state is `open` or
`sealed`." add: "`aggountant fiscal-year show` also shows the year's first and
last day and its length in months."

### `docs/user/fi/concepts/periods.md`, section "Tilikausi ja kaudet"

Replace the first paragraph:

> aggountant käsittelee tilikauden kalenterivuotena. Vuosi suljetaan
> kausittain, joko kuukausittain (`2026-03`) tai neljännesvuosittain
> (`2026-Q1`), mutta samana vuonna ei voi käyttää molempia. Kuukausi on
> suositeltu ja koeteltu vaihtoehto. Kalenterivuodesta poikkeavaa tilikautta ei
> vielä tueta.

with:

> Tilikausi on kalenterivuosi, ellei sille määritellä muuta. Yritys, jonka
> tilikausi on esimerkiksi 1.7.–30.6., määrittelee sen kerran kirjanpidon
> käyttöönotossa, ja seuraavat tilikaudet jatkuvat sen mukaan. Ensimmäinen
> tilikausi tai tilikausi, jona tilikautta muutetaan, voi olla 12 kuukautta
> lyhyempi tai pidempi, kuitenkin enintään 18 kuukautta (kirjanpitolaki 1:4).
> Tilikausi nimetään sen kalenterivuoden mukaan, jona se päättyy:
> 1.7.2026–30.6.2027 on tilikausi 2027. Kaksi tilikautta ei voi päättyä samana
> kalenterivuonna; jos muutat tilikautta, käytä yhtä pidempää siirtymätilikautta.
>
> Vuosi suljetaan kausittain. Kalenterivuoden tilikausi suljetaan joko
> kuukausittain (`2026-03`) tai neljännesvuosittain (`2026-Q1`), mutta samana
> vuonna ei voi käyttää molempia; kuukausi on suositeltu ja koeteltu
> vaihtoehto. Kalenterivuodesta poikkeava tilikausi suljetaan aina
> kuukausittain.

Section "Tilikauden tila": after "Tilikauden tila on `open` tai `sealed`." add:
"`aggountant fiscal-year show` näyttää myös tilikauden ensimmäisen ja viimeisen
päivän ja pituuden kuukausina."

### `docs/user/en/reference/status.md`, "Fiscal year and periods"

Replace

> - **The fiscal year is always the calendar year.** Other fiscal years are not
>   yet supported (`contra-accounts-fiscal-periods`).

with

> - **A fiscal year other than the calendar year is closed monthly.** Quarterly
>   closing is available only for calendar fiscal years.
> - **Two fiscal years cannot end in the same calendar year.** A change of
>   fiscal year is made with one transition year of at most 18 months.
> - **Depreciation is not calculated for a fiscal year that is not 12 months
>   long.** The year-end run refuses it; record that year's depreciation as an
>   ordinary voucher.
> - **A fiscal-year definition cannot be changed.** It can only be followed by a
>   new one, and not in a way that would move recorded vouchers to another
>   fiscal year.

### `docs/user/fi/reference/status.md`, "Tilikausi ja kaudet"

Replace

> - **Tilikausi on aina kalenterivuosi.** Muuta tilikautta ei vielä tueta
>   (`contra-accounts-fiscal-periods`).

with

> - **Kalenterivuodesta poikkeava tilikausi suljetaan kuukausittain.**
>   Neljännesvuosisulku on käytettävissä vain kalenterivuoden tilikaudelle.
> - **Kaksi tilikautta ei voi päättyä samana kalenterivuonna.** Tilikauden
>   muutos tehdään yhdellä enintään 18 kuukauden siirtymätilikaudella.
> - **Poistoja ei lasketa tilikaudelle, joka ei ole 12 kuukautta.**
>   Tilinpäätösajo kieltäytyy niistä; kirjaa sen tilikauden poistot tavallisena
>   tositteena.
> - **Tilikauden määrittelyä ei voi muuttaa.** Sen perään voi määritellä uuden,
>   mutta ei niin, että jo kirjattuja tositteita siirtyisi toiselle
>   tilikaudelle.

### `docs/user/en/reference/glossary.md`, "Fiscal year (tilikausi)"

Replace "In aggountant the fiscal year is the calendar year." with "It is
usually 12 months; a first fiscal year or the year in which it changes can be
at most 18 months. In aggountant it is the calendar year unless another fiscal
year is defined, and a fiscal year is named by the calendar year it ends in."

### `docs/user/fi/reference/glossary.md`, "Tilikausi"

Replace "aggountantissa tilikausi on kalenterivuosi." with "Se on yleensä 12
kuukautta; ensimmäinen tilikausi tai tilikausi, jona sitä muutetaan, voi olla
enintään 18 kuukautta. aggountantissa tilikausi on kalenterivuosi, ellei muuta
tilikautta määritellä, ja tilikausi nimetään sen kalenterivuoden mukaan, jona
se päättyy."

### `docs/user/en/start/is-it-for-you.md`

Replace "- your company is a small Finnish limited company whose fiscal year is
the calendar year" with "- your company is a small Finnish limited company".

### `docs/user/fi/start/is-it-for-you.md`

Replace "- yrityksesi on pieni suomalainen osakeyhtiö, jonka tilikausi on
kalenterivuosi" with "- yrityksesi on pieni suomalainen osakeyhtiö".

### `docs/user/en/start/set-up-company.md`

In "Before you start", delete the bullet "Your company's fiscal year is the
calendar year. Other fiscal years are not yet supported (…)."

In step 3, after "It saves your answers with their sources in the company
knowledge store." add: "If the fiscal year is not the calendar year, the agent
also records it in the books (`fiscal-year define`) before the first voucher.
It defines the fiscal year the first voucher falls in; later years follow on
their own."

In step 4, after the bullet about the voucher date, add: "If the company moves
to aggountant at the start of a fiscal year that does not begin on 1 January,
the opening balances are dated the first day of that fiscal year, because the
books have no fiscal year before it. At the start of a fiscal year the balances
contain only balance-sheet accounts, so this does not move any result."

In "Commands the agent runs", after `aggountant knowledge company add < fact.json`
add `aggountant fiscal-year define --start 2026-07-01 --end 2027-06-30`.

### `docs/user/fi/start/set-up-company.md`

In "Ennen kuin aloitat", delete the bullet "Yrityksesi tilikausi on
kalenterivuosi. Muuta tilikautta ei vielä tueta (…)."

In step 3, after the sentence on saving the answers to the company knowledge
store add: "Jos tilikausi ei ole kalenterivuosi, agentti kirjaa sen myös
kirjanpitoon (`fiscal-year define`) ennen ensimmäistä tositetta. Se määrittelee
tilikauden, johon ensimmäinen tosite osuu; seuraavat tilikaudet jatkuvat
itsestään."

In step 4, after the bullet about the voucher date, add: "Jos yritys siirtyy
aggountantiin sellaisen tilikauden alussa, joka ei ala 1.1., avaavat saldot
päivätään tilikauden ensimmäiselle päivälle, koska kirjanpidossa ei ole sitä
edeltävää tilikautta. Tilikauden alussa saldoissa on vain tasetilejä, joten
tämä ei siirrä tulosta."

In "Komennot, jotka agentti ajaa", after `aggountant knowledge company add <
fact.json` add `aggountant fiscal-year define --start 2026-07-01 --end
2027-06-30`.

### `docs/user/{en,fi}/reference/commands.md`, row `fiscal-year`

en: "a fiscal year's dates and state (`open`, `sealed`); defining a fiscal year
that is not the calendar year". fi: "tilikauden päivät ja tila (`open`,
`sealed`); kalenterivuodesta poikkeavan tilikauden määrittely".

### `docs/user/{en,fi}/reference/error-codes.md`, add rows (alphabetical)

en:

| `FISCAL_YEAR_CONFLICT` | a fiscal-year definition would move recorded vouchers or a closed year to another fiscal year | read the message; a definition cannot change what has already been recorded |
| `FISCAL_YEAR_UNDEFINED` | the date is before the company's first fiscal year, or no fiscal year ends in the given year | check the date or the year; `fiscal-year list` shows the definitions |
| `INVALID_FISCAL_YEAR` | the fiscal year is longer than 18 months, does not start on the first or end on the last day of a month, or does not follow the earlier fiscal years | correct the dates (Accounting Act 1:4) |

fi:

| `FISCAL_YEAR_CONFLICT` | tilikauden määrittely siirtäisi kirjattuja tositteita tai suljetun tilikauden toiselle tilikaudelle | lue viesti; määrittely ei voi muuttaa jo kirjattua |
| `FISCAL_YEAR_UNDEFINED` | päivä on ennen yrityksen ensimmäistä tilikautta, tai annettuna vuonna ei pääty tilikautta | tarkista päivä tai vuosi; `fiscal-year list` näyttää määrittelyt |
| `INVALID_FISCAL_YEAR` | tilikausi on yli 18 kuukautta, ei ala kuukauden ensimmäisenä tai pääty viimeisenä päivänä, tai ei jatka aiempia tilikausia | korjaa päivät (kirjanpitolaki 1:4) |

Also extend the `PERIOD_TRANSITION_CONFLICT` row: en "…, or months and quarters
are being mixed in the same year, or a quarter is closed in a fiscal year that
is not the calendar year"; fi "…, tai samana vuonna yritetään sekoittaa
kuukausia ja neljänneksiä tai sulkea neljännestä tilikaudella, joka ei ole
kalenterivuosi".

### `docs/user/{en,fi}/tasks/year-end.md`, step 2

en: "The report has the balance sheet at the end of December and the income
statement since the start of the year." becomes "The report has the balance
sheet at the end of the fiscal year's last month and the income statement
since the start of the fiscal year." (and "December's report" becomes "the
report for the fiscal year's last month"). fi: "Raportissa on joulukuun lopun
tase ja tuloslaskelma vuoden alusta." becomes "Raportissa on tilikauden
viimeisen kuukauden lopun tase ja tuloslaskelma tilikauden alusta."

## Decisions for later review

- **Storage in the fiscal store, not git masterdata.** The issue says master
  data; the definition decides which sealed vouchers a sealed year covers, so it
  is written under the seal's writer lock, append-only, and checked against
  recorded state. A hand-editable plaintext file could silently re-attribute
  vouchers.
- **Ending-year label, `i32` kept.** No fiscal-year id type through storage and
  wire; existing rows keep their meaning. Cost: two fiscal years ending in one
  calendar year are refused (an 18-month transition year is the way round).
- **Before the first definition**: calendar years if it starts on 1 January
  (a calendar company changing), otherwise no fiscal year, and a voucher there
  is refused at seal.
- **No `init` flags.** `init` stays flag-free; the setup flow collects the
  fiscal year after `init`, so `fiscal-year define` is the one write path.
- **Non-calendar years close by month only.** A quarterly-VAT company with a
  non-calendar year therefore cannot close by quarter; supporting that needs the
  per-calendar-year month/quarter mixing rule rethought.
- **Depreciation refused for years that are not 12 months**, instead of a
  proration rule the program would have to guess.
- **Report section title** "Tuloslaskelma, vuoden alusta" became "Tuloslaskelma,
  tilikauden alusta", which is correct for every company.
- **`skills/aggountant-init/SKILL.md` edited** (outside the files the brief
  named): without it the setup agent would never define the fiscal year. The
  edit adds the `fiscal-year define` step and the boundary-start dating rule,
  and changes "vuoden alusta" to "tilikauden alusta". Revert if the conductor
  prefers to own skill text.
- **Close-year depreciation errors at start are `CLOSE_YEAR_BLOCKED`** (caller
  actionable) instead of `PROCESS_STORE`; this also applies to an unknown asset
  class, which was misreported as a system error before.

## Review

`review.md` (four confirmed must-fix findings, each reproduced by a failing test
and fixed) and `assessment.md` are beside this file.

## Filed

- `opening-voucher-year-boundary-blocks-reports` (intake, untriaged): a
  calendar-year company starting at a year boundary and following the guide's
  "date the opening voucher on the old system's last day" cannot get any report
  for its first year (`PERIOD_NOT_SEALED` for the never-kept previous year).
  Reproduced on `main`; pre-existing, not caused by this change.

## Verification

- `scripts/ci-local.sh`: all gates passed (fmt, clippy, cargo-deny, build and
  test on stable and MSRV 1.89, user-guide gates, practice month). cargo-deny
  printed only its existing `license-not-encountered` and `duplicate` warnings.
- `issuectl doctor --json`: no findings, no warnings.
