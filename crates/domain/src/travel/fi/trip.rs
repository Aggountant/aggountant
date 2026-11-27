//! One trip: travel days, per diems, meal and night travel allowances.
//!
//! Travel days (decision 6 § and 13 §) are 24-hour periods of elapsed time
//! from the departure; the part after the last full day earns a partial per
//! diem from 2 hours and a full one over 6 hours (12 §). A trip shorter than a
//! day earns a partial per diem over 6 hours and a full one over 10 hours.
//!
//! Abroad (13 §) the per diem follows the country where each full travel day
//! ends. After the last travel day that ended abroad, the time to the end of
//! the trip earns that day's foreign per diem when it is over 10 hours and half
//! of it when it is over 2 hours; travel days that begin after the return to
//! Finland earn domestic per diems. A foreign trip of under 10 hours earns
//! domestic per diems, and one of 10 to 24 hours the destination's per diem.

use std::collections::BTreeSet;

use jiff::civil::{Date as CivilDate, DateTime};
use jiff::tz::Offset;
use jiff::SignedDuration;
use rust_decimal::Decimal;

use aggountant_model::Instant;

use super::rates::{self, CountryProblem, Edition};
use super::{
    mileage, money, refusal, round, CategoryAmounts, Context, DayAnswers, DayComputation, Lodging,
    MealAllowanceComputation, Situation, TripComputation, TripInput, TripKind, Workplace,
};
use crate::bank_days::finnish_local_time;
use crate::DomainError;

const HOUR: i64 = 3600;
const DAY: i64 = 24 * HOUR;
/// A trip longer than this is a posting or an input error, not a trip.
const MAX_TRIP_DAYS: i64 = 366;

#[derive(Debug, Clone, Default)]
pub(super) struct Amounts {
    pub mileage: Decimal,
    pub daily: Decimal,
    pub meal: Decimal,
    pub night: Decimal,
}

impl Amounts {
    fn output(&self) -> CategoryAmounts {
        CategoryAmounts {
            mileage: money(self.mileage),
            daily_allowances: money(self.daily),
            meal_allowance: money(self.meal),
            night_allowances: money(self.night),
        }
    }
}

pub(super) struct Computed {
    pub output: TripComputation,
    pub maximum: Amounts,
    pub payable: Amounts,
    pub km: Decimal,
    /// Incomes Register per diem types: 1 meal, 2 partial, 3 full, 4 foreign.
    pub allowance_codes: BTreeSet<u8>,
    /// `YYYY-MM` of the trip's end in Finnish time.
    pub end_month: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Basis {
    DomesticFull,
    DomesticPartial,
    Foreign,
    ForeignReturnFull,
    ForeignReturnHalf,
}

impl Basis {
    fn name(self) -> &'static str {
        match self {
            Basis::DomesticFull => "domestic_full",
            Basis::DomesticPartial => "domestic_partial",
            Basis::Foreign => "foreign",
            Basis::ForeignReturnFull => "foreign_return_full",
            Basis::ForeignReturnHalf => "foreign_return_half",
        }
    }

    /// Free meals that halve the per diem (12 § and 13 §, guidance 8.3.3 and
    /// 8.4.3); `None` when free meals do not halve it.
    fn halving_meals(self) -> Option<u8> {
        match self {
            Basis::DomesticPartial => Some(1),
            Basis::DomesticFull | Basis::Foreign | Basis::ForeignReturnFull => Some(2),
            Basis::ForeignReturnHalf => None,
        }
    }

    fn income_register_code(self) -> u8 {
        match self {
            Basis::DomesticPartial => 2,
            Basis::DomesticFull => 3,
            Basis::Foreign | Basis::ForeignReturnFull | Basis::ForeignReturnHalf => 4,
        }
    }
}

#[derive(Debug, Clone)]
struct PlannedDay {
    span: &'static str,
    from: Instant,
    until: Instant,
    basis: Basis,
    /// Country code and region of a foreign basis.
    country: Option<(String, Option<String>)>,
}

