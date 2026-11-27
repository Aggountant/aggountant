//! An expense claim draft (kululaskun luonnos, ADR-11 sections 1, 2 and 4):
//! the trips and receipt lines of one claimant's claim while it is being
//! collected, and what is still missing from it.
//!
//! The draft is program-owned: it changes only through the operations here,
//! each of which validates the typed fields, and [`review`] answers with the
//! open questions, the refusals that block approval and the plausibility
//! checks. Nothing is computed from the draft by hand: a trip's scheduled
//! allowances come from the travel computation ([`crate::travel::fi`]), which
//! receives the trip as it is stored here, so the trip's fields have one
//! schema, the computation's.
//!
//! Three kinds of finding, so a skill can branch on them:
//!
//! - an **open question** is an answer the user has not given; an empty
//!   answer is never taken as "none", because a missing free meal or a
//!   missing workplace entry would make an allowance too large;
//! - a **refusal** is something the law or its sources do not let the program
//!   approve as it stands;
//! - a **check** is a plausibility finding that does not block approval but
//!   is shown in the preview and the document.
//!
//! Questions and refusals block approval, never the writing of the draft.

use std::collections::{BTreeMap, BTreeSet};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use aggountant_model::{Date, Hash, Instant};

use crate::bank_days::finnish_local_time;
use crate::travel::fi::{
    compute_claim, refusal, ClaimComputation, ClaimInput, TripInput, TripKind, Workplace,
};
use crate::workplace::{ClaimantFact, Place};

/// The version of the draft file's shape.
pub const DRAFT_SCHEMA_VERSION: u32 = 1;
/// The travel computation's own limit for one claim.
const MAX_TRIPS: usize = 100;
const MAX_LINES: usize = 500;
const MAX_TEXT_CHARS: usize = 2000;
/// The largest single receipt line, in euros; a larger amount is a typo or
/// not an expense claim.
const MAX_LINE_AMOUNT: i64 = 1_000_000;

// ---------------------------------------------------------------------------
// Refusal codes of the draft's own writes and reads
// ---------------------------------------------------------------------------

pub mod code {
    pub const CLAIM_INPUT_INVALID: &str = "CLAIM_INPUT_INVALID";
    pub const CLAIM_ID_INVALID: &str = "CLAIM_ID_INVALID";
    pub const CLAIM_DRAFT_INVALID: &str = "CLAIM_DRAFT_INVALID";
    pub const CLAIM_TRIP_NOT_FOUND: &str = "CLAIM_TRIP_NOT_FOUND";
    pub const CLAIM_LINE_NOT_FOUND: &str = "CLAIM_LINE_NOT_FOUND";
    pub const CLAIM_ITEM_EXISTS: &str = "CLAIM_ITEM_EXISTS";
    pub const CLAIM_TRIP_HAS_LINES: &str = "CLAIM_TRIP_HAS_LINES";
    pub const TRAVEL_INPUT_INVALID: &str = "TRAVEL_INPUT_INVALID";
}

/// Finding codes: open questions, refusals and checks.
pub mod finding {
    // Open questions.
    pub const TRIP_STATUTORY_FIELD_MISSING: &str = "TRIP_STATUTORY_FIELD_MISSING";
    pub const TRIP_START_PLACE_UNANSWERED: &str = "TRIP_START_PLACE_UNANSWERED";
    pub const TRAVEL_COMPANY_PAID_MEALS_UNANSWERED: &str = "TRAVEL_COMPANY_PAID_MEALS_UNANSWERED";
    pub const DRIVE_VEHICLE_OWNERSHIP_UNANSWERED: &str = "DRIVE_VEHICLE_OWNERSHIP_UNANSWERED";
    pub const LINE_RECEIPT_MISSING: &str = "LINE_RECEIPT_MISSING";
    pub const LINE_COST_KIND_UNANSWERED: &str = "LINE_COST_KIND_UNANSWERED";
    // Refusals.
    pub const DRIVE_VEHICLE_NOT_OWN: &str = "DRIVE_VEHICLE_NOT_OWN";
    pub const TRIP_OVERLAP: &str = "TRIP_OVERLAP";
    pub const RECEIPT_ALREADY_BOOKED: &str = "RECEIPT_ALREADY_BOOKED";
    pub const RECEIPT_DUPLICATE: &str = "RECEIPT_DUPLICATE";
    pub const LINE_TRIP_COSTS_NOT_TAX_FREE: &str = "LINE_TRIP_COSTS_NOT_TAX_FREE";
    pub const LINE_COST_NOT_TRAVEL_OR_LODGING: &str = "LINE_COST_NOT_TRAVEL_OR_LODGING";
    // Checks.
    pub const TRIP_RECORDED_AFTERWARDS: &str = "TRIP_RECORDED_AFTERWARDS";
    pub const DRIVE_KM_BELOW_DISTANCE: &str = "DRIVE_KM_BELOW_DISTANCE";
    pub const DRIVE_KM_ABOVE_ROUTE: &str = "DRIVE_KM_ABOVE_ROUTE";
    pub const RECEIPT_OUTSIDE_TRIPS: &str = "RECEIPT_OUTSIDE_TRIPS";
    pub const RECEIPT_POSSIBLE_DUPLICATE: &str = "RECEIPT_POSSIBLE_DUPLICATE";
    pub const CLAIM_DRAFT_UNREADABLE: &str = "CLAIM_DRAFT_UNREADABLE";
}

