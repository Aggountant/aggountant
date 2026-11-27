//! Golden vectors for the Finnish travel allowance computation.
//!
//! Every trip is synthetic. Amounts come from the Tax Administration's
//! decisions 970/2025 (2026) and 929/2026 (2027); the guidance examples are
//! from VH/630/00.01.00/2026.

use aggountant_domain::travel::fi::{compute_claim, ClaimComputation, ClaimInput};
use aggountant_domain::workplace::{ClaimantFact, Place};
use aggountant_domain::DomainError;
use aggountant_model::Instant;
use serde_json::{json, Value};

/// The synthetic claimant's register: an employee of Esimerkki Oy whose
/// regular workplace is the Tampere office, with a secondary workplace in Oulu
/// and a customer site in Turku, all recorded in 2020.
fn register() -> (Vec<Place>, Vec<ClaimantFact>) {
    let place = |id: &str, kind: &str, municipality: &str| {
        json!({"employee_id": "maija", "place_id": id, "kind": kind, "label": id,
               "location": format!("Esimerkkikatu 1, {municipality}"),
               "municipality": municipality, "replaces": null,
               "recorded_at": "2020-01-01T00:00:00Z", "confirmed_by": "Maija Meikäläinen"})
    };
    let places = serde_json::from_value(json!([
        place("home", "home", "Tampere"),
        place("office", "company_premises", "Tampere"),
        place("branch", "company_premises", "Oulu"),
        place("customer", "customer_site", "Turku"),
    ]))
    .unwrap();
    let fact = |id: i64, fact: Value| {
        json!({"fact_id": id, "employee_id": "maija",
               "change": {"assert": {"valid_from": "2020-01-01", "valid_to": null, "fact": fact}},
               "recorded_at": "2020-01-01T00:00:00Z", "confirmed_by": "Maija Meikäläinen",
               "evidence_hash": null, "supersedes": null, "reason": null})
    };
    let facts = serde_json::from_value(json!([
        fact(
            1,
            json!({"kind": "relationship", "relationship": "employee"})
        ),
        fact(
            2,
            json!({"kind": "workplace", "role": "regular_primary", "place_id": "office",
                       "basis": "works_there_regularly"})
        ),
        fact(
            3,
            json!({"kind": "workplace", "role": "secondary", "place_id": "branch",
                       "basis": "premises_in_another_municipality"})
        ),
    ]))
    .unwrap();
    (places, facts)
}

fn now() -> Instant {
    Instant::parse("2030-01-01T00:00:00Z").unwrap()
}

fn claim(trips: Value) -> ClaimInput {
    serde_json::from_value(json!({ "employee_id": "maija", "trips": trips })).unwrap()
}

fn run(input: &ClaimInput) -> Result<ClaimComputation, DomainError> {
    let (places, facts) = register();
    compute_claim(input, &places, &facts, now())
}

fn compute(trips: Value) -> ClaimComputation {
    run(&claim(trips)).unwrap()
}

fn as_json(c: &ClaimComputation) -> Value {
    serde_json::to_value(c).unwrap()
}

/// A domestic trip to a special place of work that passes the distance test.
fn domestic(start: &str, end: &str) -> Value {
    json!({
        "kind": "domestic",
        "start": start,
        "end": end,
        "places": {"start_place_id": "home", "destination_place_ids": ["customer"]},
        "distances_km": {"from_departure": "80", "from_home": "80", "from_regular_workplace": "78"}
    })
}

fn foreign(start: &str, end: &str, dest: &str) -> Value {
    let mut t = domestic(start, end);
    t["kind"] = json!("foreign");
    t["destination_country"] = json!(dest);
    t
}

fn with(mut trip: Value, key: &str, value: Value) -> Value {
    trip[key] = value;
    trip
}

fn no_free_meals(days: usize) -> Value {
    Value::Array((0..days).map(|_| json!({"free_meals": 0})).collect())
}

fn daily(c: &ClaimComputation) -> &str {
    &c.totals.daily_allowances
}

fn codes(c: &ClaimComputation) -> Vec<&'static str> {
    c.refusals.iter().map(|r| r.code).collect()
}

fn day_amounts(c: &ClaimComputation) -> Vec<Option<String>> {
    c.trips[0].days.iter().map(|d| d.amount.clone()).collect()
}

// --- domestic per diems ---------------------------------------------------

#[test]
fn partial_domestic_day() {
    let c = compute(json!([with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T16:00:00+02:00"),
        "days",
        no_free_meals(1)
    )]));
    assert!(c.approvable, "{:?}", c.refusals);
    assert_eq!(daily(&c), "25.00");
    assert_eq!(c.trips[0].days[0].basis, "domestic_partial");
}

#[test]
fn full_domestic_day() {
    let c = compute(json!([with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
        "days",
        no_free_meals(1)
    )]));
    assert_eq!(daily(&c), "54.00");
    assert_eq!(c.trips[0].days[0].basis, "domestic_full");
}

#[test]
fn exactly_six_hours_earns_no_per_diem() {
    let c = compute(json!([domestic(
        "2026-03-10T08:00:00+02:00",
        "2026-03-10T14:00:00+02:00"
    )]));
    assert!(c.approvable);
    assert!(c.trips[0].days.is_empty());
    assert_eq!(daily(&c), "0.00");
    // Six hours and a minute does.
    let c = compute(json!([with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T14:01:00+02:00"),
        "days",
        no_free_meals(1)
    )]));
    assert_eq!(daily(&c), "25.00");
}

#[test]
fn exactly_ten_hours_is_still_a_partial_day() {
    let c = compute(json!([with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T18:00:00+02:00"),
        "days",
        no_free_meals(1)
    )]));
    assert_eq!(daily(&c), "25.00");
}

#[test]
fn two_day_trip_with_a_tied_two_hour_last_day() {
    // 26 hours: one full travel day and exactly 2 hours over it.
    let c = compute(json!([with(
        domestic("2026-03-09T08:00:00+02:00", "2026-03-10T10:00:00+02:00"),
        "days",
        no_free_meals(2)
    )]));
    assert_eq!(
        day_amounts(&c),
        [Some("54.00".into()), Some("25.00".into())]
    );
    assert_eq!(daily(&c), "79.00");
    // One minute short of 2 hours earns nothing for the last part.
    let c = compute(json!([with(
        domestic("2026-03-09T08:00:00+02:00", "2026-03-10T09:59:00+02:00"),
        "days",
        no_free_meals(1)
    )]));
    assert_eq!(daily(&c), "54.00");
}