/// Compute one trip. `workplace` is the register's classification, or `None`
/// when the register refused the trip: the refusal is already recorded, and
/// the trip computes no allowance and no travel cost.
pub(super) fn compute(
    n: usize,
    trip: &TripInput,
    workplace: Option<Workplace>,
    ctx: &mut Context,
) -> Result<Computed, DomainError> {
    check(n, trip)?;
    let duration = secs(trip.end.duration_since(trip.start));
    let (timeline_start, counted) = match &trip.leisure {
        None => (trip.start, duration),
        Some(l) => (
            add(l.counted_from, -i64::from(l.outbound_travel_minutes) * 60),
            secs(l.counted_until.duration_since(l.counted_from))
                + (i64::from(l.outbound_travel_minutes) + i64::from(l.return_travel_minutes)) * 60,
        ),
    };
    let last_date = finnish_local_time(trip.end).date();

    for situation in &trip.situations {
        let (code, text) = match situation {
            Situation::StartsFromHolidayHome => (
                refusal::TRAVEL_FROM_HOLIDAY_HOME,
                "the trip starts from a holiday home (vapaa-ajan asunto); the decision and the guidance do not settle where such a trip is measured from",
            ),
            Situation::WorkplaceChanges => (
                refusal::TRAVEL_WORKPLACE_CHANGE,
                "the workplace classification changes during the trip; split it into one trip per classification",
            ),
            Situation::BorderCrossingAllowance => (
                refusal::TRAVEL_BORDER_CROSSING_ALLOWANCE,
                "aggountant does not compute the border-crossing allowance (rajanylitysraha, decision 13 §)",
            ),
            Situation::Posting => (
                refusal::TRAVEL_POSTING,
                "aggountant does not compute allowances for a posting (komennus)",
            ),
            Situation::WorkAbroad => (
                refusal::TRAVEL_WORK_ABROAD,
                "aggountant does not compute allowances for a claimant who works abroad",
            ),
            Situation::SpecialIndustryDailyJourney => (
                refusal::TRAVEL_SPECIAL_INDUSTRY,
                "a daily journey to a changing place of work on a special industry (erityisala) earns travel costs and a meal allowance only (decision 3 §), which aggountant does not compute yet",
            ),
        };
        ctx.refuse(code, format!("Trip {n}: {text}."), n, None);
    }

    let mut maximum = Amounts::default();
    let mut daily_codes = BTreeSet::new();
    let mut days_out = Vec::new();
    let mut meal_out = None;
    let mut drives_out = Vec::new();
    let mut km = Decimal::ZERO;

    let distance_test_passed = distance_test(trip);
    if workplace == Some(Workplace::RegularWorkplace) {
        ctx.refuse(
            refusal::TRAVEL_TO_REGULAR_WORKPLACE,
            format!("Trip {n} goes to the claimant's regular workplace (varsinainen työpaikka): a trip between home and the regular workplace is not a work trip, so no tax-free travel allowance is due."),
            n,
            None,
        );
    } else if let Some(workplace) = workplace {
        let m = mileage::compute(n, &trip.drives, ctx);
        maximum.mileage = m.amount;
        km = m.km;
        drives_out = m.drives;

        if workplace == Workplace::SecondaryWorkplace {
            ctx.note(
                "secondary_workplace",
                format!("Trip {n} goes to a secondary workplace (toissijainen työpaikka): only travel and lodging costs are tax-free (decision 4 §), so no per diem, meal or night travel allowance is computed, and the journeys between the lodging and the secondary workplace are not tax-free travel (guidance 4.3): leave those drives out. If the trip also went to a place the register does not name, split it into one trip per workplace."),
                Some(n),
            );
        } else if !distance_test_passed {
            ctx.note(
                "distance_test_failed",
                format!("Trip {n} does not meet the distance test of decision 11 § (over 15 km from where the trip started and over 5 km from both home and the regular workplace), so no per diem is due."),
                Some(n),
            );
        }
        let plan = if workplace == Workplace::SpecialWorkPlace && distance_test_passed {
            plan(n, trip, timeline_start, counted, ctx)
        } else {
            Some(Vec::new())
        };
        let plan = plan.unwrap_or_default();
        if !plan.is_empty() {
            if trip.leisure.is_some() {
                ctx.note(
                    "leisure_counted_time",
                    format!("Trip {n} is combined with leisure: its per diems come from the stated work and travel time (guidance 8.3.2), and its days are counted from that time, not by the clock."),
                    Some(n),
                );
            }
            if offset(trip.start) != offset(trip.end) {
                ctx.note(
                    "daylight_saving_elapsed",
                    format!("Trip {n} crosses a daylight-saving change; travel days are 24 hours of elapsed time, so the change counts as it happened."),
                    Some(n),
                );
            }
        }
        for (i, day) in plan.iter().enumerate() {
            let out = per_diem_day(n, i + 1, trip, day, ctx);
            if let Some(amount) = &out.0 {
                maximum.daily += *amount;
                if *amount > Decimal::ZERO {
                    daily_codes.insert(day.basis.income_register_code());
                }
            }
            days_out.push(out.1);
        }
        if trip.days.len() > plan.len() && !plan.is_empty() {
            ctx.note(
                "unused_day_answers",
                format!(
                    "Trip {n} has {} per diem days; the answers given for days {} and later were not used.",
                    plan.len(),
                    plan.len() + 1
                ),
                Some(n),
            );
        }
        if trip.night_allowance {
            maximum.night = night_allowances(n, trip, &plan, &mut days_out, ctx);
        }
        if let Some(meal) = &trip.meal_allowance {
            let (amount, out) = meal_allowance(n, trip, workplace, meal, !plan.is_empty(), ctx);
            maximum.meal = amount;
            meal_out = Some(out);
        }
    }

    // A trip the register refused was never classified: nothing is computed,
    // so nothing is payable either, whatever `paid` states.
    let payable = match workplace {
        None => Amounts::default(),
        Some(_) => payable(n, trip, &maximum, ctx),
    };
    // The Incomes Register gets the kilometres and per diem types of what is
    // paid, not of an entitlement the company leaves unpaid.
    let mut codes = BTreeSet::new();
    if payable.daily > Decimal::ZERO {
        codes.extend(daily_codes);
    }
    if payable.meal > Decimal::ZERO {
        codes.insert(1);
    }
    if payable.mileage == Decimal::ZERO {
        km = Decimal::ZERO;
    }
    Ok(Computed {
        output: TripComputation {
            trip: n,
            id: trip.id.clone(),
            kind: trip.kind,
            workplace,
            start: trip.start.to_string(),
            end: trip.end.to_string(),
            start_finnish_time: finnish_local_time(trip.start).to_string(),
            end_finnish_time: finnish_local_time(trip.end).to_string(),
            duration_minutes: duration / 60,
            counted_minutes: counted / 60,
            distance_test_passed,
            full_day_ends_finnish_time: (1..=counted / DAY)
                .map(|i| finnish_local_time(add(timeline_start, i * DAY)).to_string())
                .collect(),
            days: days_out,
            meal_allowance: meal_out,
            drives: drives_out,
            maximum_exact: maximum.output(),
            payable_exact: payable.output(),
        },
        maximum,
        payable,
        km,
        allowance_codes: codes,
        end_month: format!("{:04}-{:02}", last_date.year(), last_date.month()),
    })
}