/// The travel computation's refusals that are answers the user has not
/// given, rather than situations the program cannot approve.
const QUESTION_CODES: &[&str] = &[
    refusal::TRAVEL_FREE_MEALS_UNANSWERED,
    refusal::TRAVEL_COUNTRY_UNANSWERED,
    refusal::TRAVEL_MEAL_ALLOWANCE_UNANSWERED,
    refusal::TRAVEL_NIGHT_LODGING_UNANSWERED,
    "REGULAR_WORKPLACE_UNKNOWN",
    "CLAIMANT_RELATIONSHIP_UNKNOWN",
];

/// A write or a stored draft the program refuses.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct DraftRefusal {
    pub code: &'static str,
    pub message: String,
}

fn refuse<T>(code: &'static str, message: impl Into<String>) -> Result<T, DraftRefusal> {
    Err(DraftRefusal {
        code,
        message: message.into(),
    })
}

// ---------------------------------------------------------------------------
// The stored draft
// ---------------------------------------------------------------------------

/// Where a trip or a line comes from (ADR-11 section 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// The user stated it.
    UserStatement,
    /// It was imported from a record, such as a travel system or a logbook.
    ImportedRecord,
    /// The agent reconstructed it (for example from a calendar) and the user
    /// confirmed it drive by drive.
    ConfirmedReconstruction,
}

/// What a receipt line paid for, asked when it matters: on a trip to a
/// secondary workplace only travel and lodging costs are tax-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostKind {
    Travel,
    Lodging,
    Other,
}

/// One claimant's claim while it is collected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimDraft {
    pub schema_version: u32,
    pub claim_id: String,
    /// The claimant, a person in the personal-data register.
    pub employee_id: String,
    pub created_at: Instant,
    /// Counts the writes; every write adds one.
    pub revision: u64,
    pub trips: Vec<DraftTrip>,
    pub lines: Vec<DraftLine>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftTrip {
    pub trip_id: String,
    /// When the trip first entered the draft.
    pub recorded_at: Instant,
    /// When it was last revised.
    pub revised_at: Option<Instant>,
    pub record: TripRecord,
}

/// A trip as `claim trip add` and `claim trip revise` take it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TripRecord {
    pub source: Source,
    /// The purpose of the trip (ennakkoperintäasetus 17 § 2 mom).
    #[serde(default)]
    pub purpose: Option<String>,
    /// Where the trip went (17 § 2 mom).
    #[serde(default)]
    pub destination: Option<String>,
    /// The route, needed when mileage is claimed (17 § 2 and 3 mom).
    #[serde(default)]
    pub route: Option<String>,
    /// How the claimant travelled (17 § 2 mom).
    #[serde(default)]
    pub travel_mode: Option<String>,
    /// Evidence artifacts of the trip (tickets, a meeting invitation).
    #[serde(default)]
    pub evidence: Vec<Hash>,
    /// Free explanation.
    #[serde(default)]
    pub note: Option<String>,
    /// The trip as the travel computation takes it (`claim compute --help`),
    /// without `id` and `drives`: the draft supplies those.
    pub trip: Value,
    /// Drives with the claimant's own or controlled vehicle.
    #[serde(default)]
    pub drives: Vec<DraftDrive>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftDrive {
    /// The drive as the travel computation takes it.
    pub drive: Value,
    /// Whether the claimant owns or controls the vehicle; mileage is tax-free
    /// only for such a vehicle (ennakkoperintäasetus 17 § 3 mom).
    #[serde(default)]
    pub own_or_controlled_vehicle: Option<bool>,
    /// Evidence of the drive (a parking receipt, a logbook page).
    #[serde(default)]
    pub evidence: Vec<Hash>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftLine {
    pub line_id: String,
    pub recorded_at: Instant,
    pub revised_at: Option<Instant>,
    pub record: LineRecord,
}

/// A receipt line: a cost the claimant paid and the company reimburses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineRecord {
    pub source: Source,
    /// The receipt artifact (`artifact attach --file`).
    #[serde(default)]
    pub receipt_hash: Option<Hash>,
    /// Why there is no receipt, when there is none.
    #[serde(default)]
    pub no_receipt_reason: Option<String>,
    pub receipt_date: Date,
    /// What the claimant paid, in euros, as a decimal string.
    pub amount: String,
    pub description: String,
    /// The trip the cost belongs to.
    #[serde(default)]
    pub trip_id: Option<String>,
    #[serde(default)]
    pub cost_kind: Option<CostKind>,
    #[serde(default)]
    pub note: Option<String>,
}

impl ClaimDraft {
    pub fn new(claim_id: &str, employee_id: &str, now: Instant) -> Result<Self, DraftRefusal> {
        check_claim_id(claim_id)?;
        if employee_id.trim().is_empty() {
            return refuse(code::CLAIM_INPUT_INVALID, "employee_id must not be empty");
        }
        Ok(ClaimDraft {
            schema_version: DRAFT_SCHEMA_VERSION,
            claim_id: claim_id.to_string(),
            employee_id: employee_id.to_string(),
            created_at: now,
            revision: 0,
            trips: Vec::new(),
            lines: Vec::new(),
        })
    }

