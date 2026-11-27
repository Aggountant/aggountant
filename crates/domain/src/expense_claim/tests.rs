//! The draft's operations and review, with Esimerkki Oy's synthetic claimant
//! Maija: regular workplace the Tampere office, a secondary workplace in Oulu
//! and a customer site in Turku.

use super::*;
use serde_json::json;

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

fn at(s: &str) -> Instant {
    Instant::parse(s).unwrap()
}

fn now() -> Instant {
    at("2026-03-12T10:00:00Z")
}

/// A complete one-day trip to the customer, driven with Maija's own car.
fn trip_record() -> TripRecord {
    serde_json::from_value(json!({
        "source": "user_statement",
        "purpose": "Asiakastapaaminen",
        "destination": "Asiakas Oy, Turku",
        "route": "Tampere–Turku–Tampere",
        "travel_mode": "oma auto",
        "trip": {
            "kind": "domestic",
            "start": "2026-03-10T07:30:00+02:00",
            "end": "2026-03-10T19:15:00+02:00",
            "places": {"start_place_id": "home", "destination_place_ids": ["customer"]},
            "distances_km": {"from_departure": "160", "from_home": "160", "from_regular_workplace": "158"},
            "days": [{"free_meals": 1, "company_paid_meals": []}]
        },
        "drives": [{"drive": {"date": "2026-03-10", "km": "330", "vehicle": "car"},
                    "own_or_controlled_vehicle": true}]
    }))
    .unwrap()
}

fn line_record(date: &str, amount: &str, hash: Option<Hash>, trip: Option<&str>) -> LineRecord {
    serde_json::from_value(json!({
        "source": "user_statement",
        "receipt_hash": hash,
        "receipt_date": date,
        "amount": amount,
        "description": "Pysäköinti",
        "trip_id": trip,
    }))
    .unwrap()
}

fn hash(n: u8) -> Hash {
    Hash::from_bytes([n; 32])
}

fn draft() -> ClaimDraft {
    ClaimDraft::new("maija-2026-03-10", "maija", now()).unwrap()
}

fn reviewed(d: &ClaimDraft, others: &Others) -> DraftReview {
    let (places, facts) = register();
    review(
        d,
        &Register {
            places: &places,
            facts: &facts,
        },
        others,
        now(),
    )
}

fn codes(findings: &[Finding]) -> Vec<&'static str> {
    findings.iter().map(|f| f.code).collect()
}

#[test]
fn a_complete_single_trip_claim_is_ready_and_computed() {
    let mut d = draft();
    let id = d.add_trip(None, trip_record(), now()).unwrap();
    assert_eq!(id, "t1");
    d.add_line(
        None,
        line_record("2026-03-10", "12.40", Some(hash(1)), Some("t1")),
        now(),
    )
    .unwrap();
    d.validate().unwrap();
    let r = reviewed(&d, &Others::default());
    assert!(r.open_questions.is_empty(), "{:?}", r.open_questions);
    assert!(r.refusals.is_empty(), "{:?}", r.refusals);
    assert!(r.checks.is_empty(), "{:?}", r.checks);
    assert!(r.ready_for_approval);
    // A full domestic day (one free meal does not halve it) and 330 km by car.
    let c = r.allowances.as_ref().unwrap();
    assert_eq!(c.totals.daily_allowances, "54.00");
    assert_eq!(r.totals.receipts, "12.40");
    assert_eq!(
        Decimal::from_str_exact(&r.totals.total).unwrap(),
        Decimal::from_str_exact(&c.totals.total).unwrap() + Decimal::new(1240, 2)
    );
    assert_eq!(r.trips[0].workplace, Some(Workplace::SpecialWorkPlace));
    assert!(!r.trips[0].recorded_afterwards);
}