/// Check trip `n`'s input on its own, before and independently of the
/// register's classification: a malformed drive is an input error even on a
/// trip the register refuses.
pub(super) fn check(n: usize, trip: &TripInput) -> Result<(), DomainError> {
    validate(n, trip, secs(trip.end.duration_since(trip.start)))?;
    let first_date = finnish_local_time(trip.start).date();
    let last_date = finnish_local_time(trip.end).date();
    for (i, drive) in trip.drives.iter().enumerate() {
        mileage::validate(n, i + 1, drive, first_date, last_date)?;
    }
    Ok(())
}

fn validate(n: usize, trip: &TripInput, duration: i64) -> Result<(), DomainError> {
    let bad = |what: &str| Err(DomainError::TravelInput(format!("trip {n}: {what}")));
    if duration <= 0 {
        return bad("end must be after start");
    }
    if duration > MAX_TRIP_DAYS * DAY {
        return bad("a trip longer than a year is not a trip");
    }
    if let Some(id) = &trip.id {
        if id.is_empty() || id.len() > 200 || id.trim() != id || id.chars().any(char::is_control) {
            return bad(
                "id must be 1..200 bytes without surrounding whitespace or control characters",
            );
        }
    }
    match trip.kind {
        TripKind::Domestic => {
            if trip.destination_country.is_some() || trip.destination_region.is_some() {
                return bad("a domestic trip has no destination_country or destination_region");
            }
            if trip
                .days
                .iter()
                .any(|d| d.end_country.is_some() || d.end_region.is_some())
            {
                return bad("the days of a domestic trip have no end_country or end_region");
            }
        }
        TripKind::Foreign => {
            if trip.destination_country.is_none() {
                return bad("a foreign trip needs destination_country (an ISO 3166 code)");
            }
        }
    }
    if trip
        .days
        .iter()
        .any(|d| d.end_region.is_some() && d.end_country.is_none())
    {
        return bad("end_region needs end_country");
    }
    if trip.days.iter().any(|d| {
        d.company_paid_meals
            .iter()
            .any(|cost| *cost <= Decimal::ZERO)
    }) {
        return bad("company_paid_meals are positive amounts");
    }
    if let Some(l) = &trip.leisure {
        let outbound_start = add(l.counted_from, -i64::from(l.outbound_travel_minutes) * 60);
        let return_end = add(l.counted_until, i64::from(l.return_travel_minutes) * 60);
        if l.counted_until <= l.counted_from {
            return bad("leisure.counted_until must be after counted_from");
        }
        if outbound_start < trip.start || return_end > trip.end {
            return bad("the counted time and its travel time must lie within the trip");
        }
    }
    if let Some(m) = &trip.meal_allowance {
        if !(1..=2).contains(&m.meals) {
            return bad("meal_allowance.meals is 1 or 2");
        }
        if m.meals == 2 && duration <= 11 * HOUR {
            return bad("meal_allowance.meals is 2 only when the working day with breaks is over 11 hours, and this trip is not that long");
        }
    }
    Ok(())
}