    /// Validate everything the operations validate, for a draft read back
    /// from its file: a hand-edited draft that fails is refused, not used.
    pub fn validate(&self) -> Result<(), DraftRefusal> {
        if self.schema_version != DRAFT_SCHEMA_VERSION {
            return refuse(
                code::CLAIM_DRAFT_INVALID,
                format!(
                    "schema_version {} is not {DRAFT_SCHEMA_VERSION}",
                    self.schema_version
                ),
            );
        }
        check_claim_id(&self.claim_id)?;
        if self.employee_id.trim().is_empty() {
            return refuse(code::CLAIM_INPUT_INVALID, "employee_id must not be empty");
        }
        if self.revision == 0 {
            return refuse(code::CLAIM_DRAFT_INVALID, "revision must be at least 1");
        }
        if self.trips.len() > MAX_TRIPS || self.lines.len() > MAX_LINES {
            return refuse(
                code::CLAIM_INPUT_INVALID,
                format!("a claim holds at most {MAX_TRIPS} trips and {MAX_LINES} lines"),
            );
        }
        let mut seen = BTreeSet::new();
        for trip in &self.trips {
            check_item_id("trip_id", &trip.trip_id)?;
            if !seen.insert(trip.trip_id.as_str()) {
                return refuse(
                    code::CLAIM_ITEM_EXISTS,
                    format!("trip {} appears twice", trip.trip_id),
                );
            }
            validate_trip(&trip.trip_id, &trip.record)?;
        }
        let mut seen_lines = BTreeSet::new();
        for line in &self.lines {
            check_item_id("line_id", &line.line_id)?;
            if !seen_lines.insert(line.line_id.as_str()) {
                return refuse(
                    code::CLAIM_ITEM_EXISTS,
                    format!("line {} appears twice", line.line_id),
                );
            }
            self.validate_line(&line.record)?;
        }
        Ok(())
    }

    /// Add a trip; returns its id (`t1`, `t2`, … unless the caller names it).
    pub fn add_trip(
        &mut self,
        trip_id: Option<&str>,
        record: TripRecord,
        now: Instant,
    ) -> Result<String, DraftRefusal> {
        let trip_id = match trip_id {
            Some(id) => {
                check_item_id("trip_id", id)?;
                if self.trip(id).is_some() {
                    return refuse(
                        code::CLAIM_ITEM_EXISTS,
                        format!("trip {id} is already in claim {}", self.claim_id),
                    );
                }
                id.to_string()
            }
            None => next_id("t", self.trips.iter().map(|t| t.trip_id.as_str())),
        };
        if self.trips.len() >= MAX_TRIPS {
            return refuse(
                code::CLAIM_INPUT_INVALID,
                format!("a claim holds at most {MAX_TRIPS} trips; start another claim"),
            );
        }
        validate_trip(&trip_id, &record)?;
        self.trips.push(DraftTrip {
            trip_id: trip_id.clone(),
            recorded_at: now,
            revised_at: None,
            record,
        });
        self.revision += 1;
        Ok(trip_id)
    }

    /// Replace a trip's record; its recording time stays.
    pub fn revise_trip(
        &mut self,
        trip_id: &str,
        record: TripRecord,
        now: Instant,
    ) -> Result<(), DraftRefusal> {
        validate_trip(trip_id, &record)?;
        let trip = self.trip_mut(trip_id)?;
        trip.record = record;
        trip.revised_at = Some(now);
        self.revision += 1;
        Ok(())
    }

    pub fn remove_trip(&mut self, trip_id: &str) -> Result<(), DraftRefusal> {
        self.trip_mut(trip_id)?;
        let attached: Vec<&str> = self
            .lines
            .iter()
            .filter(|l| l.record.trip_id.as_deref() == Some(trip_id))
            .map(|l| l.line_id.as_str())
            .collect();
        if !attached.is_empty() {
            return refuse(
                code::CLAIM_TRIP_HAS_LINES,
                format!(
                    "lines {} belong to trip {trip_id}; revise or remove them first",
                    attached.join(", ")
                ),
            );
        }
        self.trips.retain(|t| t.trip_id != trip_id);
        self.revision += 1;
        Ok(())
    }

    /// Add a receipt line; returns its id (`l1`, `l2`, … unless named).
    pub fn add_line(
        &mut self,
        line_id: Option<&str>,
        record: LineRecord,
        now: Instant,
    ) -> Result<String, DraftRefusal> {
        let line_id = match line_id {
            Some(id) => {
                check_item_id("line_id", id)?;
                if self.lines.iter().any(|l| l.line_id == id) {
                    return refuse(
                        code::CLAIM_ITEM_EXISTS,
                        format!("line {id} is already in claim {}", self.claim_id),
                    );
                }
                id.to_string()
            }
            None => next_id("l", self.lines.iter().map(|l| l.line_id.as_str())),
        };
        if self.lines.len() >= MAX_LINES {
            return refuse(
                code::CLAIM_INPUT_INVALID,
                format!("a claim holds at most {MAX_LINES} lines; start another claim"),
            );
        }
        self.validate_line(&record)?;
        self.lines.push(DraftLine {
            line_id: line_id.clone(),
            recorded_at: now,
            revised_at: None,
            record,
        });
        self.revision += 1;
        Ok(line_id)
    }

    pub fn revise_line(
        &mut self,
        line_id: &str,
        record: LineRecord,
        now: Instant,
    ) -> Result<(), DraftRefusal> {
        self.validate_line(&record)?;
        let line = self
            .lines
            .iter_mut()
            .find(|l| l.line_id == line_id)
            .ok_or_else(|| line_not_found(line_id))?;
        line.record = record;
        line.revised_at = Some(now);
        self.revision += 1;
        Ok(())
    }

    pub fn remove_line(&mut self, line_id: &str) -> Result<(), DraftRefusal> {
        if !self.lines.iter().any(|l| l.line_id == line_id) {
            return Err(line_not_found(line_id));
        }
        self.lines.retain(|l| l.line_id != line_id);
        self.revision += 1;
        Ok(())
    }

    pub fn trip(&self, trip_id: &str) -> Option<&DraftTrip> {
        self.trips.iter().find(|t| t.trip_id == trip_id)
    }

    fn trip_mut(&mut self, trip_id: &str) -> Result<&mut DraftTrip, DraftRefusal> {
        let claim_id = self.claim_id.clone();
        self.trips
            .iter_mut()
            .find(|t| t.trip_id == trip_id)
            .ok_or_else(|| DraftRefusal {
                code: code::CLAIM_TRIP_NOT_FOUND,
                message: format!("claim {claim_id} has no trip {trip_id}"),
            })
    }