#[test]
fn halving_by_free_meals() {
    // A full per diem is halved by two free meals, not by one.
    let one = compute(json!([with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
        "days",
        json!([{"free_meals": 1}])
    )]));
    assert_eq!(daily(&one), "54.00");
    let two = compute(json!([with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
        "days",
        json!([{"free_meals": 2}])
    )]));
    assert_eq!(daily(&two), "27.00");
    assert!(two.trips[0].days[0].halved_for_free_meals);
    // A partial per diem is halved by one.
    let partial = compute(json!([with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T16:00:00+02:00"),
        "days",
        json!([{"free_meals": 1}])
    )]));
    assert_eq!(daily(&partial), "12.50");
}

#[test]
fn two_free_meals_of_different_prices_halve_by_count() {
    // Prices are not an input: a free meal is one meal whatever it cost, so a
    // business lunch and a cheap dinner halve the per diem like any two.
    let c = compute(json!([with(
        domestic("2026-03-10T07:00:00+02:00", "2026-03-10T22:00:00+02:00"),
        "days",
        json!([{"free_meals": 2}])
    )]));
    assert_eq!(daily(&c), "27.00");
    // Three free meals still halve once.
    let c = compute(json!([with(
        domestic("2026-03-10T07:00:00+02:00", "2026-03-10T22:00:00+02:00"),
        "days",
        json!([{"free_meals": 3}])
    )]));
    assert_eq!(daily(&c), "27.00");
}

#[test]
fn company_paid_meal_is_deducted() {
    let c = compute(json!([with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
        "days",
        json!([{"free_meals": 0, "company_paid_meals": ["10.00"]}])
    )]));
    assert!(c.approvable);
    assert_eq!(daily(&c), "44.00");
}

#[test]
fn company_paid_meal_above_half_ends_at_half_without_refusal() {
    // Guidance 8.3.3: partial per diem 25 €, the employer pays 20 € of a
    // meal: half the partial per diem, 12.50 €, not 5 €.
    let c = compute(json!([with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T16:00:00+02:00"),
        "days",
        json!([{"free_meals": 0, "company_paid_meals": ["20.00"]}])
    )]));
    assert!(c.approvable);
    assert_eq!(daily(&c), "12.50");
    assert!(c.notes.iter().any(|n| n.rule == "company_paid_meal_floor"));
}

// --- guidance examples ----------------------------------------------------

#[test]
fn guidance_example_56_three_travel_days() {
    // Monday 08:00 to Thursday 09:00, 73 hours: three per diems.
    let c = compute(json!([with(
        domestic("2026-03-09T08:00:00+02:00", "2026-03-12T09:00:00+02:00"),
        "days",
        no_free_meals(3)
    )]));
    assert_eq!(c.trips[0].duration_minutes, 73 * 60);
    assert_eq!(c.trips[0].days.len(), 3);
    assert_eq!(daily(&c), "162.00");
}

#[test]
fn guidance_example_57_fourth_full_per_diem() {
    // Monday 08:00 to Thursday 19:00, 83 hours: the last day is over 6 hours.
    let c = compute(json!([with(
        domestic("2026-03-09T08:00:00+02:00", "2026-03-12T19:00:00+02:00"),
        "days",
        no_free_meals(4)
    )]));
    assert_eq!(c.trips[0].days.len(), 4);
    assert_eq!(c.trips[0].days[3].basis, "domestic_full");
    assert_eq!(daily(&c), "216.00");
}

#[test]
fn guidance_example_58_leisure_before_the_work() {
    // Leaves Saturday 12:00 for leisure; work Monday 09:00 to Tuesday 16:00;
    // 4.5 hours each way. Per diems from 40 hours: two full per diems.
    let trip = with(
        with(
            domestic("2026-03-07T12:00:00+02:00", "2026-03-10T20:30:00+02:00"),
            "leisure",
            json!({
                "counted_from": "2026-03-09T09:00:00+02:00",
                "counted_until": "2026-03-10T16:00:00+02:00",
                "outbound_travel_minutes": 270,
                "return_travel_minutes": 270
            }),
        ),
        "days",
        no_free_meals(2),
    );
    let c = compute(json!([trip]));
    assert!(c.approvable, "{:?}", c.refusals);
    assert_eq!(c.trips[0].counted_minutes, 40 * 60);
    assert_eq!(daily(&c), "108.00");
}

#[test]
fn guidance_example_59_leisure_after_the_work() {
    // Leaves Wednesday 19:00, negotiations end Thursday 20:00, stays for the
    // weekend, returns Sunday evening in 4 hours: 29 hours, one full and one
    // partial per diem.
    let trip = with(
        with(
            domestic("2026-03-11T19:00:00+02:00", "2026-03-15T23:00:00+02:00"),
            "leisure",
            json!({
                "counted_from": "2026-03-11T19:00:00+02:00",
                "counted_until": "2026-03-12T20:00:00+02:00",
                "outbound_travel_minutes": 0,
                "return_travel_minutes": 240
            }),
        ),
        "days",
        no_free_meals(2),
    );
    let c = compute(json!([trip]));
    assert_eq!(c.trips[0].counted_minutes, 29 * 60);
    assert_eq!(
        day_amounts(&c),
        [Some("54.00".into()), Some("25.00".into())]
    );
    assert!(c.notes.iter().any(|n| n.rule == "leisure_counted_time"));
}

#[test]
fn guidance_example_61_foreign_trip_with_return_day_rule() {
    // Germany, Sunday 17:00 to Wednesday 20:00 (75 hours). Days 1 and 2 end
    // in Germany, day 3 in Finland. Three German per diems (the time after
    // Tuesday 17:00 is over 10 hours) and one domestic partial per diem.
    let trip = with(
        foreign(
            "2026-03-08T17:00:00+02:00",
            "2026-03-11T20:00:00+02:00",
            "DE",
        ),
        "days",
        json!([
            {"free_meals": 0, "end_country": "DE"},
            {"free_meals": 0, "end_country": "DE"},
            {"free_meals": 0, "end_country": "FI"},
            {"free_meals": 0}
        ]),
    );
    let c = compute(json!([trip]));
    assert!(c.approvable, "{:?}", c.refusals);
    let bases: Vec<_> = c.trips[0].days.iter().map(|d| d.basis).collect();
    assert_eq!(
        bases,
        [
            "foreign",
            "foreign",
            "foreign_return_full",
            "domestic_partial"
        ]
    );
    assert_eq!(daily(&c), "259.00"); // 3 × 78 + 25
}