/// Decision 11 §: over 15 km from where the trip started, and over 5 km from
/// both home and the regular workplace.
fn distance_test(trip: &TripInput) -> bool {
    let d = &trip.distances_km;
    d.from_departure > Decimal::from(15)
        && d.from_home > Decimal::from(5)
        && d.from_regular_workplace
            .is_none_or(|km| km > Decimal::from(5))
}

/// The per diem days of the trip, or `None` when a refusal stops the plan.
fn plan(
    n: usize,
    trip: &TripInput,
    start: Instant,
    counted: i64,
    ctx: &mut Context,
) -> Option<Vec<PlannedDay>> {
    match trip.kind {
        TripKind::Domestic => Some(domestic_plan(start, counted, 0)),
        TripKind::Foreign if counted < 10 * HOUR => {
            ctx.note(
                "foreign_under_10_hours",
                format!("Trip {n} lasted under 10 hours, so it earns domestic per diems, not the destination's (decision 13 §)."),
                Some(n),
            );
            Some(domestic_plan(start, counted, 0))
        }
        TripKind::Foreign if counted < DAY => {
            let dest = trip.destination_country.as_deref().expect("validated");
            let region = trip.destination_region.as_deref();
            check_country(n, None, dest, region, ctx)?;
            Some(vec![PlannedDay {
                span: "single",
                from: start,
                until: add(start, counted),
                basis: Basis::Foreign,
                country: Some((dest.to_string(), region.map(str::to_string))),
            }])
        }
        TripKind::Foreign => {
            let dest = trip.destination_country.as_deref().expect("validated");
            check_country(n, None, dest, trip.destination_region.as_deref(), ctx)?;
            foreign_plan(n, trip, start, counted, ctx)
        }
    }
}

/// Domestic per diem days from `start`, numbering full days from `skip_days`
/// (used after the foreign part of a trip).
fn domestic_plan(start: Instant, duration: i64, skip_days: i64) -> Vec<PlannedDay> {
    let full_days = duration / DAY;
    let rest = duration % DAY;
    if full_days == 0 {
        let basis = if duration > 10 * HOUR {
            Basis::DomesticFull
        } else if duration > 6 * HOUR {
            Basis::DomesticPartial
        } else {
            return Vec::new();
        };
        return vec![PlannedDay {
            span: "single",
            from: start,
            until: add(start, duration),
            basis,
            country: None,
        }];
    }
    let mut days: Vec<PlannedDay> = (skip_days..full_days)
        .map(|i| PlannedDay {
            span: "full",
            from: add(start, i * DAY),
            until: add(start, (i + 1) * DAY),
            basis: Basis::DomesticFull,
            country: None,
        })
        .collect();
    let basis = if rest > 6 * HOUR {
        Some(Basis::DomesticFull)
    } else if rest >= 2 * HOUR {
        Some(Basis::DomesticPartial)
    } else {
        None
    };
    if let Some(basis) = basis {
        days.push(PlannedDay {
            span: "part",
            from: add(start, full_days * DAY),
            until: add(start, duration),
            basis,
            country: None,
        });
    }
    days
}