    /// The date of the draft's first content (a trip's Finnish start date or
    /// a receipt date), from which a claim id is generated.
    pub fn first_content_date(&self) -> Option<Date> {
        let trips = self
            .trips
            .iter()
            .filter_map(|t| trip_input(&t.trip_id, &t.record).ok())
            .map(|t| finnish_date(t.start));
        let lines = self.lines.iter().map(|l| l.record.receipt_date);
        trips.chain(lines).min()
    }

    /// Every trip as the travel computation takes it, with its id.
    pub fn trip_inputs(&self) -> Vec<(String, Result<TripInput, DraftRefusal>)> {
        self.trips
            .iter()
            .map(|t| (t.trip_id.clone(), trip_input(&t.trip_id, &t.record)))
            .collect()
    }

    /// Every artifact the draft refers to: receipts and evidence.
    pub fn artifacts(&self) -> BTreeSet<Hash> {
        let mut out = BTreeSet::new();
        for trip in &self.trips {
            out.extend(trip.record.evidence.iter().copied());
            for drive in &trip.record.drives {
                out.extend(drive.evidence.iter().copied());
            }
        }
        for line in &self.lines {
            out.extend(line.record.receipt_hash);
        }
        out
    }

    fn validate_line(&self, record: &LineRecord) -> Result<(), DraftRefusal> {
        amount(&record.amount)?;
        text("description", Some(&record.description))?;
        text("no_receipt_reason", record.no_receipt_reason.as_ref())?;
        text("note", record.note.as_ref())?;
        if record.receipt_hash.is_some() && record.no_receipt_reason.is_some() {
            return refuse(
                code::CLAIM_INPUT_INVALID,
                "a line has either receipt_hash or no_receipt_reason, not both",
            );
        }
        if let Some(trip_id) = &record.trip_id {
            if self.trip(trip_id).is_none() {
                return refuse(
                    code::CLAIM_TRIP_NOT_FOUND,
                    format!("claim {} has no trip {trip_id}", self.claim_id),
                );
            }
        }
        Ok(())
    }
}

fn line_not_found(line_id: &str) -> DraftRefusal {
    DraftRefusal {
        code: code::CLAIM_LINE_NOT_FOUND,
        message: format!("the claim has no line {line_id}"),
    }
}

/// A claim id: it names a directory, so it is restricted to a safe alphabet.
pub fn check_claim_id(id: &str) -> Result<(), DraftRefusal> {
    if !safe_id(id, 80) {
        return refuse(
            code::CLAIM_ID_INVALID,
            format!("claim id {id:?} must be 1..80 characters a-z, 0-9, '.', '_' or '-', starting with a letter or digit"),
        );
    }
    Ok(())
}

fn check_item_id(field: &str, id: &str) -> Result<(), DraftRefusal> {
    if !safe_id(id, 64) {
        return refuse(
            code::CLAIM_INPUT_INVALID,
            format!("{field} {id:?} must be 1..64 characters a-z, 0-9, '.', '_' or '-', starting with a letter or digit"),
        );
    }
    Ok(())
}

fn safe_id(id: &str, max: usize) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= max
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(b))
        && !id.contains("..")
}

/// The generated id of a new claim: the claimant and the first content date,
/// with a number added when the id is taken.
pub fn generated_claim_id(employee_id: &str, first: Date, taken: impl Fn(&str) -> bool) -> String {
    let mut slug = String::new();
    for c in employee_id.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug: String = slug.trim_end_matches('-').chars().take(40).collect();
    let slug = slug.trim_end_matches('-');
    let base = if slug.is_empty() {
        format!("claim-{first}")
    } else {
        format!("{slug}-{first}")
    };
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !taken(id))
        .expect("an unused id exists")
}

fn next_id<'a>(prefix: &str, existing: impl Iterator<Item = &'a str>) -> String {
    let used: BTreeSet<&str> = existing.collect();
    (1..)
        .map(|n| format!("{prefix}{n}"))
        .find(|id| !used.contains(id.as_str()))
        .expect("an unused id exists")
}

fn text(field: &str, value: Option<&String>) -> Result<(), DraftRefusal> {
    let Some(value) = value else { return Ok(()) };
    if value.trim().is_empty() {
        return refuse(
            code::CLAIM_INPUT_INVALID,
            format!("{field} must not be empty; leave it out when unknown"),
        );
    }
    if value.chars().count() > MAX_TEXT_CHARS {
        return refuse(
            code::CLAIM_INPUT_INVALID,
            format!("{field} is longer than {MAX_TEXT_CHARS} characters"),
        );
    }
    if value
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return refuse(
            code::CLAIM_INPUT_INVALID,
            format!("{field} contains control characters"),
        );
    }
    Ok(())
}

/// A positive euro amount with at most two decimals.
fn amount(s: &str) -> Result<Decimal, DraftRefusal> {
    let shape_ok = !s.is_empty()
        && s.bytes().all(|b| b.is_ascii_digit() || b == b'.')
        && !s.starts_with('.')
        && !s.ends_with('.')
        && s.bytes().filter(|b| *b == b'.').count() <= 1;
    let parsed = shape_ok
        .then(|| Decimal::from_str_exact(s).ok())
        .flatten()
        .filter(|d| d.scale() <= 2 && *d > Decimal::ZERO && *d <= Decimal::from(MAX_LINE_AMOUNT));
    parsed.ok_or_else(|| DraftRefusal {
        code: code::CLAIM_INPUT_INVALID,
        message: format!(
            "amount {s:?} must be a positive euro amount with at most two decimals, such as \"12.40\", at most {MAX_LINE_AMOUNT}"
        ),
    })
}