#[test]
fn foreign_return_half_when_the_last_day_ended_abroad() {
    // Two full days end in Sweden, 3 hours after: half the Swedish per diem,
    // not halved by a free meal.
    let trip = with(
        foreign(
            "2026-03-08T08:00:00+02:00",
            "2026-03-10T11:00:00+02:00",
            "SE",
        ),
        "days",
        json!([
            {"free_meals": 0, "end_country": "SE"},
            {"free_meals": 0, "end_country": "SE"},
            {"free_meals": 2}
        ]),
    );
    let c = compute(json!([trip]));
    assert_eq!(c.trips[0].days[2].basis, "foreign_return_half");
    assert_eq!(day_amounts(&c)[2].as_deref(), Some("35.00"));
    assert_eq!(daily(&c), "175.00"); // 70 + 70 + 35
}

#[test]
fn foreign_full_day_halved_by_two_free_meals() {
    let trip = with(
        foreign(
            "2026-03-08T08:00:00+02:00",
            "2026-03-09T08:00:00+02:00",
            "EE",
        ),
        "days",
        json!([{"free_meals": 2, "end_country": "EE"}]),
    );
    assert_eq!(daily(&compute(json!([trip]))), "41.00");
}

#[test]
fn guidance_example_63_under_ten_hours_abroad_is_domestic() {
    let c = compute(json!([with(
        foreign(
            "2026-03-10T07:00:00+02:00",
            "2026-03-10T16:00:00+02:00",
            "SE"
        ),
        "days",
        no_free_meals(1)
    )]));
    assert_eq!(c.trips[0].days[0].basis, "domestic_partial");
    assert_eq!(daily(&c), "25.00");
}

#[test]
fn foreign_day_trip_of_ten_hours_earns_the_destination_rate() {
    let c = compute(json!([with(
        foreign(
            "2026-03-10T07:00:00+02:00",
            "2026-03-10T17:00:00+02:00",
            "SE"
        ),
        "days",
        no_free_meals(1)
    )]));
    assert_eq!(c.trips[0].days[0].basis, "foreign");
    assert_eq!(daily(&c), "70.00");
}

#[test]
fn listed_city_and_region_rates() {
    let london = with(
        with(
            foreign(
                "2026-03-10T06:00:00+02:00",
                "2026-03-10T22:00:00+02:00",
                "GB",
            ),
            "destination_region",
            json!("london"),
        ),
        "days",
        no_free_meals(1),
    );
    assert_eq!(daily(&compute(json!([london]))), "89.00");
    let jersey = with(
        foreign(
            "2026-03-10T06:00:00+02:00",
            "2026-03-10T22:00:00+02:00",
            "JE",
        ),
        "days",
        no_free_meals(1),
    );
    assert_eq!(daily(&compute(json!([jersey]))), "84.00");
    let hong_kong = with(
        foreign(
            "2026-03-10T06:00:00+02:00",
            "2026-03-10T22:00:00+02:00",
            "HK",
        ),
        "days",
        no_free_meals(1),
    );
    assert_eq!(daily(&compute(json!([hong_kong]))), "79.00");
    let gibraltar = with(
        foreign(
            "2026-03-10T06:00:00+02:00",
            "2026-03-10T22:00:00+02:00",
            "GI",
        ),
        "days",
        no_free_meals(1),
    );
    let c = compute(json!([gibraltar]));
    assert_eq!(daily(&c), "52.00");
    assert_eq!(
        c.trips[0].days[0].foreign_row.as_deref(),
        Some("Maa, jota ei ole erikseen mainittu")
    );
}

// --- year boundary, rounding, stamps -------------------------------------

#[test]
fn year_boundary_day_gets_the_lower_amount() {
    // Day 1 spans 2026 and 2027: min(54, 55) = 54. The part in 2027: 25.
    let c = compute(json!([with(
        domestic("2026-12-31T12:00:00+02:00", "2027-01-01T14:00:00+02:00"),
        "days",
        no_free_meals(2)
    )]));
    assert!(c.approvable);
    assert_eq!(c.trips[0].days[0].years, [2026, 2027]);
    assert_eq!(
        day_amounts(&c),
        [Some("54.00".into()), Some("25.00".into())]
    );
    assert!(c
        .notes
        .iter()
        .any(|n| n.rule == "year_boundary_lower_amount"));
    let years: Vec<i16> = c.rate_table.editions_used.iter().map(|e| e.year).collect();
    assert_eq!(years, [2026, 2027]);
    // A full day wholly in 2027 earns the 2027 amount.
    let c = compute(json!([with(
        domestic("2027-01-05T08:00:00+02:00", "2027-01-05T19:00:00+02:00"),
        "days",
        no_free_meals(1)
    )]));
    assert_eq!(daily(&c), "55.00");
}

#[test]
fn year_boundary_foreign_and_night_allowance_take_the_lower_amount() {
    // Germany 78 (2026) vs 80 (2027).
    let trip = with(
        foreign(
            "2026-12-31T08:00:00+02:00",
            "2027-01-01T08:00:00+02:00",
            "DE",
        ),
        "days",
        json!([{"free_meals": 0, "end_country": "DE"}]),
    );
    let c = compute(json!([trip]));
    assert!(c.approvable, "{:?}", c.refusals);
    assert_eq!(daily(&c), "78.00");
    // Night travel allowance 16 (2026) vs 15 (2027).
    let trip = with(
        with(
            domestic("2026-12-31T08:00:00+02:00", "2027-01-01T08:00:00+02:00"),
            "days",
            json!([{"free_meals": 0, "lodging": "self_arranged"}]),
        ),
        "night_allowance",
        json!(true),
    );
    let c = compute(json!([trip]));
    assert!(c.approvable, "{:?}", c.refusals);
    assert_eq!(c.totals.night_allowances, "15.00");
}

#[test]
fn refusal_night_allowance_abroad() {
    let trip = with(
        with(
            foreign(
                "2026-03-08T08:00:00+02:00",
                "2026-03-09T08:00:00+02:00",
                "EE",
            ),
            "days",
            json!([{"free_meals": 0, "end_country": "EE", "lodging": "self_arranged"}]),
        ),
        "night_allowance",
        json!(true),
    );
    assert_eq!(
        codes(&compute(json!([trip]))),
        ["TRAVEL_NIGHT_ALLOWANCE_ABROAD"]
    );
}

#[test]
fn full_day_ends_are_shown_before_the_countries_are_answered() {
    let c = compute(json!([foreign(
        "2026-03-08T17:00:00+02:00",
        "2026-03-11T20:00:00+02:00",
        "DE"
    )]));
    assert_eq!(
        c.trips[0].full_day_ends_finnish_time,
        [
            "2026-03-09T17:00:00",
            "2026-03-10T17:00:00",
            "2026-03-11T17:00:00"
        ]
    );
}

