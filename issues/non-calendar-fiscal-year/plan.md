# Plan: fiscal years that do not follow the calendar year

## The law

Kirjanpitolaki (1336/1997) 1 luku 4 § (Tilikausi), Finlex
<https://www.finlex.fi/fi/lainsaadanto/1997/1336>, checked 2026-11-12:

> Tilikausi on 12 kuukautta. Toimintaa aloitettaessa tai lopetettaessa taikka
> tilinpäätöksen ajankohtaa muutettaessa tilikausi saa olla tätä lyhyempi tai
> pitempi, kuitenkin enintään 18 kuukautta.

The law sets no minimum length. aggountant accepts any defined year of
1–18 whole calendar months and refuses a longer one citing KPL 1:4. It cannot
verify the reason (start, end, change of timing), so it does not try.

## Identity of a fiscal year

The fiscal year stays an `i32` everywhere it is stored (year state, the
year-transition log, attribution `effective_year`, opening-balance sources,
statement `fiscal_year`, archive directory, close-year natural keys). The
integer becomes a *label*: the calendar year in which the fiscal year **ends**.
For a calendar year the label is the year itself, so every stored value in an
existing home keeps its meaning and nothing is migrated. Ending-year labelling
is also how Vero identifies a tilikausi (the verovuosi is the calendar year the
tilikausi ends in).

Labels must be unique and strictly increasing in time; then every existing
integer comparison (`effective_year <= posting_year`, the sealed-year interval
guard, `statement_year < year` for openings) keeps working unchanged. Two
fiscal years ending in the same calendar year (e.g. 1.7.2026–30.6.2027 followed
by a 6-month change year 1.7.2027–31.12.2027) cannot be labelled, so such a
definition is **refused** with a clear error that suggests the 18-month
alternative. This is the deliberate gap of this slice. Labels may skip a value
(an 18-month year), so "the previous fiscal year" is taken from the calendar,
never computed as `year - 1`.

## What stores the definition

A new fiscal-store table (migration v7) `fiscal_year_definition(start_date,
end_date, actor, recorded_at)`, written only by `Ledger::define_fiscal_year`
in a `BEGIN IMMEDIATE` transaction, append-only.

Why the fiscal store and not the git reference masterdata the issue names:
the definition decides which sealed vouchers a sealed year covers, so it must
be checked against the vouchers, periods and year rows in the same writer lock
that seals them (`crates/ledger/AGENTS.md`: fiscal SQLite is the single
authority for period state). A hand-editable plaintext file could silently
re-attribute sealed vouchers. A non-calendar year's start and end are also
bound into its statements (and so into the `SealYear` hash).

The calendar (`aggountant_model::FiscalCalendar`, pure) is derived from the
definitions:

- no definitions: calendar years (every existing home);
- a date inside a defined year: that year;
- after a defined year: 12-month years continue from its end until the next
  definition, whose start must fall on one of those boundaries;
- before the first definition: calendar years if it starts on 1 January
  (a calendar company switching); otherwise no fiscal year exists there
  (a company whose first year starts mid-year), and a voucher dated there is
  refused at seal (`FISCAL_YEAR_UNDEFINED`).

Each defined year starts on the first day of a month and ends on the last day
of a month, because it is closed month by month.

`define` refuses a definition that would change anything already recorded:
every sealed voucher keeps its label, every `fiscal_year` row keeps its exact
date range, and the new definition must follow every existing one. Repeating an
identical definition is an idempotent replay. Set it during setup with
`aggountant fiscal-year define --start YYYY-MM-DD --end YYYY-MM-DD`; `init`
stays flag-free because the setup flow collects the fiscal year after `init`
(user guide, step 3).

## Every place that derives a year from a date today

| Place | Today | Change |
| --- | --- | --- |
| `ledger::fiscal_year_data` | `PeriodId::containing(date).year()` per voucher; prior year `year - 1` | calendar `year_of`; prior from calendar |
| `ledger::year_vouchers` | calendar year of posting date | calendar `year_of` |
| `ledger::attribute_adjustment` | posting year = calendar year | calendar `year_of` |
| `ledger::redate_into_year` | re-dates to `year-12-31` | re-dates to the fiscal year's end |
| `ledger::year_close_periods` | months/quarters of calendar `year` | calendar year: unchanged; other: its months, refusing active quarter coverage in its range |
| store `transition_period` sealed-year guard | `year_of_period` = calendar year | every fiscal year the period overlaps, from the calendar read in the same transaction |
| store `commit` (seal) | no year check | refuse a date outside any fiscal year |
| `ledger::reports` monthly statements | `period.year()` as fiscal year | fiscal year containing the month; YTD from its start |
| `domain::compute_statements` / `tase_erittelyt` | filter `containing(date).year() != year` | filter by the fiscal year's date range; statements carry start/end for a non-calendar year |
| `domain` opening validation | model checks `live_period.year() == fiscal_year`, `prior + 1 == year` | model keeps `prior < year`; domain checks the live period is inside the year's range; ledger takes the prior label from the calendar |
| `domain::compute_depreciation` | resolves at given year-end date, annual rate | takes the fiscal year; refuses a year that is not 12 months (annual rates are not prorated by guesswork) |
| `process::close_year` | doc date `year-12-31`; target year `year + 1` with Jan/Q1; intake filter on `date[0..4]` | fiscal year end; next fiscal year's periods; intake by date range |
| CLI `fiscal-year show` | `year`, state | adds `start`, `end`, `months`; new `define` and `list` |

Not changed, because they are calendar-year by law or by design: monthly and
quarterly `PeriodId`s, VAT periods and `FilingYear`, payroll tax year and YTD
(calendar tax year), `run_payroll`/`hunt_receipts` quarter lookups, the
month/quarter mixing guard in `transition_period` (per calendar year; a mixed
non-calendar year is refused at `year_close_periods` instead).

## Deliberately left out

- Two fiscal years ending in the same calendar year (refused, see above).
- Changing or deleting a definition; only appending is possible.
- Quarterly closing of a non-calendar year (monthly only; legacy quarters stay
  for calendar years).
- Prorating depreciation for a short or long year (refused).
- An external opening-balance command: opening balances are recorded as a
  voucher, as today. For a company starting at a fiscal-year boundary the
  voucher is dated the first day of the year (the day before is outside any
  year when the first defined year starts mid-calendar-year).

## Existing homes

Migration v7 creates an empty table; an empty table is the calendar-year
calendar. A test opens a home written by the `main` binary (committed fixture)
and checks it still verifies, seals and reports.

## Simplicity

Reuses the `i32` year everywhere instead of introducing a fiscal-year id type
through storage and wire. One calendar type in `model` is the only place that
maps dates to years; the three ledger copies of "posting year" and the two
domain filters call it. No generic period abstraction, no recurrence rules
beyond "12 months after a defined year".
