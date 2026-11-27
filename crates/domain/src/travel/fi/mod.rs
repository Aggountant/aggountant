//! Finnish scheduled tax-free travel allowances (TVL 71–73 §).
//!
//! [`compute_claim`] takes the trips of one claim and returns, for each trip,
//! the travel days (matkavuorokaudet), per diems (päivärahat), meal allowance
//! (ateriakorvaus), night travel allowance (yömatkaraha) and mileage
//! (kilometrikorvaus), and for the claim the Incomes Register lines and the
//! proposed postings, with the rate-table edition that was used. It writes
//! nothing.
//!
//! The amounts come from the Tax Administration's yearly decision on tax-free
//! travel allowances (bundled as `data/fi-travel-allowances.json`); the
//! worked examples of its guidance VH/630/00.01.00/2026 are the test vectors.
//! Where the decision or the guidance leaves a situation open, the trip is
//! refused with a [`Refusal`] code instead of being computed by a guess. A
//! refusal blocks approval of the claim, never the writing of a draft, so it
//! is part of the result, not an error.
//!
//! Rules the sources leave open, settled by the product owner (ADR-11
//! section 2): a travel day across the year boundary gets the lower of the two
//! years' amounts; a travel day is 24 hours of elapsed time, so a
//! daylight-saving change counts as it happened; amounts are rounded once per
//! claim, half away from zero; a free meal is one meal whatever its price; a
//! company-paid own meal is deducted from the per diem down to half of it;
//! leisure combined with a trip is computed from the work and travel time the
//! user states.
//!
//! The trip carries its own workplace classification ([`Workplace`]). The
//! regular-workplace register will produce that value from its snapshot for
//! the trip's start date and pass it here unchanged.

mod mileage;
mod rates;
mod trip;

use std::collections::BTreeSet;

use rust_decimal::{Decimal, RoundingStrategy};
use serde::{Deserialize, Deserializer, Serialize};

use aggountant_model::{Date, Instant};

use crate::DomainError;

pub use rates::{reads_on_this_thread as rate_table_reads_on_this_thread, Decision};

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

/// The trips of one claim (the stdin of `claim compute`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimInput {
    pub trips: Vec<TripInput>,
}

/// One trip. Every field the computation uses is typed; an answer the
/// computation needs and the input leaves out is a refusal, never a default.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TripInput {
    /// The caller's own label for the trip, echoed in the result.
    #[serde(default)]
    pub id: Option<String>,
    pub kind: TripKind,
    /// Departure from home or the workplace, RFC 3339 with an offset.
    #[serde(deserialize_with = "instant")]
    pub start: Instant,
    /// Return home or to the workplace, RFC 3339 with an offset.
    #[serde(deserialize_with = "instant")]
    pub end: Instant,
    /// What the place of work is for this claimant on the trip's date.
    pub workplace: Workplace,
    /// The distances of decision 11 §, along the shortest available route.
    pub distances_km: Distances,
    /// The destination country (ISO 3166 code) of a foreign trip; it sets the
    /// per diem of a foreign trip that lasts less than 24 hours.
    #[serde(default)]
    pub destination_country: Option<String>,
    /// A city or region the decision lists under the destination country.
    #[serde(default)]
    pub destination_region: Option<String>,
    /// Leisure combined with the trip: the per diems come from the stated
    /// work and travel time instead of the whole absence (guidance 8.3.2).
    #[serde(default)]
    pub leisure: Option<Leisure>,
    /// Answers for each computed per diem day, in order: day 1 first.
    #[serde(default)]
    pub days: Vec<DayAnswers>,
    /// Asked only when no per diem is due (decision 14 §).
    #[serde(default)]
    pub meal_allowance: Option<MealAllowanceInput>,
    /// Whether night travel allowances are claimed (decision 17 §).
    #[serde(default)]
    pub night_allowance: bool,
    /// Drives with the claimant's own or controlled vehicle.
    #[serde(default)]
    pub drives: Vec<Drive>,
    /// Situations the sources do not settle; each one refuses the trip.
    #[serde(default)]
    pub situations: Vec<Situation>,
    /// What the company actually pays, when less than the maximum. A missing
    /// category is paid at the computed maximum.
    #[serde(default)]
    pub paid: Option<Paid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TripKind {
    Domestic,
    Foreign,
}