#[test]
fn a_day_ending_at_midnight_new_year_belongs_to_the_old_year() {
    let c = compute(json!([with(
        domestic("2026-12-31T08:00:00+02:00", "2027-01-01T00:00:00+02:00"),
        "days",
        no_free_meals(1)
    )]));
    assert_eq!(c.trips[0].days[0].years, [2026]);
}

#[test]
fn daylight_saving_change_counts_as_elapsed_time() {
    // Summer time starts 2026-03-29 at 03:00 local. From Saturday 08:00 to
    // Sunday 09:00 on the clock is 24 hours of elapsed time: exactly one day.
    let c = compute(json!([with(
        domestic("2026-03-28T08:00:00+02:00", "2026-03-29T09:00:00+03:00"),
        "days",
        no_free_meals(1)
    )]));
    assert_eq!(c.trips[0].duration_minutes, 24 * 60);
    assert_eq!(c.trips[0].days.len(), 1);
    assert!(c.notes.iter().any(|n| n.rule == "daylight_saving_elapsed"));
}

#[test]
fn mileage_rounding_reports_whole_kilometres() {
    let trip = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T13:00:00+02:00"),
        "drives",
        json!([{"date": "2026-03-10", "km": "125.6", "vehicle": "car"}]),
    );
    let c = compute(json!([trip]));
    assert_eq!(c.totals.mileage, "69.08");
    assert_eq!(c.totals.mileage_km, 126);
    let v = as_json(&c);
    assert_eq!(
        v["incomes_register_lines"],
        json!([{"income_type": "311", "amount": "69.08", "kilometers": 126}])
    );
}

#[test]
fn mileage_is_rounded_once_per_claim() {
    // 10.05 km × 0.55 = 5.5275 twice: 11.055 → 11.06 once, not 5.53 + 5.53.
    let trip = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T13:00:00+02:00"),
        "drives",
        json!([
            {"date": "2026-03-10", "km": "10.05", "vehicle": "car"},
            {"date": "2026-03-10", "km": "10.05", "vehicle": "car"}
        ]),
    );
    let c = compute(json!([trip]));
    assert_eq!(c.trips[0].maximum_exact.mileage, "11.055");
    assert_eq!(c.totals.mileage, "11.06");
}

#[test]
fn mileage_surcharges_passengers_and_car_benefit() {
    // 100 km with a trailer, 20 km of it on a forest road, two passengers:
    // 100 × (0.55 + 0.09 + 2 × 0.04) + 20 × 0.12 = 72 + 2.40 = 74.40.
    let trip = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T13:00:00+02:00"),
        "drives",
        json!([{"date": "2026-03-10", "km": "100", "vehicle": "car",
                 "surcharges": ["trailer"], "forest_road_km": "20", "passengers": 2}]),
    );
    assert_eq!(compute(json!([trip])).totals.mileage, "74.40");
    let trip = with(
        domestic("2027-03-10T08:00:00+02:00", "2027-03-10T13:00:00+02:00"),
        "drives",
        json!([{"date": "2027-03-10", "km": "100", "vehicle": "car", "car_benefit": true}]),
    );
    assert_eq!(compute(json!([trip])).totals.mileage, "12.00");
    let trip = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T13:00:00+02:00"),
        "drives",
        json!([{"date": "2026-03-10", "km": "10", "vehicle": "snowmobile"}]),
    );
    assert_eq!(compute(json!([trip])).totals.mileage, "13.40");
}

#[test]
fn output_stamps_the_table_hash_and_decisions() {
    let c = compute(json!([with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
        "days",
        no_free_meals(1)
    )]));
    let hash = c
        .rate_table
        .version
        .strip_prefix("fi-travel-allowances@sha256:")
        .unwrap();
    assert_eq!(hash.len(), 64);
    let d = &c.rate_table.editions_used[0].decision;
    assert_eq!(d.statute, "970/2025");
    assert_eq!(d.diary_number, "VH/6575/00.01.00/2025");
    let v = as_json(&c);
    assert_eq!(
        v["rate_table"]["editions_used"][0]["decision"]["statute"],
        "970/2025"
    );
}

#[test]
fn incomes_register_lines_and_postings() {
    let trip = with(
        with(
            domestic("2026-03-10T07:00:00+02:00", "2026-03-11T10:00:00+02:00"),
            "days",
            no_free_meals(2),
        ),
        "drives",
        json!([{"date": "2026-03-10", "km": "200", "vehicle": "car"}]),
    );
    let c = compute(json!([trip]));
    let v = as_json(&c);
    assert_eq!(
        v["incomes_register_lines"],
        json!([
            {"income_type": "311", "amount": "110.00", "kilometers": 200},
            {"income_type": "331", "amount": "79.00", "allowance_codes": [2, 3]}
        ])
    );
    // file-wage reads the calculator's lines as they are.
    let lines: Vec<aggountant_domain::TravelLine> =
        serde_json::from_value(v["incomes_register_lines"].clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&lines).unwrap(),
        v["incomes_register_lines"]
    );
    assert_eq!(
        v["postings"],
        json!([{"month": "2026-03", "lines": [
            {"account": "7300", "account_name": "Matkakulut", "side": "debit", "amount": "110.00"},
            {"account": "7350", "account_name": "Päivärahat", "side": "debit", "amount": "79.00"},
            {"account": "2740", "account_name": "Muut lyhytaikaiset velat", "side": "credit", "amount": "189.00"}
        ]}])
    );
    assert_eq!(c.totals.total, "189.00");
}

#[test]
fn night_allowance_needs_four_night_hours_and_no_provided_lodging() {
    // Day 1 (Mon 08:00 to Tue 08:00) has 10 night hours; the part after it
    // (Tue 08:00–18:00) has none.
    let trip = with(
        with(
            domestic("2026-03-09T08:00:00+02:00", "2026-03-10T18:00:00+02:00"),
            "days",
            json!([{"free_meals": 0, "lodging": "self_arranged"}, {"free_meals": 0}]),
        ),
        "night_allowance",
        json!(true),
    );
    let c = compute(json!([trip]));
    assert!(c.approvable, "{:?}", c.refusals);
    assert_eq!(c.totals.night_allowances, "16.00");
    assert!(c
        .notes
        .iter()
        .any(|n| n.rule == "night_allowance_not_reported"));
}