#[test]
fn every_missing_answer_is_an_open_question_with_a_code() {
    let mut d = draft();
    let record: TripRecord = serde_json::from_value(json!({
        "source": "user_statement",
        "trip": {
            "kind": "domestic",
            "start": "2026-03-10T07:30:00+02:00",
            "end": "2026-03-10T19:15:00+02:00",
            "places": {"destination_place_ids": ["customer"]},
            "distances_km": {"from_departure": "160", "from_home": "160", "from_regular_workplace": "158"}
        },
        "drives": [{"drive": {"date": "2026-03-10", "km": "330", "vehicle": "car"}}]
    }))
    .unwrap();
    d.add_trip(None, record, now()).unwrap();
    d.add_line(None, line_record("2026-03-10", "5.00", None, None), now())
        .unwrap();
    let r = reviewed(&d, &Others::default());
    let q = codes(&r.open_questions);
    for expected in [
        finding::TRIP_STATUTORY_FIELD_MISSING,
        finding::TRIP_START_PLACE_UNANSWERED,
        finding::DRIVE_VEHICLE_OWNERSHIP_UNANSWERED,
        refusal::TRAVEL_FREE_MEALS_UNANSWERED,
        finding::TRAVEL_COMPANY_PAID_MEALS_UNANSWERED,
        finding::LINE_RECEIPT_MISSING,
    ] {
        assert!(q.contains(&expected), "{expected} missing from {q:?}");
    }
    let fields: Vec<&str> = r
        .open_questions
        .iter()
        .filter(|f| f.code == finding::TRIP_STATUTORY_FIELD_MISSING)
        .filter_map(|f| f.field.as_deref())
        .collect();
    assert_eq!(fields, ["purpose", "destination", "travel_mode", "route"]);
    let meals = r
        .open_questions
        .iter()
        .find(|f| f.code == refusal::TRAVEL_FREE_MEALS_UNANSWERED)
        .unwrap();
    assert_eq!(meals.trip_id.as_deref(), Some("t1"));
    assert_eq!(meals.day, Some(1));
    assert!(meals.message.starts_with("Trip t1"), "{}", meals.message);
    assert!(!r.ready_for_approval);
}

#[test]
fn an_explicit_empty_company_paid_meal_list_answers_the_question() {
    let mut d = draft();
    let mut record = trip_record();
    record.trip["days"] = json!([{"free_meals": 0}]);
    d.add_trip(None, record.clone(), now()).unwrap();
    let r = reviewed(&d, &Others::default());
    assert_eq!(
        codes(&r.open_questions),
        [finding::TRAVEL_COMPANY_PAID_MEALS_UNANSWERED]
    );
    record.trip["days"] = json!([{"free_meals": 0, "company_paid_meals": []}]);
    d.revise_trip("t1", record, now()).unwrap();
    assert!(reviewed(&d, &Others::default()).ready_for_approval);
}

#[test]
fn a_vehicle_not_the_claimants_refuses_its_mileage() {
    let mut d = draft();
    let mut record = trip_record();
    record.drives[0].own_or_controlled_vehicle = Some(false);
    d.add_trip(None, record, now()).unwrap();
    let r = reviewed(&d, &Others::default());
    assert_eq!(codes(&r.refusals), [finding::DRIVE_VEHICLE_NOT_OWN]);
    assert_eq!(r.refusals[0].field.as_deref(), Some("drives[0]"));
}

#[test]
fn recorded_afterwards_is_after_the_end_of_the_following_month() {
    let end = at("2026-03-10T17:15:00Z");
    assert!(!recorded_afterwards(end, at("2026-04-30T12:00:00Z")));
    assert!(recorded_afterwards(end, at("2026-05-01T12:00:00Z")));
    // 30 April 22:30 UTC is already 1 May in Finland (summer time).
    assert!(recorded_afterwards(end, at("2026-04-30T22:30:00Z")));
    // A trip ending on 31 January: the next month is February.
    let jan = at("2026-01-31T15:00:00Z");
    assert!(!recorded_afterwards(jan, at("2026-02-28T12:00:00Z")));
    assert!(recorded_afterwards(jan, at("2026-03-01T12:00:00Z")));
}

#[test]
fn a_late_recording_is_a_check_not_a_block() {
    let mut d = draft();
    d.add_trip(None, trip_record(), at("2026-06-01T09:00:00Z"))
        .unwrap();
    let r = reviewed(&d, &Others::default());
    assert_eq!(codes(&r.checks), [finding::TRIP_RECORDED_AFTERWARDS]);
    assert!(r.trips[0].recorded_afterwards);
    assert!(r.ready_for_approval);
}