fn validate_trip(trip_id: &str, record: &TripRecord) -> Result<(), DraftRefusal> {
    for (field, value) in [
        ("purpose", &record.purpose),
        ("destination", &record.destination),
        ("route", &record.route),
        ("travel_mode", &record.travel_mode),
        ("note", &record.note),
    ] {
        text(field, value.as_ref())?;
    }
    trip_input(trip_id, record).map(|_| ())
}

/// The trip as the travel computation takes it: the stored `trip` object with
/// the draft's id and drives.
fn trip_input(trip_id: &str, record: &TripRecord) -> Result<TripInput, DraftRefusal> {
    let Value::Object(object) = &record.trip else {
        return refuse(
            code::TRAVEL_INPUT_INVALID,
            format!("trip {trip_id}: trip must be a JSON object as `claim compute --help` describes one trip"),
        );
    };
    for key in ["id", "drives"] {
        if object.contains_key(key) {
            return refuse(
                code::TRAVEL_INPUT_INVALID,
                format!("trip {trip_id}: trip.{key} is not given here; the draft supplies the id from trip_id and the drives from the record's drives list"),
            );
        }
    }
    let mut object = object.clone();
    object.insert("id".into(), Value::String(trip_id.to_string()));
    object.insert(
        "drives".into(),
        Value::Array(record.drives.iter().map(|d| d.drive.clone()).collect()),
    );
    serde_json::from_value(Value::Object(object)).map_err(|e| DraftRefusal {
        code: code::TRAVEL_INPUT_INVALID,
        message: format!("trip {trip_id}: {e}"),
    })
}

fn finnish_date(at: Instant) -> Date {
    let d = finnish_local_time(at).date();
    Date::from_ymd(i32::from(d.year()), d.month() as u32, d.day() as u32)
        .expect("a civil date is a valid date")
}

/// Whether a trip is "recorded afterwards" (ADR-11 section 4): recorded after
/// the end of the calendar month following the month it ended in. Claims are
/// collected monthly, so a trip recorded by the end of the next month is the
/// ordinary cycle; one recorded later rests on recollection or a
/// reconstruction, which the preview and the document say. No source gives a
/// number, so the rule follows the claim cycle rather than a day count.
pub fn recorded_afterwards(trip_end: Instant, recorded_at: Instant) -> bool {
    let end = finnish_date(trip_end).inner();
    let next_month_last = end
        .first_of_month()
        .checked_add(jiff::Span::new().months(1))
        .expect("the next month exists")
        .last_of_month();
    finnish_date(recorded_at).inner() > next_month_last
}

// ---------------------------------------------------------------------------
// Review: questions, refusals, checks
// ---------------------------------------------------------------------------

/// One open question, refusal or check, with what it is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub code: &'static str,
    pub message: String,
    pub trip_id: Option<String>,
    pub line_id: Option<String>,
    /// The computed per diem day, day 1 first.
    pub day: Option<usize>,
    /// The record field the finding is about.
    pub field: Option<String>,
}

impl Finding {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Finding {
            code,
            message: message.into(),
            trip_id: None,
            line_id: None,
            day: None,
            field: None,
        }
    }
    fn trip(mut self, id: &str) -> Self {
        self.trip_id = Some(id.to_string());
        self
    }
    fn line(mut self, id: &str) -> Self {
        self.line_id = Some(id.to_string());
        self
    }
    fn field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

/// A trip in another open draft of the same claimant.
#[derive(Debug, Clone)]
pub struct OtherTrip {
    pub claim_id: String,
    pub trip_id: String,
    pub start: Instant,
    pub end: Instant,
}

/// A receipt line in another open draft.
#[derive(Debug, Clone)]
pub struct OtherLine {
    pub claim_id: String,
    pub line_id: String,
    pub receipt_hash: Option<Hash>,
    pub receipt_date: Date,
    pub amount: String,
}

/// What the review compares the draft with, besides the register.
#[derive(Debug, Clone, Default)]
pub struct Others {
    /// Trips of the claimant's other open drafts.
    pub trips: Vec<OtherTrip>,
    /// Lines of every other open draft.
    pub lines: Vec<OtherLine>,
    /// Other drafts that could not be read, so they were not compared.
    pub unreadable: Vec<String>,
    /// Sealed vouchers attaching a receipt of this draft.
    pub booked: BTreeMap<Hash, Vec<u64>>,
}

/// The claimant's register rows, from the personal-data store.
pub struct Register<'a> {
    pub places: &'a [Place],
    pub facts: &'a [ClaimantFact],
}