#[test]
fn meal_allowance_when_no_per_diem_is_due() {
    let trip = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T13:00:00+02:00"),
        "meal_allowance",
        json!({"usual_eating_place_available": false, "lunch_benefit_usable": false, "employer_arranged_meal": false, "meals": 1}),
    );
    let c = compute(json!([trip]));
    assert!(c.approvable);
    assert_eq!(c.totals.meal_allowances, "13.50");
    assert_eq!(
        as_json(&c)["incomes_register_lines"],
        json!([{"income_type": "331", "amount": "13.50", "allowance_codes": [1]}])
    );
    // A usable lunch benefit means no tax-free meal allowance.
    let trip = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T13:00:00+02:00"),
        "meal_allowance",
        json!({"usual_eating_place_available": false, "lunch_benefit_usable": true, "employer_arranged_meal": false, "meals": 1}),
    );
    assert_eq!(compute(json!([trip])).totals.meal_allowances, "0.00");
}

#[test]
fn distance_test_failure_leaves_no_per_diem() {
    let mut trip = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
        "days",
        no_free_meals(1),
    );
    trip["distances_km"] =
        json!({"from_departure": "15", "from_home": "15", "from_regular_workplace": "15"});
    let c = compute(json!([trip]));
    assert!(!c.trips[0].distance_test_passed);
    assert_eq!(daily(&c), "0.00");
}

#[test]
fn secondary_workplace_earns_mileage_only() {
    let mut trip = with(
        with(
            domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
            "days",
            no_free_meals(1),
        ),
        "drives",
        json!([{"date": "2026-03-10", "km": "100", "vehicle": "car"}]),
    );
    trip["places"]["destination_place_ids"] = json!(["branch"]);
    let c = compute(json!([trip]));
    assert!(c.approvable);
    assert_eq!(daily(&c), "0.00");
    assert_eq!(c.totals.mileage, "55.00");
}

#[test]
fn a_lower_paid_amount_is_used() {
    let trip = with(
        with(
            domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
            "days",
            no_free_meals(1),
        ),
        "paid",
        json!({"daily_allowances": "40.00"}),
    );
    let c = compute(json!([trip]));
    assert!(c.approvable);
    assert_eq!(daily(&c), "40.00");
    assert_eq!(c.trips[0].maximum_exact.daily_allowances, "54.00");
}

// --- the foreign table ----------------------------------------------------

#[test]
fn foreign_table_matches_the_decisions() {
    // Row count: the decision's own rows (countries, regions, cities and the
    // "not separately listed" row), against `foreign_rows` taken from it.
    let table: Value =
        serde_json::from_str(include_str!("../data/fi-travel-allowances.json")).unwrap();
    for edition in table["editions"].as_array().unwrap() {
        let countries = edition["foreign"]["countries"].as_object().unwrap();
        let mut rows = std::collections::BTreeSet::new();
        for (code, c) in countries {
            rows.insert(format!("{code}:{}", c["row"]));
            for r in c["regions"].as_object().into_iter().flatten() {
                rows.insert(format!("{code}:{}", r.1["row"]));
            }
        }
        assert_eq!(
            rows.len() + 1,
            edition["decision"]["foreign_rows"].as_u64().unwrap() as usize
        );
        assert_eq!(rows.len() + 1, 211);
    }
    let spot = |year: usize, code: &str, region: Option<&str>| -> String {
        let c = &table["editions"][year]["foreign"]["countries"][code];
        match region {
            None => c["amount"].as_str().unwrap().to_string(),
            Some(r) => c["regions"][r]["amount"].as_str().unwrap().to_string(),
        }
    };
    // 2026, decision 970/2025 13 §.
    for (code, region, amount) in [
        ("AF", None, "58"),
        ("DE", None, "78"),
        ("SE", None, "70"),
        ("EE", None, "82"),
        ("GB", None, "84"),
        ("GB", Some("edinburgh"), "89"),
        ("ES", Some("canary-islands"), "75"),
        ("PT", Some("madeira"), "71"),
        ("RU", Some("moscow"), "108"),
        ("RU", Some("st-petersburg"), "100"),
        ("US", Some("washington"), "93"),
        ("TR", Some("istanbul"), "44"),
        ("CN", Some("hong-kong"), "79"),
        ("XK", None, "62"),
        ("ZW", None, "122"),
    ] {
        assert_eq!(spot(0, code, region), amount, "2026 {code} {region:?}");
    }
    assert_eq!(table["editions"][0]["foreign"]["not_listed"], "52");
    // 2027, decision 929/2026 13 §.
    for (code, region, amount) in [
        ("AF", None, "57"),
        ("DE", None, "80"),
        ("SE", None, "71"),
        ("US", None, "89"),
        ("US", Some("new-york"), "97"),
        ("GB", Some("london"), "92"),
        ("PT", Some("azores"), "75"),
        ("ZW", None, "125"),
    ] {
        assert_eq!(spot(1, code, region), amount, "2027 {code} {region:?}");
    }
    assert_eq!(table["editions"][1]["foreign"]["not_listed"], "54");
}

#[test]
fn domestic_amounts_match_the_decisions() {
    let table: Value =
        serde_json::from_str(include_str!("../data/fi-travel-allowances.json")).unwrap();
    let e26 = &table["editions"][0];
    assert_eq!(e26["mileage_eur_per_km"]["car"], "0.55");
    assert_eq!(e26["car_benefit_fuel_eur_per_km"], "0.11");
    assert_eq!(e26["domestic"]["full_day"], "54");
    assert_eq!(e26["domestic"]["meal_allowance"], "13.50");
    assert_eq!(e26["domestic"]["night_travel_allowance"], "16");
    let e27 = &table["editions"][1];
    assert_eq!(e27["mileage_eur_per_km"]["car"], "0.58");
    assert_eq!(e27["car_surcharges_eur_per_km"]["trailer"], "0.10");
    assert_eq!(e27["domestic"]["full_day"], "55");
    assert_eq!(e27["domestic"]["meal_allowance_two_meals"], "27.50");
    assert_eq!(e27["domestic"]["night_travel_allowance"], "15");
}

// --- refusals ---------------------------------------------------------------

#[test]
fn refusal_invalid_country_code_never_falls_to_not_listed() {
    let c = compute(json!([with(
        foreign(
            "2026-03-10T06:00:00+02:00",
            "2026-03-10T22:00:00+02:00",
            "ZZ"
        ),
        "days",
        no_free_meals(1)
    )]));
    assert!(!c.approvable);
    assert_eq!(codes(&c), ["TRAVEL_COUNTRY_INVALID"]);
    assert!(c.trips[0].days.is_empty());
    let c = compute(json!([with(
        with(
            foreign(
                "2026-03-10T06:00:00+02:00",
                "2026-03-10T22:00:00+02:00",
                "GB"
            ),
            "destination_region",
            json!("manchester")
        ),
        "days",
        no_free_meals(1)
    )]));
    assert_eq!(codes(&c), ["TRAVEL_COUNTRY_INVALID"]);
}