#[test]
fn overlapping_trips_are_refused_within_and_across_drafts() {
    let mut d = draft();
    d.add_trip(None, trip_record(), now()).unwrap();
    d.add_trip(None, trip_record(), now()).unwrap();
    let r = reviewed(&d, &Others::default());
    assert_eq!(codes(&r.refusals), [finding::TRIP_OVERLAP]);
    assert_eq!(r.refusals[0].trip_id.as_deref(), Some("t2"));

    d.remove_trip("t2").unwrap();
    let others = Others {
        trips: vec![OtherTrip {
            claim_id: "maija-2026-03-01".into(),
            trip_id: "t4".into(),
            start: at("2026-03-10T15:00:00Z"),
            end: at("2026-03-10T20:00:00Z"),
        }],
        ..Others::default()
    };
    let r = reviewed(&d, &others);
    assert_eq!(codes(&r.refusals), [finding::TRIP_OVERLAP]);
    assert!(r.refusals[0].message.contains("maija-2026-03-01"));
}

#[test]
fn receipts_booked_or_repeated_are_refused_and_similar_ones_checked() {
    let mut d = draft();
    d.add_trip(None, trip_record(), now()).unwrap();
    d.add_line(
        None,
        line_record("2026-03-10", "12.40", Some(hash(1)), Some("t1")),
        now(),
    )
    .unwrap();
    d.add_line(
        None,
        line_record("2026-03-10", "12.40", Some(hash(1)), Some("t1")),
        now(),
    )
    .unwrap();
    d.add_line(
        None,
        line_record("2026-03-10", "12.4", Some(hash(2)), Some("t1")),
        now(),
    )
    .unwrap();
    let others = Others {
        booked: [(hash(2), vec![17])].into_iter().collect(),
        ..Others::default()
    };
    let r = reviewed(&d, &others);
    assert_eq!(
        codes(&r.refusals),
        [finding::RECEIPT_DUPLICATE, finding::RECEIPT_ALREADY_BOOKED]
    );
    assert_eq!(r.refusals[0].line_id.as_deref(), Some("l2"));
    assert!(r.refusals[1].message.contains("17"));
    assert_eq!(codes(&r.checks), [finding::RECEIPT_POSSIBLE_DUPLICATE]);
    assert_eq!(r.checks[0].line_id.as_deref(), Some("l3"));
}

#[test]
fn a_receipt_outside_every_trip_is_checked() {
    let mut d = draft();
    d.add_trip(None, trip_record(), now()).unwrap();
    d.add_line(
        None,
        line_record("2026-03-11", "8.00", Some(hash(3)), None),
        now(),
    )
    .unwrap();
    d.add_line(
        None,
        line_record("2026-03-10", "9.00", Some(hash(4)), None),
        now(),
    )
    .unwrap();
    let r = reviewed(&d, &Others::default());
    assert_eq!(codes(&r.checks), [finding::RECEIPT_OUTSIDE_TRIPS]);
    assert_eq!(r.checks[0].line_id.as_deref(), Some("l1"));
}