/// The place of work the trip goes to, for this claimant (ADR-11 section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Workplace {
    /// Erityinen työntekemispaikka: all scheduled allowances can be due.
    SpecialWorkPlace,
    /// Toissijainen työpaikka: travel costs only (decision 4 §).
    SecondaryWorkplace,
    /// Varsinainen työpaikka: no tax-free travel allowance; refused.
    RegularWorkplace,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Distances {
    /// From the place the trip started to the farthest place of work.
    #[serde(deserialize_with = "decimal")]
    pub from_departure: Decimal,
    /// From the claimant's home to the nearest place of work on the trip.
    #[serde(deserialize_with = "decimal")]
    pub from_home: Decimal,
    /// From the regular workplace to the nearest place of work on the trip;
    /// `null` only when the claimant has no regular workplace.
    /// Required: an omitted field is an error, not "no regular workplace".
    #[serde(deserialize_with = "optional_decimal")]
    pub from_regular_workplace: Option<Decimal>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Leisure {
    /// When the time that counts for per diems starts: the start of work, or
    /// the departure when the outbound journey was made for the work.
    #[serde(deserialize_with = "instant")]
    pub counted_from: Instant,
    /// When the work ends.
    #[serde(deserialize_with = "instant")]
    pub counted_until: Instant,
    /// Outbound travel time outside the counted period, in minutes.
    pub outbound_travel_minutes: u32,
    /// Return travel time outside the counted period, in minutes.
    pub return_travel_minutes: u32,
}

/// The answers for one computed per diem day.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DayAnswers {
    /// Free meals in the legal sense (ilmainen ateria): meals the employer
    /// arranged or that were included in a ticket, a course package or, abroad,
    /// the hotel room price. A breakfast is not one. `null` is unanswered.
    #[serde(default)]
    pub free_meals: Option<u8>,
    /// The company's share of each meal the traveller bought and the company
    /// paid (for example with the company card), in euros.
    #[serde(default, deserialize_with = "decimal_list")]
    pub company_paid_meals: Vec<Decimal>,
    /// Where a full travel day of a foreign trip ended: an ISO 3166 code, `FI`
    /// for Finland. On a ship or plane, the country it last left (on the way
    /// out, the one it first arrives in).
    #[serde(default)]
    pub end_country: Option<String>,
    /// A city or region the decision lists under `end_country`.
    #[serde(default)]
    pub end_region: Option<String>,
    /// The night arrangements, asked when night travel allowances are
    /// claimed and the day has 4 hours between 21 and 07.
    #[serde(default)]
    pub lodging: Option<Lodging>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lodging {
    /// The employer arranged or paid the lodging, a cabin or a berth.
    Provided,
    /// The traveller arranged the night without cost to the employer.
    SelfArranged,
    /// The night was spent travelling (a night flight) or working.
    NightTravelOrWork,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MealAllowanceInput {
    /// Whether the claimant could eat at the usual eating place during the
    /// meal break. `null` is unanswered.
    #[serde(default)]
    pub usual_eating_place_available: Option<bool>,
    /// Whether a lunch benefit (lounasseteli or -kortti) could be used on the
    /// trip. `null` is unanswered.
    #[serde(default)]
    pub lunch_benefit_usable: Option<bool>,
    /// Whether the employer arranged a meal on the trip, for example one in a
    /// course package (guidance 8.6). `null` is unanswered.
    #[serde(default)]
    pub employer_arranged_meal: Option<bool>,
    /// Meals outside the usual eating place: 1, or 2 when the working day
    /// with breaks is over 11 hours.
    pub meals: u8,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Drive {
    pub date: Date,
    /// Kilometres driven, at most two decimals.
    #[serde(deserialize_with = "decimal")]
    pub km: Decimal,
    pub vehicle: Vehicle,
    /// Car surcharges of decision 9 §, each at most once.
    #[serde(default)]
    pub surcharges: Vec<Surcharge>,
    /// Kilometres of the drive on a forest road or a closed road site.
    #[serde(default, deserialize_with = "optional_decimal")]
    pub forest_road_km: Option<Decimal>,
    /// Other people carried whose transport is the employer's matter.
    #[serde(default)]
    pub passengers: u8,
    /// The car is a car benefit car and the claimant paid the fuel.
    #[serde(default)]
    pub car_benefit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vehicle {
    Car,
    MotorboatUpTo50hp,
    MotorboatOver50hp,
    Snowmobile,
    Atv,
    Motorcycle,
    Moped,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Surcharge {
    Trailer,
    Caravan,
    HeavyLoad,
    HeavyEquipment,
    Dog,
}

/// The situations the decision and the guidance do not settle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Situation {
    /// The trip starts from a holiday home.
    StartsFromHolidayHome,
    /// The workplace classification changes during the trip.
    WorkplaceChanges,
    /// A border-crossing allowance (rajanylitysraha) is claimed.
    BorderCrossingAllowance,
    /// The trip is a posting (komennus).
    Posting,
    /// The claimant works abroad (a regular workplace abroad).
    WorkAbroad,
    /// A daily journey between home and a changing special place of work on
    /// an industry where such places change often (erityisala), with no
    /// regular workplace (decision 3 §, last paragraph).
    SpecialIndustryDailyJourney,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Paid {
    #[serde(default, deserialize_with = "optional_decimal")]
    pub mileage: Option<Decimal>,
    #[serde(default, deserialize_with = "optional_decimal")]
    pub daily_allowances: Option<Decimal>,
    #[serde(default, deserialize_with = "optional_decimal")]
    pub meal_allowance: Option<Decimal>,
    #[serde(default, deserialize_with = "optional_decimal")]
    pub night_allowances: Option<Decimal>,
}

fn instant<'de, D: Deserializer<'de>>(d: D) -> Result<Instant, D::Error> {
    let s = String::deserialize(d)?;
    Instant::parse(&s).map_err(serde::de::Error::custom)
}

fn parse_decimal(s: &str) -> Result<Decimal, String> {
    if s.is_empty()
        || !s.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        || s.starts_with('.')
        || s.ends_with('.')
    {
        return Err(format!(
            "{s:?} must be an unsigned decimal string such as \"12.50\""
        ));
    }
    let d = Decimal::from_str_exact(s).map_err(|e| format!("{s:?}: {e}"))?;
    if d.scale() > 2 {
        return Err(format!("{s:?} has more than two decimals"));
    }
    Ok(d)
}

fn decimal<'de, D: Deserializer<'de>>(d: D) -> Result<Decimal, D::Error> {
    let s = String::deserialize(d)?;
    parse_decimal(&s).map_err(serde::de::Error::custom)
}

fn optional_decimal<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Decimal>, D::Error> {
    match Option::<String>::deserialize(d)? {
        None => Ok(None),
        Some(s) => parse_decimal(&s)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

fn decimal_list<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Decimal>, D::Error> {
    Vec::<String>::deserialize(d)?
        .iter()
        .map(|s| parse_decimal(s).map_err(serde::de::Error::custom))
        .collect()
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// The computed claim. Amounts are decimal strings in euros; an `_exact`
/// amount is unrounded, everything else is rounded half away from zero once
/// per claim.
#[derive(Debug, Clone, Serialize)]
pub struct ClaimComputation {
    /// `true` when nothing was refused; a refused claim cannot be approved.
    pub approvable: bool,
    pub rate_table: RateTableStamp,
    pub trips: Vec<TripComputation>,
    pub totals: Totals,
    /// Empty while anything is refused.
    pub incomes_register_lines: Vec<IncomesRegisterLine>,
    /// One proposed voucher per calendar month in which a trip ends. Empty
    /// while anything is refused.
    pub postings: Vec<MonthPostings>,
    pub refusals: Vec<Refusal>,
    /// The rules where the sources are silent, listed when the result
    /// depends on them.
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RateTableStamp {
    /// `fi-travel-allowances@sha256:<hex>` of the bundled table.
    pub version: String,
    /// The editions the computation used, with their decisions.
    pub editions_used: Vec<EditionStamp>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditionStamp {
    pub year: i16,
    pub decision: Decision,
}

#[derive(Debug, Clone, Serialize)]
pub struct TripComputation {
    /// 1-based position in the input.
    pub trip: usize,
    pub id: Option<String>,
    pub kind: TripKind,
    pub workplace: Workplace,
    pub start: String,
    pub end: String,
    /// Finnish local times of the start and end, for reading.
    pub start_finnish_time: String,
    pub end_finnish_time: String,
    pub duration_minutes: i64,
    /// The time that counts for per diems: the duration, or with leisure the
    /// stated work and travel time.
    pub counted_minutes: i64,
    pub distance_test_passed: bool,
    /// Finnish local times at which each full 24-hour travel day ends, on the
    /// counted timeline: where the claimant was at these moments answers
    /// `days[].end_country` of a foreign trip.
    pub full_day_ends_finnish_time: Vec<String>,
    pub days: Vec<DayComputation>,
    pub meal_allowance: Option<MealAllowanceComputation>,
    pub drives: Vec<DriveComputation>,
    pub maximum_exact: CategoryAmounts,
    /// What the company pays: `paid` where given, the maximum otherwise.
    pub payable_exact: CategoryAmounts,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CategoryAmounts {
    pub mileage: String,
    pub daily_allowances: String,
    pub meal_allowance: String,
    pub night_allowances: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DayComputation {
    /// 1-based per diem day; answers in `days[day - 1]` belong to it.
    pub day: usize,
    /// `full` (24 hours), `part` (the time after the last full day) or
    /// `single` (a trip of under 24 hours).
    pub span: &'static str,
    pub from_finnish_time: String,
    pub until_finnish_time: String,
    pub minutes: i64,
    /// The calendar years the day touches; two at the year boundary.
    pub years: Vec<i16>,
    /// `domestic_full`, `domestic_partial`, `foreign`, `foreign_return_full`
    /// or `foreign_return_half`.
    pub basis: &'static str,
    pub country: Option<String>,
    pub region: Option<String>,
    /// The decision's row the foreign amount came from.
    pub foreign_row: Option<String>,
    /// The scheduled amount before meals; `null` when it cannot be computed.
    pub scheduled: Option<String>,
    pub free_meals: Option<u8>,
    pub halved_for_free_meals: bool,
    pub company_paid_meals: String,
    /// The per diem maximum of the day; `null` when refused.
    pub amount: Option<String>,
    /// The night travel allowance of the day, when claimed.
    pub night_allowance: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MealAllowanceComputation {
    pub meals: u8,
    pub amount: Option<String>,
    /// Why the amount is zero, when it is.
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DriveComputation {
    pub date: String,
    pub km: String,
    pub vehicle: Vehicle,
    pub rate_eur_per_km: Option<String>,
    pub amount_exact: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Totals {
    pub mileage: String,
    /// Kilometres for the Incomes Register, rounded half away from zero.
    pub mileage_km: i64,
    pub daily_allowances: String,
    pub meal_allowances: String,
    pub night_allowances: String,
    pub total: String,
}

/// A line of the tax-free travel allowance report (income types 311 and 331).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "income_type")]
pub enum IncomesRegisterLine {
    #[serde(rename = "311")]
    Kilometres { amount: String, kilometers: i64 },
    /// One amount for every per diem type it holds: 1 meal allowance,
    /// 2 partial, 3 full, 4 foreign.
    #[serde(rename = "331")]
    DailyAllowance {
        amount: String,
        allowance_codes: Vec<u8>,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct MonthPostings {
    /// The month the trips end in (`YYYY-MM`), which dates the voucher.
    pub month: String,
    pub lines: Vec<PostingLine>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostingLine {
    pub account: &'static str,
    pub account_name: &'static str,
    pub side: &'static str,
    pub amount: String,
}

/// A computation the program refuses to make, with the reason. It blocks
/// approval, not the draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Refusal {
    pub code: &'static str,
    pub message: String,
    /// 1-based trip, when the refusal concerns one.
    pub trip: Option<usize>,
    /// 1-based per diem day, when the refusal concerns one.
    pub day: Option<usize>,
}

/// A rule the result depends on, named so the reader can check it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Note {
    pub rule: &'static str,
    pub text: String,
    pub trip: Option<usize>,
}

/// The refusal codes.
pub mod refusal {
    pub const ALLOWANCE_ABOVE_MAXIMUM: &str = "ALLOWANCE_ABOVE_MAXIMUM";
    pub const TRAVEL_RATES_MISSING: &str = "TRAVEL_RATES_MISSING";
    pub const TRAVEL_COUNTRY_INVALID: &str = "TRAVEL_COUNTRY_INVALID";
    pub const TRAVEL_COUNTRY_UNANSWERED: &str = "TRAVEL_COUNTRY_UNANSWERED";
    pub const TRAVEL_TERRITORY_UNSETTLED: &str = "TRAVEL_TERRITORY_UNSETTLED";
    pub const TRAVEL_FREE_MEALS_UNANSWERED: &str = "TRAVEL_FREE_MEALS_UNANSWERED";
    pub const TRAVEL_MEAL_ALLOWANCE_UNANSWERED: &str = "TRAVEL_MEAL_ALLOWANCE_UNANSWERED";
    pub const TRAVEL_MEAL_ALLOWANCE_WITH_PER_DIEM: &str = "TRAVEL_MEAL_ALLOWANCE_WITH_PER_DIEM";
    pub const TRAVEL_NIGHT_LODGING_UNANSWERED: &str = "TRAVEL_NIGHT_LODGING_UNANSWERED";
    pub const TRAVEL_NIGHT_ALLOWANCE_WITH_LODGING: &str = "TRAVEL_NIGHT_ALLOWANCE_WITH_LODGING";
    pub const TRAVEL_NIGHT_ALLOWANCE_WITH_LEISURE: &str = "TRAVEL_NIGHT_ALLOWANCE_WITH_LEISURE";
    pub const TRAVEL_NIGHT_ALLOWANCE_ABROAD: &str = "TRAVEL_NIGHT_ALLOWANCE_ABROAD";
    pub const TRAVEL_FREE_AND_PAID_MEAL_SAME_DAY: &str = "TRAVEL_FREE_AND_PAID_MEAL_SAME_DAY";
    pub const TRAVEL_PAID_MEAL_ON_RETURN_HALF: &str = "TRAVEL_PAID_MEAL_ON_RETURN_HALF";
    pub const TRAVEL_FOREIGN_DAYS_IN_FINLAND: &str = "TRAVEL_FOREIGN_DAYS_IN_FINLAND";
    pub const TRAVEL_FROM_HOLIDAY_HOME: &str = "TRAVEL_FROM_HOLIDAY_HOME";
    pub const TRAVEL_WORKPLACE_CHANGE: &str = "TRAVEL_WORKPLACE_CHANGE";
    pub const TRAVEL_BORDER_CROSSING_ALLOWANCE: &str = "TRAVEL_BORDER_CROSSING_ALLOWANCE";
    pub const TRAVEL_POSTING: &str = "TRAVEL_POSTING";
    pub const TRAVEL_WORK_ABROAD: &str = "TRAVEL_WORK_ABROAD";
    pub const TRAVEL_SPECIAL_INDUSTRY: &str = "TRAVEL_SPECIAL_INDUSTRY";
    pub const TRAVEL_TO_REGULAR_WORKPLACE: &str = "TRAVEL_TO_REGULAR_WORKPLACE";
}

// ---------------------------------------------------------------------------
// Computation
// ---------------------------------------------------------------------------

/// The accounts of the proposed postings, from the general chart
/// `fi-general-2026`. The approval slice resolves them from the home's active
/// chart when it seals; the preview only shows the shape.
const ACCOUNT_TRAVEL: (&str, &str) = ("7300", "Matkakulut");
const ACCOUNT_DAILY: (&str, &str) = ("7350", "Päivärahat");
const ACCOUNT_PAYABLE: (&str, &str) = ("2740", "Muut lyhytaikaiset velat");

const MAX_TRIPS: usize = 100;

/// Compute the scheduled tax-free travel allowances of a claim. Input that
/// is malformed or contradictory is an error (`TRAVEL_INPUT_INVALID`); a
/// situation the program does not compute is a [`Refusal`] in the result.
pub fn compute_claim(input: &ClaimInput) -> Result<ClaimComputation, DomainError> {
    if input.trips.is_empty() || input.trips.len() > MAX_TRIPS {
        return Err(DomainError::TravelInput(format!(
            "a claim holds 1..={MAX_TRIPS} trips, got {}",
            input.trips.len()
        )));
    }
    let mut ctx = Context::default();
    let mut trips = Vec::with_capacity(input.trips.len());
    let mut payable = Vec::with_capacity(input.trips.len());
    for (i, trip) in input.trips.iter().enumerate() {
        let computed = trip::compute(i + 1, trip, &mut ctx)?;
        payable.push(computed.payable.clone());
        trips.push(computed);
    }

    let sum = |f: fn(&trip::Amounts) -> Decimal| -> Decimal { payable.iter().map(f).sum() };
    let mileage = sum(|a| a.mileage);
    let daily = sum(|a| a.daily);
    let meal = sum(|a| a.meal);
    let night = sum(|a| a.night);
    let km: Decimal = trips.iter().map(|t| t.km).sum();
    // Each trip's payment was checked against its own rounded maximum; the
    // claim is also checked against its maximum rounded once.
    let max_sum =
        |f: fn(&trip::Amounts) -> Decimal| -> Decimal { trips.iter().map(|t| f(&t.maximum)).sum() };
    for (name, paid, maximum) in [
        ("mileage allowance", mileage, max_sum(|a| a.mileage)),
        ("per diem", daily, max_sum(|a| a.daily)),
        ("meal allowance", meal, max_sum(|a| a.meal)),
        ("night travel allowance", night, max_sum(|a| a.night)),
    ] {
        let already = ctx
            .refusals
            .iter()
            .any(|r| r.code == refusal::ALLOWANCE_ABOVE_MAXIMUM);
        if !already && round(paid) > round(maximum) {
            ctx.refuse_claim(
                refusal::ALLOWANCE_ABOVE_MAXIMUM,
                format!("The claim's {name}, {}, is above its scheduled maximum {} rounded once per claim. The excess is pay (palkka) that needs tax withholding, which aggountant cannot do yet.", money(round(paid)), money(round(maximum))),
            );
        }
    }
    if [mileage, daily, meal, night]
        .iter()
        .any(|a| a.normalize().scale() > 2)
    {
        ctx.note(
            "rounding_once_per_claim",
            "Amounts are rounded to cents once per claim, half away from zero (product owner 2026-11-26); per-trip amounts stay exact.",
            None,
        );
    }
    let totals = Totals {
        mileage: money(round(mileage)),
        mileage_km: round_km(km),
        daily_allowances: money(round(daily)),
        meal_allowances: money(round(meal)),
        night_allowances: money(round(night)),
        total: money(round(mileage) + round(daily) + round(meal) + round(night)),
    };

    let approvable = ctx.refusals.is_empty();
    let (incomes_register_lines, postings) = if approvable {
        let lines =
            incomes_register_lines(&trips, round(mileage), round_km(km), round(daily + meal));
        if night > Decimal::ZERO {
            ctx.note(
                "night_allowance_not_reported",
                "The night travel allowance (yömatkaraha) is not reported to the Incomes Register when it is within the scheduled maximum.",
                None,
            );
        }
        (lines, month_postings(&trips, &mut ctx))
    } else {
        (Vec::new(), Vec::new())
    };

    let table = rates::table();
    let editions_used = ctx
        .years
        .iter()
        .filter_map(|y| table.edition(*y))
        .map(|e| EditionStamp {
            year: e.year,
            decision: e.decision.clone(),
        })
        .collect();

    Ok(ClaimComputation {
        approvable,
        rate_table: RateTableStamp {
            version: rates::table_version().to_string(),
            editions_used,
        },
        trips: trips.into_iter().map(|t| t.output).collect(),
        totals,
        incomes_register_lines,
        postings,
        refusals: ctx.refusals,
        notes: ctx.notes,
    })
}

fn incomes_register_lines(
    trips: &[trip::Computed],
    mileage: Decimal,
    km: i64,
    daily_and_meal: Decimal,
) -> Vec<IncomesRegisterLine> {
    let mut lines = Vec::new();
    if mileage > Decimal::ZERO {
        lines.push(IncomesRegisterLine::Kilometres {
            amount: money(mileage),
            kilometers: km,
        });
    }
    if daily_and_meal > Decimal::ZERO {
        let codes: BTreeSet<u8> = trips
            .iter()
            .flat_map(|t| t.allowance_codes.iter().copied())
            .collect();
        lines.push(IncomesRegisterLine::DailyAllowance {
            amount: money(daily_and_meal),
            allowance_codes: codes.into_iter().collect(),
        });
    }
    lines
}

fn month_postings(trips: &[trip::Computed], ctx: &mut Context) -> Vec<MonthPostings> {
    let months: BTreeSet<&str> = trips.iter().map(|t| t.end_month.as_str()).collect();
    if months.len() > 1 {
        ctx.note(
            "postings_split_by_month",
            "The trips end in several months: each month's voucher takes the rounded running total less what earlier months booked, so the vouchers add up to the claim totals.",
            None,
        );
    }
    // Round the running total, not each month, so that rounding stays once
    // per claim and the vouchers sum to the claim.
    let (mut travel_exact, mut daily_exact) = (Decimal::ZERO, Decimal::ZERO);
    let (mut travel_booked, mut daily_booked) = (Decimal::ZERO, Decimal::ZERO);
    months
        .into_iter()
        .filter_map(|month| {
            for t in trips.iter().filter(|t| t.end_month == month) {
                travel_exact += t.payable.mileage + t.payable.night;
                daily_exact += t.payable.daily + t.payable.meal;
            }
            let travel = round(travel_exact) - travel_booked;
            let daily = round(daily_exact) - daily_booked;
            travel_booked += travel;
            daily_booked += daily;
            if travel + daily == Decimal::ZERO {
                return None;
            }
            let mut lines = Vec::new();
            for ((account, name), amount) in [(ACCOUNT_TRAVEL, travel), (ACCOUNT_DAILY, daily)] {
                if amount > Decimal::ZERO {
                    lines.push(PostingLine {
                        account,
                        account_name: name,
                        side: "debit",
                        amount: money(amount),
                    });
                }
            }
            lines.push(PostingLine {
                account: ACCOUNT_PAYABLE.0,
                account_name: ACCOUNT_PAYABLE.1,
                side: "credit",
                amount: money(travel + daily),
            });
            Some(MonthPostings {
                month: month.to_string(),
                lines,
            })
        })
        .collect()
}

/// What the computation collects across trips.
#[derive(Debug, Default)]
struct Context {
    refusals: Vec<Refusal>,
    notes: Vec<Note>,
    years: BTreeSet<i16>,
}

impl Context {
    fn refuse_claim(&mut self, code: &'static str, message: String) {
        let refusal = Refusal {
            code,
            message,
            trip: None,
            day: None,
        };
        if !self.refusals.contains(&refusal) {
            self.refusals.push(refusal);
        }
    }

    fn refuse(
        &mut self,
        code: &'static str,
        message: impl Into<String>,
        trip: usize,
        day: Option<usize>,
    ) {
        let refusal = Refusal {
            code,
            message: message.into(),
            trip: Some(trip),
            day,
        };
        if !self.refusals.contains(&refusal) {
            self.refusals.push(refusal);
        }
    }

    fn note(&mut self, rule: &'static str, text: impl Into<String>, trip: Option<usize>) {
        let note = Note {
            rule,
            text: text.into(),
            trip,
        };
        if !self.notes.contains(&note) {
            self.notes.push(note);
        }
    }

    /// The edition for `year`, or a `TRAVEL_RATES_MISSING` refusal.
    fn edition(
        &mut self,
        year: i16,
        trip: usize,
        day: Option<usize>,
    ) -> Option<&'static rates::Edition> {
        match rates::table().edition(year) {
            Some(e) => {
                self.years.insert(year);
                Some(e)
            }
            None => {
                self.refuse(
                    refusal::TRAVEL_RATES_MISSING,
                    format!(
                        "aggountant has no travel allowance rate table for {year}: the Tax Administration's decision for that year is not bundled with this version, and amounts are never carried over from another year. Update aggountant."
                    ),
                    trip,
                    day,
                );
                None
            }
        }
    }
}

/// Round to cents, half away from zero.
fn round(d: Decimal) -> Decimal {
    d.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

fn round_km(km: Decimal) -> i64 {
    use rust_decimal::prelude::ToPrimitive;
    km.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
        .to_i64()
        .expect("kilometres fit in i64")
}

/// A euro amount with at least two decimals and no trailing zeros beyond them.
fn money(d: Decimal) -> String {
    let mut n = d.normalize();
    if n.scale() < 2 {
        n.rescale(2);
    }
    n.to_string()
}