#[test]
fn refusal_unsettled_territory() {
    let c = compute(json!([with(
        foreign(
            "2026-03-10T06:00:00+02:00",
            "2026-03-10T22:00:00+02:00",
            "MO"
        ),
        "days",
        no_free_meals(1)
    )]));
    assert_eq!(codes(&c), ["TRAVEL_TERRITORY_UNSETTLED"]);
}

#[test]
fn refusal_missing_year_edition() {
    let c = compute(json!([with(
        domestic("2028-03-10T08:00:00+02:00", "2028-03-10T19:00:00+02:00"),
        "days",
        no_free_meals(1)
    )]));
    assert!(!c.approvable);
    assert_eq!(codes(&c), ["TRAVEL_RATES_MISSING"]);
    assert_eq!(c.trips[0].days[0].amount, None);
    assert!(c.incomes_register_lines.is_empty());
    assert!(c.postings.is_empty());
    let c = compute(json!([with(
        domestic("2025-12-10T08:00:00+02:00", "2025-12-10T13:00:00+02:00"),
        "drives",
        json!([{"date": "2025-12-10", "km": "10", "vehicle": "car"}])
    )]));
    assert_eq!(codes(&c), ["TRAVEL_RATES_MISSING"]);
}

#[test]
fn refusal_allowance_above_maximum() {
    let trip = with(
        with(
            domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
            "days",
            no_free_meals(1),
        ),
        "paid",
        json!({"daily_allowances": "60.00"}),
    );
    let c = compute(json!([trip]));
    assert_eq!(codes(&c), ["ALLOWANCE_ABOVE_MAXIMUM"]);
    assert!(c.refusals[0].message.contains("withholding"));
}

#[test]
fn refusal_free_meals_unanswered() {
    let c = compute(json!([domestic(
        "2026-03-10T08:00:00+02:00",
        "2026-03-10T19:00:00+02:00"
    )]));
    assert_eq!(codes(&c), ["TRAVEL_FREE_MEALS_UNANSWERED"]);
    let c = compute(json!([with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
        "days",
        json!([{"free_meals": null}])
    )]));
    assert_eq!(codes(&c), ["TRAVEL_FREE_MEALS_UNANSWERED"]);
}

#[test]
fn refusal_meal_allowance_unanswered() {
    let trip = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T13:00:00+02:00"),
        "meal_allowance",
        json!({"usual_eating_place_available": false, "meals": 1}),
    );
    assert_eq!(
        codes(&compute(json!([trip]))),
        ["TRAVEL_MEAL_ALLOWANCE_UNANSWERED"]
    );
}

#[test]
fn refusal_meal_allowance_with_per_diem() {
    let trip = with(
        with(
            domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
            "days",
            no_free_meals(1),
        ),
        "meal_allowance",
        json!({"usual_eating_place_available": false, "lunch_benefit_usable": false, "employer_arranged_meal": false, "meals": 1}),
    );
    assert_eq!(
        codes(&compute(json!([trip]))),
        ["TRAVEL_MEAL_ALLOWANCE_WITH_PER_DIEM"]
    );
}

#[test]
fn refusal_night_allowance_with_lodging_and_unanswered() {
    let trip = |lodging: Value| {
        with(
            with(
                domestic("2026-03-09T08:00:00+02:00", "2026-03-10T08:00:00+02:00"),
                "days",
                json!([{"free_meals": 0, "lodging": lodging}]),
            ),
            "night_allowance",
            json!(true),
        )
    };
    assert_eq!(
        codes(&compute(json!([trip(json!("provided"))]))),
        ["TRAVEL_NIGHT_ALLOWANCE_WITH_LODGING"]
    );
    assert_eq!(
        codes(&compute(json!([trip(Value::Null)]))),
        ["TRAVEL_NIGHT_LODGING_UNANSWERED"]
    );
    let c = compute(json!([trip(json!("night_travel_or_work"))]));
    assert!(c.approvable);
    assert_eq!(c.totals.night_allowances, "0.00");
}

#[test]
fn refusal_night_allowance_with_leisure() {
    let trip = with(
        with(
            with(
                domestic("2026-03-11T19:00:00+02:00", "2026-03-15T23:00:00+02:00"),
                "leisure",
                json!({
                    "counted_from": "2026-03-11T19:00:00+02:00",
                    "counted_until": "2026-03-12T20:00:00+02:00",
                    "outbound_travel_minutes": 0,
                    "return_travel_minutes": 240
                }),
            ),
            "days",
            no_free_meals(2),
        ),
        "night_allowance",
        json!(true),
    );
    assert_eq!(
        codes(&compute(json!([trip]))),
        ["TRAVEL_NIGHT_ALLOWANCE_WITH_LEISURE"]
    );
}

#[test]
fn refusal_free_and_company_paid_meal_on_the_same_day() {
    let c = compute(json!([with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
        "days",
        json!([{"free_meals": 1, "company_paid_meals": ["8.00"]}])
    )]));
    assert_eq!(codes(&c), ["TRAVEL_FREE_AND_PAID_MEAL_SAME_DAY"]);
}

#[test]
fn refusal_company_paid_meal_on_the_return_half() {
    let trip = with(
        foreign(
            "2026-03-08T08:00:00+02:00",
            "2026-03-09T11:00:00+02:00",
            "SE",
        ),
        "days",
        json!([
            {"free_meals": 0, "end_country": "SE"},
            {"free_meals": 0, "company_paid_meals": ["12.00"]}
        ]),
    );
    assert_eq!(
        codes(&compute(json!([trip]))),
        ["TRAVEL_PAID_MEAL_ON_RETURN_HALF"]
    );
}

#[test]
fn refusal_foreign_trip_through_finland() {
    let trip = with(
        foreign(
            "2026-03-08T08:00:00+02:00",
            "2026-03-11T09:00:00+02:00",
            "SE",
        ),
        "days",
        json!([
            {"free_meals": 0, "end_country": "SE"},
            {"free_meals": 0, "end_country": "FI"},
            {"free_meals": 0, "end_country": "EE"}
        ]),
    );
    assert_eq!(
        codes(&compute(json!([trip]))),
        ["TRAVEL_FOREIGN_DAYS_IN_FINLAND"]
    );
}

#[test]
fn refusal_foreign_day_country_unanswered() {
    let trip = with(
        foreign(
            "2026-03-08T08:00:00+02:00",
            "2026-03-09T09:00:00+02:00",
            "SE",
        ),
        "days",
        json!([{"free_meals": 0}]),
    );
    assert_eq!(
        codes(&compute(json!([trip]))),
        ["TRAVEL_COUNTRY_UNANSWERED"]
    );
}