#[derive(Debug, Clone, Serialize)]
pub struct TripStatus {
    pub trip_id: String,
    pub recorded_afterwards: bool,
    /// What the register makes of the trip's places; `None` when it refuses.
    pub workplace: Option<Workplace>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftTotals {
    /// Scheduled allowances, rounded once per claim.
    pub allowances: String,
    pub receipts: String,
    pub total: String,
}

/// What the draft still needs, and what it computes to so far.
#[derive(Debug, Clone, Serialize)]
pub struct DraftReview {
    /// No open question and no refusal remains.
    pub ready_for_approval: bool,
    pub open_questions: Vec<Finding>,
    pub refusals: Vec<Finding>,
    pub checks: Vec<Finding>,
    pub trips: Vec<TripStatus>,
    /// The travel computation of the draft's trips (`claim compute`); `null`
    /// when the draft has no computable trip.
    pub allowances: Option<ClaimComputation>,
    pub totals: DraftTotals,
}

#[derive(Default)]
struct Findings {
    questions: Vec<Finding>,
    refusals: Vec<Finding>,
    checks: Vec<Finding>,
}

/// Review the draft: compute its trips with the claimant's register and list
/// what is missing, what blocks approval and what looks implausible.
pub fn review(
    draft: &ClaimDraft,
    register: &Register<'_>,
    others: &Others,
    now: Instant,
) -> DraftReview {
    let mut f = Findings::default();
    let inputs: Vec<(String, TripInput)> = draft
        .trip_inputs()
        .into_iter()
        .filter_map(|(id, input)| match input {
            Ok(input) => Some((id, input)),
            Err(refused) => {
                f.refusals
                    .push(Finding::new(refused.code, refused.message).trip(&id));
                None
            }
        })
        .collect();
    let allowances = compute(draft, &inputs, register, now, &mut f);
    let computed = |id: &str| {
        allowances
            .as_ref()
            .and_then(|c| c.trips.iter().find(|t| t.id.as_deref() == Some(id)))
    };

    let mut trips = Vec::new();
    for trip in &draft.trips {
        let Some((_, input)) = inputs.iter().find(|(id, _)| *id == trip.trip_id) else {
            continue;
        };
        let id = trip.trip_id.as_str();
        let record = &trip.record;
        for (field, value) in [
            ("purpose", &record.purpose),
            ("destination", &record.destination),
            ("travel_mode", &record.travel_mode),
        ] {
            if value.is_none() {
                f.questions.push(
                    Finding::new(
                        finding::TRIP_STATUTORY_FIELD_MISSING,
                        format!("Trip {id}: the travel claim must show the trip's {} (ennakkoperintäasetus 17 §).", statutory_name(field)),
                    )
                    .trip(id)
                    .field(field),
                );
            }
        }
        if !record.drives.is_empty() && record.route.is_none() {
            f.questions.push(
                Finding::new(
                    finding::TRIP_STATUTORY_FIELD_MISSING,
                    format!("Trip {id}: mileage is claimed, so the travel claim must show the route the kilometres were driven (ennakkoperintäasetus 17 §)."),
                )
                .trip(id)
                .field("route"),
            );
        }
        let start_place_given = record
            .trip
            .get("places")
            .and_then(Value::as_object)
            .is_some_and(|p| p.contains_key("start_place_id"));
        if !start_place_given {
            f.questions.push(
                Finding::new(
                    finding::TRIP_START_PLACE_UNANSWERED,
                    format!("Trip {id}: where did the trip start? Give trip.places.start_place_id: the registered place (home or a workplace), or null when it started elsewhere."),
                )
                .trip(id)
                .field("trip.places.start_place_id"),
            );
        }
        for (i, drive) in record.drives.iter().enumerate() {
            match drive.own_or_controlled_vehicle {
                None => f.questions.push(
                    Finding::new(
                        finding::DRIVE_VEHICLE_OWNERSHIP_UNANSWERED,
                        format!("Trip {id}, drive {}: was the vehicle the claimant's own or one they control? Mileage is tax-free only for such a vehicle.", i + 1),
                    )
                    .trip(id)
                    .field(format!("drives[{i}].own_or_controlled_vehicle")),
                ),
                Some(false) => f.refusals.push(
                    Finding::new(
                        finding::DRIVE_VEHICLE_NOT_OWN,
                        format!("Trip {id}, drive {}: mileage is tax-free only for a vehicle the claimant owns or controls (ennakkoperintäasetus 17 § 3 mom); remove the drive and claim the real costs as receipt lines.", i + 1),
                    )
                    .trip(id)
                    .field(format!("drives[{i}]")),
                ),
                Some(true) => {}
            }
        }
        if let Some(c) = computed(id) {
            for day in c.days.iter().filter(|d| d.scheduled.is_some()) {
                let answered = record
                    .trip
                    .get("days")
                    .and_then(|days| days.get(day.day - 1))
                    .and_then(Value::as_object)
                    .is_some_and(|d| d.contains_key("company_paid_meals"));
                if !answered {
                    let mut q = Finding::new(
                        finding::TRAVEL_COMPANY_PAID_MEALS_UNANSWERED,
                        format!("Trip {id}, day {}: did the company pay for meals the traveller bought (for example with the company card)? Give company_paid_meals, [] when none.", day.day),
                    )
                    .trip(id)
                    .field(format!("trip.days[{}].company_paid_meals", day.day - 1));
                    q.day = Some(day.day);
                    f.questions.push(q);
                }
            }
        }
        check_km(id, input, &mut f);
        let afterwards = recorded_afterwards(input.end, trip.recorded_at);
        if afterwards {
            f.checks.push(
                Finding::new(
                    finding::TRIP_RECORDED_AFTERWARDS,
                    format!("Trip {id} ended {} and was recorded {}, after the end of the following month: it is marked as recorded afterwards in the preview and the document.", finnish_date(input.end), finnish_date(trip.recorded_at)),
                )
                .trip(id),
            );
        }
        trips.push(TripStatus {
            trip_id: trip.trip_id.clone(),
            recorded_afterwards: afterwards,
            workplace: computed(id).and_then(|c| c.workplace),
        });
    }

    check_overlaps(&inputs, others, &mut f);
    check_lines(draft, &inputs, &trips, others, &mut f);
    for claim_id in &others.unreadable {
        f.checks.push(Finding::new(
            finding::CLAIM_DRAFT_UNREADABLE,
            format!("Claim draft {claim_id} does not validate, so its trips and receipts were not compared with this one; fix or discard it."),
        ));
    }

    let receipts: Decimal = draft
        .lines
        .iter()
        .filter_map(|l| amount(&l.record.amount).ok())
        .sum();
    let allowance_total = allowances
        .as_ref()
        .and_then(|c| Decimal::from_str_exact(&c.totals.total).ok())
        .unwrap_or(Decimal::ZERO);
    let money = |d: Decimal| format!("{:.2}", d);
    DraftReview {
        ready_for_approval: f.questions.is_empty() && f.refusals.is_empty(),
        open_questions: f.questions,
        refusals: f.refusals,
        checks: f.checks,
        trips,
        allowances,
        totals: DraftTotals {
            allowances: money(allowance_total),
            receipts: money(receipts),
            total: money(allowance_total + receipts),
        },
    }
}

fn statutory_name(field: &str) -> &'static str {
    match field {
        "purpose" => "purpose (matkan tarkoitus)",
        "destination" => "destination (kohde)",
        _ => "mode of travel (matkustamistapa)",
    }
}