#[test]
fn receipt_lines_follow_the_trips_register_snapshot() {
    let mut d = draft();
    // A trip to the secondary workplace: travel and lodging costs only.
    let mut secondary = trip_record();
    secondary.trip["places"]["destination_place_ids"] = json!(["branch"]);
    d.add_trip(None, secondary, now()).unwrap();
    d.add_line(
        None,
        line_record("2026-03-10", "89.00", Some(hash(5)), Some("t1")),
        now(),
    )
    .unwrap();
    let r = reviewed(&d, &Others::default());
    assert_eq!(
        codes(&r.open_questions),
        [finding::LINE_COST_KIND_UNANSWERED]
    );
    let mut line = d.lines[0].record.clone();
    line.cost_kind = Some(CostKind::Other);
    d.revise_line("l1", line.clone(), now()).unwrap();
    let r = reviewed(&d, &Others::default());
    assert_eq!(
        codes(&r.refusals),
        [finding::LINE_COST_NOT_TRAVEL_OR_LODGING]
    );
    line.cost_kind = Some(CostKind::Lodging);
    d.revise_line("l1", line, now()).unwrap();
    assert!(reviewed(&d, &Others::default()).refusals.is_empty());

    // A trip to the regular workplace: no tax-free travel cost.
    let mut regular = trip_record();
    regular.trip["start"] = json!("2026-03-11T07:30:00+02:00");
    regular.trip["end"] = json!("2026-03-11T19:15:00+02:00");
    regular.drives[0].drive["date"] = json!("2026-03-11");
    regular.trip["places"]["destination_place_ids"] = json!(["office"]);
    d.add_trip(None, regular, now()).unwrap();
    d.add_line(
        None,
        line_record("2026-03-11", "6.00", Some(hash(6)), Some("t2")),
        now(),
    )
    .unwrap();
    let r = reviewed(&d, &Others::default());
    assert_eq!(
        codes(&r.refusals),
        [
            refusal::TRAVEL_TO_REGULAR_WORKPLACE,
            finding::LINE_TRIP_COSTS_NOT_TAX_FREE
        ]
    );
}

#[test]
fn without_a_register_entry_the_trip_and_its_costs_wait_for_the_answer() {
    let mut d = draft();
    d.add_trip(None, trip_record(), now()).unwrap();
    d.add_line(
        None,
        line_record("2026-03-10", "12.40", Some(hash(1)), Some("t1")),
        now(),
    )
    .unwrap();
    let (places, _) = register();
    let r = review(
        &d,
        &Register {
            places: &places,
            facts: &[],
        },
        &Others::default(),
        now(),
    );
    assert_eq!(codes(&r.open_questions), ["CLAIMANT_RELATIONSHIP_UNKNOWN"]);
    assert_eq!(codes(&r.refusals), [finding::LINE_TRIP_COSTS_NOT_TAX_FREE]);
    assert_eq!(r.trips[0].workplace, None);
}

#[test]
fn a_trip_the_computation_rejects_is_a_refusal_of_that_trip_only() {
    let mut d = draft();
    let mut bad = trip_record();
    bad.trip["end"] = json!("2026-03-10T06:00:00+02:00");
    d.add_trip(Some("broken"), bad, now()).unwrap();
    let mut good = trip_record();
    good.trip["start"] = json!("2026-03-11T07:30:00+02:00");
    good.trip["end"] = json!("2026-03-11T19:15:00+02:00");
    good.drives[0].drive["date"] = json!("2026-03-11");
    d.add_trip(None, good, now()).unwrap();
    let r = reviewed(&d, &Others::default());
    assert_eq!(codes(&r.refusals), [code::TRAVEL_INPUT_INVALID]);
    assert_eq!(r.refusals[0].trip_id.as_deref(), Some("broken"));
    assert_eq!(
        r.refusals[0].message,
        "Trip broken: end must be after start"
    );
    assert_eq!(r.allowances.unwrap().trips.len(), 1);
}

#[test]
fn kilometres_far_from_the_distance_are_checked() {
    let mut d = draft();
    let mut few = trip_record();
    few.drives[0].drive["km"] = json!("100");
    d.add_trip(None, few, now()).unwrap();
    assert_eq!(
        codes(&reviewed(&d, &Others::default()).checks),
        [finding::DRIVE_KM_BELOW_DISTANCE]
    );
    let mut many = trip_record();
    many.drives[0].drive["km"] = json!("520");
    d.revise_trip("t1", many, now()).unwrap();
    assert_eq!(
        codes(&reviewed(&d, &Others::default()).checks),
        [finding::DRIVE_KM_ABOVE_ROUTE]
    );
}