#[test]
fn refusal_situations_the_sources_do_not_settle() {
    for (situation, code) in [
        ("starts_from_holiday_home", "TRAVEL_FROM_HOLIDAY_HOME"),
        ("workplace_changes", "TRAVEL_WORKPLACE_CHANGE"),
        (
            "border_crossing_allowance",
            "TRAVEL_BORDER_CROSSING_ALLOWANCE",
        ),
        ("posting", "TRAVEL_POSTING"),
        ("work_abroad", "TRAVEL_WORK_ABROAD"),
        ("special_industry_daily_journey", "TRAVEL_SPECIAL_INDUSTRY"),
    ] {
        let trip = with(
            with(
                domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
                "days",
                no_free_meals(1),
            ),
            "situations",
            json!([situation]),
        );
        let c = compute(json!([trip]));
        assert_eq!(codes(&c), [code], "{situation}");
        assert!(!c.approvable);
    }
}

#[test]
fn refusal_trip_to_the_regular_workplace() {
    let mut trip = domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00");
    trip["places"]["destination_place_ids"] = json!(["office"]);
    let c = compute(json!([trip]));
    assert_eq!(codes(&c), ["TRAVEL_TO_REGULAR_WORKPLACE"]);
    assert_eq!(c.totals.total, "0.00");
}

// --- the regular-workplace register ----------------------------------------

#[test]
fn the_register_classifies_the_trip() {
    let trip = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
        "days",
        no_free_meals(1),
    );
    let c = compute(json!([trip]));
    assert!(c.approvable);
    let out = as_json(&c);
    assert_eq!(out["trips"][0]["workplace"], "special_work_place");
    assert!(
        !out["notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["rule"] == "workplace_from_trip_input"),
        "the classification no longer comes from the trip input"
    );
    // A caller cannot supply the classification.
    let mut typed = domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00");
    typed["workplace"] = json!("special_work_place");
    assert!(serde_json::from_value::<ClaimInput>(
        json!({"employee_id": "maija", "trips": [typed]})
    )
    .is_err());
}

#[test]
fn without_a_regular_workplace_entry_the_trip_computes_nothing() {
    let (places, facts) = register();
    // Only the relationship: no workplace entry on the trip's date.
    let facts: Vec<ClaimantFact> = facts.into_iter().take(1).collect();
    let trip = with(
        with(
            domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
            "days",
            no_free_meals(1),
        ),
        "drives",
        json!([{"date": "2026-03-10", "km": "100", "vehicle": "car"}]),
    );
    let mut paid = trip.clone();
    paid["paid"] = json!({"daily_allowances": "54.00", "mileage": "55.00"});
    // The second trip states paid amounts: still nothing is payable.
    let c = compute_claim(&claim(json!([trip.clone(), paid])), &places, &facts, now()).unwrap();
    assert_eq!(
        codes(&c),
        ["REGULAR_WORKPLACE_UNKNOWN", "REGULAR_WORKPLACE_UNKNOWN"]
    );
    assert!(!c.approvable);
    assert_eq!(c.totals.total, "0.00");
    assert_eq!(as_json(&c)["trips"][0]["workplace"], Value::Null);
    // Without a relationship, the relationship is what is missing.
    let c = compute_claim(&claim(json!([trip])), &places, &[], now()).unwrap();
    assert_eq!(codes(&c), ["CLAIMANT_RELATIONSHIP_UNKNOWN"]);
    assert_eq!(c.totals.total, "0.00");
}

#[test]
fn the_register_refusals_reach_the_result() {
    let base = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
        "days",
        no_free_meals(1),
    );
    for (destinations, code) in [
        (json!(["home"]), "TRIP_TO_HOME"),
        (json!(["nowhere"]), "PLACE_NOT_FOUND"),
        // A secondary workplace and a registered special site: split the trip.
        (json!(["branch", "customer"]), "TRAVEL_WORKPLACE_CHANGE"),
    ] {
        let mut trip = base.clone();
        trip["places"]["destination_place_ids"] = destinations.clone();
        let c = compute(json!([trip]));
        assert_eq!(codes(&c), [code], "{destinations}");
        assert_eq!(c.totals.total, "0.00", "{destinations}");
    }
}

#[test]
fn every_refusal_code_is_tested() {
    // Keep this list equal to `refusal::*`; each code above has its test.
    use aggountant_domain::travel::fi::refusal::*;
    let all = [
        ALLOWANCE_ABOVE_MAXIMUM,
        TRAVEL_RATES_MISSING,
        TRAVEL_COUNTRY_INVALID,
        TRAVEL_COUNTRY_UNANSWERED,
        TRAVEL_TERRITORY_UNSETTLED,
        TRAVEL_FREE_MEALS_UNANSWERED,
        TRAVEL_MEAL_ALLOWANCE_UNANSWERED,
        TRAVEL_MEAL_ALLOWANCE_WITH_PER_DIEM,
        TRAVEL_NIGHT_LODGING_UNANSWERED,
        TRAVEL_NIGHT_ALLOWANCE_WITH_LODGING,
        TRAVEL_NIGHT_ALLOWANCE_WITH_LEISURE,
        TRAVEL_NIGHT_ALLOWANCE_ABROAD,
        TRAVEL_FREE_AND_PAID_MEAL_SAME_DAY,
        TRAVEL_PAID_MEAL_ON_RETURN_HALF,
        TRAVEL_FOREIGN_DAYS_IN_FINLAND,
        TRAVEL_FROM_HOLIDAY_HOME,
        TRAVEL_WORKPLACE_CHANGE,
        TRAVEL_BORDER_CROSSING_ALLOWANCE,
        TRAVEL_POSTING,
        TRAVEL_WORK_ABROAD,
        TRAVEL_SPECIAL_INDUSTRY,
        TRAVEL_TO_REGULAR_WORKPLACE,
    ];
    let source = include_str!("travel_fi.rs");
    for code in all {
        assert!(
            source.matches(&format!("\"{code}\"")).count() >= 1,
            "{code} has no refusal test"
        );
    }
}

// --- review findings --------------------------------------------------------

#[test]
fn meal_allowance_is_zero_when_the_employer_arranged_a_meal() {
    let trip = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T13:00:00+02:00"),
        "meal_allowance",
        json!({"usual_eating_place_available": false, "lunch_benefit_usable": false, "employer_arranged_meal": true, "meals": 1}),
    );
    let c = compute(json!([trip]));
    assert!(c.approvable);
    assert_eq!(c.totals.meal_allowances, "0.00");
    let trip = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T13:00:00+02:00"),
        "meal_allowance",
        json!({"usual_eating_place_available": false, "lunch_benefit_usable": false, "meals": 1}),
    );
    assert_eq!(
        codes(&compute(json!([trip]))),
        ["TRAVEL_MEAL_ALLOWANCE_UNANSWERED"]
    );
}