fn foreign_plan(
    n: usize,
    trip: &TripInput,
    start: Instant,
    duration: i64,
    ctx: &mut Context,
) -> Option<Vec<PlannedDay>> {
    let full_days = (duration / DAY) as usize;
    let rest = duration % DAY;
    // Where each full travel day ended: `None` for Finland.
    let mut ends: Vec<Option<(String, Option<String>)>> = Vec::with_capacity(full_days);
    let mut complete = true;
    for i in 0..full_days {
        let answer = trip.days.get(i).cloned().unwrap_or_default();
        match answer.end_country.as_deref() {
            None => {
                ctx.refuse(
                    refusal::TRAVEL_COUNTRY_UNANSWERED,
                    format!("Trip {n}, day {}: where the travel day ended (end_country) is not answered; a foreign per diem follows the country where each full travel day ends.", i + 1),
                    n,
                    Some(i + 1),
                );
                complete = false;
            }
            Some(code) if rates::is_domestic(code) => {
                if answer.end_region.is_some() {
                    complete = false;
                    ctx.refuse(
                        refusal::TRAVEL_COUNTRY_INVALID,
                        format!("Trip {n}, day {}: a travel day that ends in Finland has no end_region.", i + 1),
                        n,
                        Some(i + 1),
                    );
                }
                ends.push(None);
            }
            Some(code) => {
                let region = answer.end_region.as_deref();
                if check_country(n, Some(i + 1), code, region, ctx).is_none() {
                    complete = false;
                }
                ends.push(Some((code.to_string(), region.map(str::to_string))));
            }
        }
    }
    if !complete {
        return None;
    }
    let abroad = ends.iter().take_while(|e| e.is_some()).count();
    if abroad == 0 || ends[abroad..].iter().any(Option::is_some) {
        ctx.refuse(
            refusal::TRAVEL_FOREIGN_DAYS_IN_FINLAND,
            format!("Trip {n}: a travel day ends in Finland before a later one ends abroad, or no full travel day ends abroad. The decision does not settle the per diems of a foreign trip that passes through Finland; split the trip or ask your accountant."),
            n,
            None,
        );
        return None;
    }
    let mut days: Vec<PlannedDay> = (0..abroad)
        .map(|i| PlannedDay {
            span: "full",
            from: add(start, i as i64 * DAY),
            until: add(start, (i as i64 + 1) * DAY),
            basis: Basis::Foreign,
            country: ends[i].clone(),
        })
        .collect();
    let last_abroad = ends[abroad - 1].clone();
    if abroad < full_days {
        days.push(PlannedDay {
            span: "full",
            from: add(start, abroad as i64 * DAY),
            until: add(start, (abroad as i64 + 1) * DAY),
            basis: Basis::ForeignReturnFull,
            country: last_abroad,
        });
        days.extend(domestic_plan(start, duration, abroad as i64 + 1));
    } else if rest > 10 * HOUR || rest > 2 * HOUR {
        days.push(PlannedDay {
            span: "part",
            from: add(start, full_days as i64 * DAY),
            until: add(start, duration),
            basis: if rest > 10 * HOUR {
                Basis::ForeignReturnFull
            } else {
                Basis::ForeignReturnHalf
            },
            country: last_abroad,
        });
    }
    Some(days)
}

/// Check that a country answer resolves to a row of the foreign table.
fn check_country(
    n: usize,
    day: Option<usize>,
    code: &str,
    region: Option<&str>,
    ctx: &mut Context,
) -> Option<()> {
    let edition = &rates::table().editions[0];
    let at = match day {
        Some(d) => format!("Trip {n}, day {d}"),
        None => format!("Trip {n}"),
    };
    match rates::foreign_row(edition, code, region) {
        Ok(_) => Some(()),
        Err(problem) => {
            let (code_out, text) = match problem {
                CountryProblem::Invalid => (
                    refusal::TRAVEL_COUNTRY_INVALID,
                    format!("{at}: {code:?} is not an ISO 3166 country code. An unknown code is never treated as a country the decision does not list."),
                ),
                CountryProblem::InvalidRegion => (
                    refusal::TRAVEL_COUNTRY_INVALID,
                    format!("{at}: the decision lists no city or region {:?} under {code}.", region.unwrap_or_default()),
                ),
                CountryProblem::Domestic => (
                    refusal::TRAVEL_COUNTRY_INVALID,
                    format!("{at}: {code} is Finland, not a foreign destination."),
                ),
                CountryProblem::Unsettled(reason) => (
                    refusal::TRAVEL_TERRITORY_UNSETTLED,
                    format!("{at}: {code}: {reason}, so its per diem is not settled."),
                ),
            };
            ctx.refuse(code_out, text, n, day);
            None
        }
    }
}