/// Compute the trips with the register. A trip whose input the computation
/// rejects is a refusal of that trip, and the others are still computed.
fn compute(
    draft: &ClaimDraft,
    inputs: &[(String, TripInput)],
    register: &Register<'_>,
    now: Instant,
    f: &mut Findings,
) -> Option<ClaimComputation> {
    let claim = |trips: Vec<TripInput>| ClaimInput {
        employee_id: draft.employee_id.clone(),
        trips,
    };
    let mut ok: Vec<(String, TripInput)> = Vec::new();
    for (id, input) in inputs {
        match compute_claim(
            &claim(vec![input.clone()]),
            register.places,
            register.facts,
            now,
        ) {
            Ok(_) => ok.push((id.clone(), input.clone())),
            Err(e) => f.refusals.push(
                Finding::new(
                    code::TRAVEL_INPUT_INVALID,
                    format!("Trip {id}: {}", without_trip_number(&e.to_string())),
                )
                .trip(id),
            ),
        }
    }
    if ok.is_empty() {
        return None;
    }
    let ids: Vec<String> = ok.iter().map(|(id, _)| id.clone()).collect();
    let computed = match compute_claim(
        &claim(ok.into_iter().map(|(_, t)| t).collect()),
        register.places,
        register.facts,
        now,
    ) {
        Ok(c) => c,
        Err(e) => {
            f.refusals
                .push(Finding::new(code::TRAVEL_INPUT_INVALID, e.to_string()));
            return None;
        }
    };
    for r in &computed.refusals {
        let id = r.trip.and_then(|n| ids.get(n - 1));
        let message = match (r.trip, id) {
            (Some(n), Some(id)) => r
                .message
                .strip_prefix(&format!("Trip {n}"))
                .map(|rest| format!("Trip {id}{rest}"))
                .unwrap_or_else(|| r.message.clone()),
            _ => r.message.clone(),
        };
        let mut finding = Finding::new(r.code, message);
        finding.trip_id = id.cloned();
        finding.day = r.day;
        if QUESTION_CODES.contains(&r.code) {
            f.questions.push(finding);
        } else {
            f.refusals.push(finding);
        }
    }
    Some(computed)
}

/// The computation's input errors name the trip by its position; the draft
/// names it by id.
fn without_trip_number(message: &str) -> String {
    let message = message
        .strip_prefix("TRAVEL_INPUT_INVALID: ")
        .unwrap_or(message);
    match message.strip_prefix("trip ") {
        Some(rest) => rest
            .split_once(": ")
            .map(|(_, tail)| tail.to_string())
            .unwrap_or_else(|| message.to_string()),
        None => message.to_string(),
    }
}

/// Kilometres against the trip's distance (decision 11 §): a domestic trip
/// driven less than one way, or driven well over a return journey with room
/// for errands and detours, is worth a second look.
fn check_km(id: &str, input: &TripInput, f: &mut Findings) {
    if input.drives.is_empty() {
        return;
    }
    let km: Decimal = input.drives.iter().map(|d| d.km).sum();
    let one_way = input.distances_km.from_departure;
    if input.kind == TripKind::Domestic && km < one_way {
        f.checks.push(
            Finding::new(
                finding::DRIVE_KM_BELOW_DISTANCE,
                format!("Trip {id}: the drives add up to {km} km, less than the {one_way} km one way to the farthest place of work. Fine when part of the trip was made by other means; otherwise check the drives or the distance."),
            )
            .trip(id),
        );
    }
    let generous_return = one_way * Decimal::from(3) + Decimal::from(20);
    if km > generous_return {
        f.checks.push(
            Finding::new(
                finding::DRIVE_KM_ABOVE_ROUTE,
                format!("Trip {id}: the drives add up to {km} km, more than one and a half times the return journey of {one_way} km each way. Explain the detours in the route or the note, or check the kilometres."),
            )
            .trip(id),
        );
    }
}

/// A trip overlapping another trip of the same claimant is refused (ADR-11
/// section 2): no one is on two trips at once.
fn check_overlaps(inputs: &[(String, TripInput)], others: &Others, f: &mut Findings) {
    let overlaps = |a: &TripInput, start: Instant, end: Instant| a.start < end && start < a.end;
    for (i, (id, trip)) in inputs.iter().enumerate() {
        for (other_id, other) in &inputs[..i] {
            if overlaps(trip, other.start, other.end) {
                f.refusals.push(
                    Finding::new(
                        finding::TRIP_OVERLAP,
                        format!("Trip {id} overlaps trip {other_id} of this claim; a claimant is on one trip at a time. Correct the times or remove one."),
                    )
                    .trip(id),
                );
            }
        }
        for other in &others.trips {
            if overlaps(trip, other.start, other.end) {
                f.refusals.push(
                    Finding::new(
                        finding::TRIP_OVERLAP,
                        format!("Trip {id} overlaps trip {} of claim draft {}; a claimant is on one trip at a time.", other.trip_id, other.claim_id),
                    )
                    .trip(id),
                );
            }
        }
    }
}