#[test]
fn two_meal_allowances_need_a_trip_over_eleven_hours() {
    let meal = json!({"usual_eating_place_available": false, "lunch_benefit_usable": false, "employer_arranged_meal": false, "meals": 2});
    let mut short = with(
        domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
        "meal_allowance",
        meal.clone(),
    );
    short["distances_km"]["from_departure"] = json!("10");
    let input = claim(json!([short]));
    assert_eq!(
        run(&input).unwrap_err().error_code(),
        "TRAVEL_INPUT_INVALID"
    );
    let mut long = with(
        domestic("2026-03-10T07:00:00+02:00", "2026-03-10T19:30:00+02:00"),
        "meal_allowance",
        meal,
    );
    long["distances_km"]["from_departure"] = json!("10");
    assert_eq!(compute(json!([long])).totals.meal_allowances, "27.00");
}

#[test]
fn an_omitted_regular_workplace_distance_is_an_error() {
    let mut trip = domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00");
    trip["distances_km"] = json!({"from_departure": "80", "from_home": "80"});
    assert!(serde_json::from_value::<ClaimInput>(
        json!({ "employee_id": "maija", "trips": [trip.clone()] })
    )
    .is_err());
    trip["distances_km"]["from_regular_workplace"] = Value::Null;
    trip["days"] = no_free_meals(1);
    // The register records a regular workplace, so the distance is required.
    assert_eq!(
        run(&claim(json!([trip.clone()]))).unwrap_err().error_code(),
        "TRAVEL_INPUT_INVALID"
    );
    // A claimant recorded as having no regular workplace has no such distance.
    let (places, mut facts) = register();
    facts.truncate(1);
    facts.push(
        serde_json::from_value(json!({"fact_id": 9, "employee_id": "maija",
            "change": {"assert": {"valid_from": "2020-01-01", "valid_to": null,
                "fact": {"kind": "workplace", "role": "none", "basis": "board_member_only"}}},
            "recorded_at": "2020-01-01T00:00:00Z", "confirmed_by": "Maija Meikäläinen",
            "evidence_hash": null, "supersedes": null, "reason": null}))
        .unwrap(),
    );
    let c = compute_claim(&claim(json!([trip])), &places, &facts, now()).unwrap();
    assert_eq!(daily(&c), "54.00");
}

fn short_drive(date: &str, km: &str) -> Value {
    with(
        domestic(
            &format!("{date}T08:00:00+02:00"),
            &format!("{date}T12:00:00+02:00"),
        ),
        "drives",
        json!([{"date": date, "km": km, "vehicle": "car"}]),
    )
}

#[test]
fn paid_amounts_are_also_checked_against_the_claim_maximum() {
    // Each trip's exact maximum is 5.5055 (10.01 km × 0.55); paying 5.51 on
    // each passes the trip check but totals 11.02 > round(11.011) = 11.01.
    let trip = with(
        short_drive("2026-03-10", "10.01"),
        "paid",
        json!({"mileage": "5.51"}),
    );
    let c = compute(json!([trip.clone(), trip]));
    assert_eq!(codes(&c), ["ALLOWANCE_ABOVE_MAXIMUM"]);
    assert_eq!(c.refusals[0].trip, None);
}

#[test]
fn monthly_vouchers_add_up_to_the_claim() {
    let c = compute(json!([
        short_drive("2026-01-10", "10.01"),
        short_drive("2026-02-10", "10.01")
    ]));
    assert_eq!(c.totals.mileage, "11.01");
    let credits: Vec<&str> = c
        .postings
        .iter()
        .map(|m| m.lines.last().unwrap().amount.as_str())
        .collect();
    assert_eq!(credits, ["5.51", "5.50"]);
}

#[test]
fn unpaid_allowances_leave_no_kilometres_or_types_in_the_report() {
    let unpaid = with(
        with(
            with(
                domestic("2026-03-09T08:00:00+02:00", "2026-03-09T16:00:00+02:00"),
                "days",
                no_free_meals(1),
            ),
            "drives",
            json!([{"date": "2026-03-09", "km": "100", "vehicle": "car"}]),
        ),
        "paid",
        json!({"mileage": "0", "daily_allowances": "0"}),
    );
    let paid = with(
        with(
            domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
            "days",
            no_free_meals(1),
        ),
        "drives",
        json!([{"date": "2026-03-10", "km": "200", "vehicle": "car"}]),
    );
    let c = compute(json!([unpaid, paid]));
    assert_eq!(
        as_json(&c)["incomes_register_lines"],
        json!([
            {"income_type": "311", "amount": "110.00", "kilometers": 200},
            {"income_type": "331", "amount": "54.00", "allowance_codes": [3]}
        ])
    );
}

// --- input errors -----------------------------------------------------------

#[test]
fn malformed_trip_input_is_an_error() {
    let bad = |trip: Value| run(&claim(json!([trip]))).unwrap_err().error_code();
    assert_eq!(
        bad(domestic(
            "2026-03-10T19:00:00+02:00",
            "2026-03-10T08:00:00+02:00"
        )),
        "TRAVEL_INPUT_INVALID"
    );
    assert_eq!(
        bad(with(
            domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
            "drives",
            json!([{"date": "2026-03-12", "km": "10", "vehicle": "car"}])
        )),
        "TRAVEL_INPUT_INVALID"
    );
    assert_eq!(
        bad(with(
            domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
            "drives",
            json!([{"date": "2026-03-10", "km": "10", "vehicle": "moped", "surcharges": ["trailer"]}])
        )),
        "TRAVEL_INPUT_INVALID"
    );
    // Times need an explicit offset.
    let input =
        serde_json::from_value::<ClaimInput>(json!({"employee_id": "maija", "trips": [domestic(
            "2026-03-10T08:00:00",
            "2026-03-10T19:00:00+02:00"
        )]}));
    assert!(input.is_err());
    // Amounts are strings with at most two decimals.
    let input =
        serde_json::from_value::<ClaimInput>(json!({"employee_id": "maija", "trips": [with(
            domestic("2026-03-10T08:00:00+02:00", "2026-03-10T19:00:00+02:00"),
            "drives",
            json!([{"date": "2026-03-10", "km": "10.123", "vehicle": "car"}])
        )]}));
    assert!(input.is_err());
}