/// The scheduled amount of a basis in one edition.
fn scheduled_in(edition: &Edition, day: &PlannedDay) -> Option<Decimal> {
    match day.basis {
        Basis::DomesticFull => Some(edition.domestic.full_day),
        Basis::DomesticPartial => Some(edition.domestic.partial_day),
        Basis::Foreign | Basis::ForeignReturnFull | Basis::ForeignReturnHalf => {
            let (code, region) = day.country.as_ref().expect("a foreign day has a country");
            let amount = rates::foreign_row(edition, code, region.as_deref())
                .ok()?
                .amount;
            Some(if day.basis == Basis::ForeignReturnHalf {
                amount / Decimal::TWO
            } else {
                amount
            })
        }
    }
}

/// The lowest value of `f` over the editions of the years the window touches
/// (a travel day across the year boundary gets the lower amount); `None` when
/// an edition is missing.
fn lowest_over_years(
    n: usize,
    day: Option<usize>,
    from: Instant,
    until: Instant,
    ctx: &mut Context,
    f: impl Fn(&Edition) -> Option<Decimal>,
) -> (Option<Decimal>, Vec<i16>) {
    let years = years(from, until);
    let mut lowest: Option<Decimal> = None;
    let mut missing = false;
    for year in &years {
        match ctx.edition(*year, n, day).and_then(&f) {
            Some(amount) => lowest = Some(lowest.map_or(amount, |l| l.min(amount))),
            None => missing = true,
        }
    }
    if years.len() > 1 {
        ctx.note(
            "year_boundary_lower_amount",
            "A travel day across the year boundary gets the lower of the two years' amounts (product owner 2026-11-26).",
            Some(n),
        );
    }
    (if missing { None } else { lowest }, years)
}