fn check_lines(
    draft: &ClaimDraft,
    inputs: &[(String, TripInput)],
    trips: &[TripStatus],
    others: &Others,
    f: &mut Findings,
) {
    for (i, line) in draft.lines.iter().enumerate() {
        let id = line.line_id.as_str();
        let record = &line.record;
        if record.receipt_hash.is_none() && record.no_receipt_reason.is_none() {
            f.questions.push(
                Finding::new(
                    finding::LINE_RECEIPT_MISSING,
                    format!("Line {id}: attach the receipt (`artifact attach --file`) and give its receipt_hash, or say why there is none in no_receipt_reason."),
                )
                .line(id)
                .field("receipt_hash"),
            );
        }
        if let Some(hash) = record.receipt_hash {
            if let Some(numbers) = others.booked.get(&hash) {
                let numbers: Vec<String> = numbers.iter().map(u64::to_string).collect();
                f.refusals.push(
                    Finding::new(
                        finding::RECEIPT_ALREADY_BOOKED,
                        format!("Line {id}: the receipt is already the attachment of voucher {}; a receipt is booked once.", numbers.join(", ")),
                    )
                    .line(id),
                );
            }
            let earlier = draft.lines[..i]
                .iter()
                .find(|l| l.record.receipt_hash == Some(hash))
                .map(|l| format!("line {} of this claim", l.line_id));
            let elsewhere = others
                .lines
                .iter()
                .find(|l| l.receipt_hash == Some(hash))
                .map(|l| format!("line {} of claim draft {}", l.line_id, l.claim_id));
            if let Some(other) = earlier.or(elsewhere) {
                f.refusals.push(
                    Finding::new(
                        finding::RECEIPT_DUPLICATE,
                        format!("Line {id}: the same receipt is already {other}."),
                    )
                    .line(id),
                );
            }
        }
        let same_date_amount = |date: Date, amt: &str, hash: Option<Hash>| {
            date == record.receipt_date
                && amount(amt).ok() == amount(&record.amount).ok()
                && (hash.is_none() || hash != record.receipt_hash)
        };
        let similar = draft.lines[..i]
            .iter()
            .find(|l| {
                same_date_amount(
                    l.record.receipt_date,
                    &l.record.amount,
                    l.record.receipt_hash,
                )
            })
            .map(|l| format!("line {} of this claim", l.line_id))
            .or_else(|| {
                others
                    .lines
                    .iter()
                    .find(|l| same_date_amount(l.receipt_date, &l.amount, l.receipt_hash))
                    .map(|l| format!("line {} of claim draft {}", l.line_id, l.claim_id))
            });
        if let Some(other) = similar {
            f.checks.push(
                Finding::new(
                    finding::RECEIPT_POSSIBLE_DUPLICATE,
                    format!("Line {id} has the same date and amount as {other}; check that it is not the same cost twice."),
                )
                .line(id),
            );
        }
        match &record.trip_id {
            Some(trip_id) => line_on_trip(id, trip_id, record, trips, f),
            None if !inputs.is_empty() => {
                let within = inputs.iter().any(|(_, t)| {
                    finnish_date(t.start) <= record.receipt_date
                        && record.receipt_date <= finnish_date(t.end)
                });
                if !within {
                    f.checks.push(
                        Finding::new(
                            finding::RECEIPT_OUTSIDE_TRIPS,
                            format!("Line {id} is dated {}, outside every trip of this claim, and belongs to none. Attach it to its trip (trip_id), or check that it is the company's cost.", record.receipt_date),
                        )
                        .line(id),
                    );
                }
            }
            None => {}
        }
    }
}

/// A cost of a trip follows the trip's register snapshot (ADR-11 section 3):
/// without a workplace entry, or on a trip to the regular workplace, no travel
/// cost is tax-free; on a trip to a secondary workplace only travel and
/// lodging costs are.
fn line_on_trip(
    id: &str,
    trip_id: &str,
    record: &LineRecord,
    trips: &[TripStatus],
    f: &mut Findings,
) {
    let Some(status) = trips.iter().find(|t| t.trip_id == trip_id) else {
        // The trip itself is refused as input; its finding says why.
        return;
    };
    match status.workplace {
        None | Some(Workplace::RegularWorkplace) => f.refusals.push(
            Finding::new(
                finding::LINE_TRIP_COSTS_NOT_TAX_FREE,
                format!("Line {id} belongs to trip {trip_id}, which the claimant register does not make a work trip (see the trip's own finding), so its costs cannot be reimbursed tax-free as travel costs."),
            )
            .line(id)
            .trip(trip_id),
        ),
        Some(Workplace::SecondaryWorkplace) => match record.cost_kind {
            None => f.questions.push(
                Finding::new(
                    finding::LINE_COST_KIND_UNANSWERED,
                    format!("Line {id} belongs to trip {trip_id} to a secondary workplace (toissijainen työpaikka), where only travel and lodging costs are tax-free: is it travel, lodging or other? Give cost_kind."),
                )
                .line(id)
                .field("cost_kind"),
            ),
            Some(CostKind::Other) => f.refusals.push(
                Finding::new(
                    finding::LINE_COST_NOT_TRAVEL_OR_LODGING,
                    format!("Line {id}: on a trip to a secondary workplace (toissijainen työpaikka) only travel and lodging costs are tax-free (TVL 71 § 1 mom); this cost is not one."),
                )
                .line(id),
            ),
            Some(CostKind::Travel | CostKind::Lodging) => {}
        },
        Some(Workplace::SpecialWorkPlace) => {}
    }
}

#[cfg(test)]
mod tests;