#[test]
fn writes_validate_typed_fields() {
    let mut d = draft();
    let mut record = trip_record();
    record.trip["id"] = json!("x");
    assert_eq!(
        d.add_trip(None, record, now()).unwrap_err().code,
        code::TRAVEL_INPUT_INVALID
    );
    let mut record = trip_record();
    record.trip["kind"] = json!("cruise");
    assert_eq!(
        d.add_trip(None, record, now()).unwrap_err().code,
        code::TRAVEL_INPUT_INVALID
    );
    let mut record = trip_record();
    record.purpose = Some("  ".into());
    assert_eq!(
        d.add_trip(None, record, now()).unwrap_err().code,
        code::CLAIM_INPUT_INVALID
    );
    assert_eq!(
        d.add_trip(Some("Bad Id"), trip_record(), now())
            .unwrap_err()
            .code,
        code::CLAIM_INPUT_INVALID
    );
    for amount in ["0", "-1", "1.234", "1,20", "", ".5", "1e3"] {
        assert_eq!(
            d.add_line(None, line_record("2026-03-10", amount, None, None), now())
                .unwrap_err()
                .code,
            code::CLAIM_INPUT_INVALID,
            "{amount}"
        );
    }
    assert_eq!(
        d.add_line(
            None,
            line_record("2026-03-10", "1.00", None, Some("t9")),
            now()
        )
        .unwrap_err()
        .code,
        code::CLAIM_TRIP_NOT_FOUND
    );
    assert_eq!(d.revision, 0, "a refused write changes nothing");
    assert!(d.trips.is_empty() && d.lines.is_empty());
}

#[test]
fn a_trip_with_lines_is_not_removed_and_ids_are_not_reused_while_taken() {
    let mut d = draft();
    d.add_trip(None, trip_record(), now()).unwrap();
    d.add_line(
        None,
        line_record("2026-03-10", "1.00", Some(hash(1)), Some("t1")),
        now(),
    )
    .unwrap();
    assert_eq!(
        d.remove_trip("t1").unwrap_err().code,
        code::CLAIM_TRIP_HAS_LINES
    );
    assert_eq!(
        d.add_trip(Some("t1"), trip_record(), now())
            .unwrap_err()
            .code,
        code::CLAIM_ITEM_EXISTS
    );
    d.remove_line("l1").unwrap();
    d.remove_trip("t1").unwrap();
    assert_eq!(d.revision, 4);
    assert_eq!(
        d.remove_trip("t1").unwrap_err().code,
        code::CLAIM_TRIP_NOT_FOUND
    );
}

#[test]
fn a_stored_draft_that_does_not_validate_is_refused() {
    let mut d = draft();
    d.add_trip(None, trip_record(), now()).unwrap();
    d.validate().unwrap();
    let mut edited = d.clone();
    edited.trips[0].record.trip["kind"] = json!("cruise");
    assert_eq!(
        edited.validate().unwrap_err().code,
        code::TRAVEL_INPUT_INVALID
    );
    let mut edited = d.clone();
    edited.trips.push(edited.trips[0].clone());
    assert_eq!(edited.validate().unwrap_err().code, code::CLAIM_ITEM_EXISTS);
    let mut edited = d.clone();
    edited.schema_version = 2;
    assert_eq!(
        edited.validate().unwrap_err().code,
        code::CLAIM_DRAFT_INVALID
    );
    let mut value = serde_json::to_value(&d).unwrap();
    value["trips"][0]["record"]["mileage_paid"] = json!("100");
    assert!(serde_json::from_value::<ClaimDraft>(value).is_err());
}

#[test]
fn generated_claim_ids_come_from_the_claimant_and_the_first_date() {
    let date = Date::parse("2026-03-10").unwrap();
    assert_eq!(
        generated_claim_id("maija", date, |_| false),
        "maija-2026-03-10"
    );
    assert_eq!(
        generated_claim_id("Maija M.", date, |id| id == "maija-m-2026-03-10"),
        "maija-m-2026-03-10-2"
    );
    assert_eq!(
        generated_claim_id("ÄÖ", date, |_| false),
        "claim-2026-03-10"
    );
    check_claim_id(&generated_claim_id(&"x".repeat(200), date, |_| false)).unwrap();
    for bad in ["", "../x", "A", "a b", "a/b", "-a", "a..b"] {
        assert!(check_claim_id(bad).is_err(), "{bad}");
    }
    let mut d = draft();
    d.add_line(None, line_record("2026-03-12", "1.00", None, None), now())
        .unwrap();
    d.add_trip(None, trip_record(), now()).unwrap();
    assert_eq!(d.first_content_date(), Some(date));
}