fn per_diem_day(
    n: usize,
    index: usize,
    trip: &TripInput,
    day: &PlannedDay,
    ctx: &mut Context,
) -> (Option<Decimal>, DayComputation) {
    let answers = trip.days.get(index - 1).cloned().unwrap_or_default();
    let (scheduled, years) = lowest_over_years(n, Some(index), day.from, day.until, ctx, |e| {
        scheduled_in(e, day)
    });
    let foreign_row = day.country.as_ref().and_then(|(code, region)| {
        rates::foreign_row(&rates::table().editions[0], code, region.as_deref())
            .ok()
            .map(|r| r.row)
    });
    let paid: Decimal = answers.company_paid_meals.iter().sum();
    let mut halved = false;
    let amount = match (scheduled, answers.free_meals) {
        (_, None) => {
            ctx.refuse(
                refusal::TRAVEL_FREE_MEALS_UNANSWERED,
                format!("Trip {n}, day {index}: the free meals (ilmaiset ateriat) are not answered. An empty answer is not \"none\": answer 0 when there were none."),
                n,
                Some(index),
            );
            None
        }
        (None, Some(_)) => None,
        (Some(full), Some(free)) => {
            per_diem_after_meals(n, index, day.basis, full, free, paid, &mut halved, ctx)
        }
    };
    let (country, region) = match &day.country {
        Some((c, r)) => (Some(c.clone()), r.clone()),
        None => (None, None),
    };
    (
        amount,
        DayComputation {
            day: index,
            span: day.span,
            from_finnish_time: finnish_local_time(day.from).to_string(),
            until_finnish_time: finnish_local_time(day.until).to_string(),
            minutes: secs(day.until.duration_since(day.from)) / 60,
            years,
            basis: day.basis.name(),
            country,
            region,
            foreign_row,
            scheduled: scheduled.map(money),
            free_meals: answers.free_meals,
            halved_for_free_meals: halved,
            company_paid_meals: money(paid),
            amount: amount.map(money),
            night_allowance: None,
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn per_diem_after_meals(
    n: usize,
    index: usize,
    basis: Basis,
    scheduled: Decimal,
    free_meals: u8,
    company_paid: Decimal,
    halved: &mut bool,
    ctx: &mut Context,
) -> Option<Decimal> {
    if company_paid > Decimal::ZERO {
        if free_meals > 0 {
            ctx.refuse(
                refusal::TRAVEL_FREE_AND_PAID_MEAL_SAME_DAY,
                format!("Trip {n}, day {index}: the day has both a free meal and a company-paid meal; the sources do not settle how the two combine."),
                n,
                Some(index),
            );
            return None;
        }
        if basis == Basis::ForeignReturnHalf {
            ctx.refuse(
                refusal::TRAVEL_PAID_MEAL_ON_RETURN_HALF,
                format!("Trip {n}, day {index}: a company-paid meal on the half foreign per diem of the return; the sources do not settle how far it reduces a per diem that free meals do not halve."),
                n,
                Some(index),
            );
            return None;
        }
        let half = scheduled / Decimal::TWO;
        if scheduled - company_paid < half {
            ctx.note(
                "company_paid_meal_floor",
                "A company-paid meal is deducted from the per diem, but the per diem never goes below half (guidance VH/630/00.01.00/2026, 8.3.3 and example 60).",
                Some(n),
            );
            return Some(half);
        }
        return Some(scheduled - company_paid);
    }
    match basis.halving_meals() {
        Some(threshold) if free_meals >= threshold => {
            *halved = true;
            Some(scheduled / Decimal::TWO)
        }
        _ => Some(scheduled),
    }
}

fn night_allowances(
    n: usize,
    trip: &TripInput,
    plan: &[PlannedDay],
    days_out: &mut [DayComputation],
    ctx: &mut Context,
) -> Decimal {
    if trip.leisure.is_some() {
        ctx.refuse(
            refusal::TRAVEL_NIGHT_ALLOWANCE_WITH_LEISURE,
            format!("Trip {n}: night travel allowance on a trip combined with leisure. Its days are counted from the stated work time, not by the clock, so the 21–07 rule cannot be applied."),
            n,
            None,
        );
        return Decimal::ZERO;
    }
    if trip.kind == TripKind::Foreign {
        ctx.refuse(
            refusal::TRAVEL_NIGHT_ALLOWANCE_ABROAD,
            format!("Trip {n}: night travel allowance on a foreign trip. The decision does not say in which time zone the hours between 21 and 07 are counted abroad."),
            n,
            None,
        );
        return Decimal::ZERO;
    }
    if plan.is_empty() {
        ctx.note(
            "night_allowance_needs_per_diem_day",
            format!("Trip {n} has no day that earns a per diem, so no night travel allowance is due (decision 17 §)."),
            Some(n),
        );
        return Decimal::ZERO;
    }
    let mut total = Decimal::ZERO;
    for (i, day) in plan.iter().enumerate() {
        if night_seconds(day.from, day.until) < 4 * HOUR {
            continue;
        }
        let index = i + 1;
        let lodging = trip.days.get(i).and_then(|d: &DayAnswers| d.lodging);
        let amount = match lodging {
            None => {
                ctx.refuse(
                    refusal::TRAVEL_NIGHT_LODGING_UNANSWERED,
                    format!("Trip {n}, day {index}: the day has 4 hours between 21 and 07, but whether the employer provided the lodging is not answered."),
                    n,
                    Some(index),
                );
                None
            }
            Some(Lodging::Provided) => {
                ctx.refuse(
                    refusal::TRAVEL_NIGHT_ALLOWANCE_WITH_LODGING,
                    format!("Trip {n}, day {index}: night travel allowance is claimed for a night whose lodging, cabin or berth the employer provided or paid; it is due only when the employer did not (decision 17 §)."),
                    n,
                    Some(index),
                );
                None
            }
            Some(Lodging::NightTravelOrWork) => {
                ctx.note(
                    "night_travel_or_work",
                    "No night travel allowance for a night spent on a night flight or working (guidance 8.7.2).",
                    Some(n),
                );
                Some(Decimal::ZERO)
            }
            Some(Lodging::SelfArranged) => {
                lowest_over_years(n, Some(index), day.from, day.until, ctx, |e| {
                    Some(e.domestic.night_travel_allowance)
                })
                .0
            }
        };
        if let Some(a) = amount {
            total += a;
        }
        if let Some(out) = days_out.get_mut(i) {
            out.night_allowance = amount.map(money);
        }
    }
    total
}

fn meal_allowance(
    n: usize,
    trip: &TripInput,
    workplace: Workplace,
    meal: &super::MealAllowanceInput,
    per_diem_due: bool,
    ctx: &mut Context,
) -> (Decimal, MealAllowanceComputation) {
    let zero = |reason: &str| {
        (
            Decimal::ZERO,
            MealAllowanceComputation {
                meals: meal.meals,
                amount: Some(money(Decimal::ZERO)),
                reason: Some(reason.to_string()),
            },
        )
    };
    let refused = || {
        (
            Decimal::ZERO,
            MealAllowanceComputation {
                meals: meal.meals,
                amount: None,
                reason: None,
            },
        )
    };
    if per_diem_due {
        ctx.refuse(
            refusal::TRAVEL_MEAL_ALLOWANCE_WITH_PER_DIEM,
            format!("Trip {n}: a meal allowance (ateriakorvaus) is claimed on a trip that earns a per diem; it is due only when no per diem is paid (decision 14 §)."),
            n,
            None,
        );
        return refused();
    }
    if workplace == Workplace::SecondaryWorkplace {
        return zero("a trip to a secondary workplace earns travel costs only");
    }
    let (Some(usual_place), Some(lunch_benefit), Some(arranged)) = (
        meal.usual_eating_place_available,
        meal.lunch_benefit_usable,
        meal.employer_arranged_meal,
    ) else {
        ctx.refuse(
            refusal::TRAVEL_MEAL_ALLOWANCE_UNANSWERED,
            format!("Trip {n}: a meal allowance needs three answers: could the claimant eat at the usual eating place, could a lunch benefit be used on the trip, and did the employer arrange a meal (decision 14 §, guidance 8.6)."),
            n,
            None,
        );
        return refused();
    };
    if arranged {
        return zero("the employer arranged a meal on the trip (for example in a course package), so no tax-free meal allowance is computed");
    }
    if usual_place {
        return zero("the claimant could eat at the usual eating place");
    }
    if lunch_benefit {
        return zero(
            "a lunch benefit could be used on the trip, so the usual eating place was available",
        );
    }
    let (amount, _) = lowest_over_years(n, None, trip.start, trip.end, ctx, |e| {
        Some(if meal.meals == 2 {
            e.domestic.meal_allowance_two_meals
        } else {
            e.domestic.meal_allowance
        })
    });
    match amount {
        Some(a) => (
            a,
            MealAllowanceComputation {
                meals: meal.meals,
                amount: Some(money(a)),
                reason: None,
            },
        ),
        None => refused(),
    }
}

/// What the company pays, refusing any amount above the maximum.
fn payable(n: usize, trip: &TripInput, maximum: &Amounts, ctx: &mut Context) -> Amounts {
    let paid = trip.paid.clone().unwrap_or_default();
    let mut check = |name: &str, given: Option<Decimal>, max: Decimal| -> Decimal {
        match given {
            None => max,
            Some(p) if p > round(max) => {
                ctx.refuse(
                    refusal::ALLOWANCE_ABOVE_MAXIMUM,
                    format!("Trip {n}: the {name} paid, {}, is above the scheduled maximum {}. The excess is pay (palkka) that needs tax withholding, which aggountant cannot do yet; pay at most the maximum.", money(p), money(round(max))),
                    n,
                    None,
                );
                p
            }
            Some(p) => p,
        }
    };
    Amounts {
        mileage: check("mileage allowance", paid.mileage, maximum.mileage),
        daily: check("per diem", paid.daily_allowances, maximum.daily),
        meal: check("meal allowance", paid.meal_allowance, maximum.meal),
        night: check(
            "night travel allowance",
            paid.night_allowances,
            maximum.night,
        ),
    }
}

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

fn secs(d: SignedDuration) -> i64 {
    d.as_secs()
}

fn add(i: Instant, seconds: i64) -> Instant {
    i.checked_add(SignedDuration::from_secs(seconds))
        .expect("a trip instant stays representable")
}

fn offset(i: Instant) -> SignedDuration {
    finnish_local_time(i).duration_since(i.utc())
}

/// The Finnish calendar years a window `[from, until)` touches.
fn years(from: Instant, until: Instant) -> Vec<i16> {
    let first = finnish_local_time(from).year();
    let last = finnish_local_time(add(until, -1)).year();
    (first..=last).collect()
}

/// The instant of a Finnish local time that exists exactly once (21:00 and
/// 07:00 are never in a daylight-saving gap or overlap).
fn finnish_instant(local: DateTime) -> Instant {
    [3, 2]
        .into_iter()
        .filter_map(|h| {
            Offset::constant(h)
                .to_timestamp(local)
                .ok()
                .map(Instant::from)
        })
        .find(|i| finnish_local_time(*i) == local)
        .expect("21:00 and 07:00 exist in Finnish time")
}

/// Seconds of `[from, until)` between 21:00 and 07:00 Finnish time.
fn night_seconds(from: Instant, until: Instant) -> i64 {
    let first: CivilDate = finnish_local_time(from)
        .date()
        .yesterday()
        .expect("date in range");
    let last: CivilDate = finnish_local_time(until).date();
    let mut total = 0;
    let mut date = first;
    while date <= last {
        let night_start = finnish_instant(date.at(21, 0, 0, 0));
        let night_end = finnish_instant(date.tomorrow().expect("date in range").at(7, 0, 0, 0));
        let a = from.max(night_start);
        let b = until.min(night_end);
        if b > a {
            total += secs(b.duration_since(a));
        }
        date = date.tomorrow().expect("date in range");
    }
    total
}
