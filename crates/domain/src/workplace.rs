//! The claimant register: dated, confirmed facts about the people who may file
//! an expense claim, and the regular-workplace rules that follow from them
//! (ADR-11 sections 1 and 3).
//!
//! Every fact is bitemporal. `valid_from`/`valid_to` say when it holds,
//! `recorded_at` says when the program learned it. A fact is never edited: a
//! correction is a new fact that supersedes the old one, so "what was in force
//! on D as known at T" stays answerable after a backdated correction. That is
//! what lets a sealed claim show what its approver knew.
//!
//! The functions here are pure. The ledger stores the rows and calls
//! [`validate_place`] and [`validate_fact`] inside its writer transaction; the
//! views and the trip snapshot are folds over the stored rows.
//!
//! The legal grounds are the Tax Administration's guidance
//! *Työmatkakustannusten korvaukset verotuksessa* (VH/630/00.01.00/2026,
//! "ohje" below) and TVL 71, 72 and 72 b §.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use aggountant_model::{Date, Hash, Instant};

/// What kind of place a register entry names. A place is recorded once per
/// person, with the location distances are measured from and the municipality
/// the secondary-workplace rule compares (ohje 4.1: "paikkakunnalla tarkoitetaan
/// kuntaa").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceKind {
    /// The employer company's own premises.
    CompanyPremises,
    /// Premises of a company in the same group or interest group as the
    /// employer (samaan intressipiiriin kuuluva taho, ohje 4.1).
    GroupCompanyPremises,
    /// The person's home.
    Home,
    /// A work space the person arranged for themselves (itse hankittu
    /// työskentelytila, ohje 10.7).
    SelfArrangedSpace,
    /// A customer's premises.
    CustomerSite,
    /// A trade fair, training or meeting venue (ohje 5.2.1).
    Venue,
    Other,
}

/// The role a place has for a person during the entry's validity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkplaceRole {
    /// The primary regular workplace (ensisijainen varsinainen työpaikka).
    RegularPrimary,
    /// Another regular workplace (ohje 3.2: several regular workplaces).
    RegularOther,
    /// A secondary workplace (toissijainen työpaikka, TVL 71 § 1 mom).
    Secondary,
    /// A special work site the register tracks, for example a customer site
    /// visited for a long engagement (erityinen työntekemispaikka).
    TrackedSite,
    /// The person has no regular workplace, stated explicitly (for example an
    /// outside board member). Carries no place.
    None,
}

impl WorkplaceRole {
    /// Whether an entry with this role settles the regular-workplace question
    /// for its dates.
    fn settles_regular_workplace(self) -> bool {
        matches!(
            self,
            WorkplaceRole::RegularPrimary | WorkplaceRole::RegularOther | WorkplaceRole::None
        )
    }

    fn as_str(self) -> &'static str {
        match self {
            WorkplaceRole::RegularPrimary => "regular_primary",
            WorkplaceRole::RegularOther => "regular_other",
            WorkplaceRole::Secondary => "secondary",
            WorkplaceRole::TrackedSite => "tracked_site",
            WorkplaceRole::None => "none",
        }
    }
}

/// Why a workplace entry is what it is: a closed list, each code tied to the
/// guidance passage it rests on. `Other` needs a written explanation. Adding a
/// code later is additive; the stored text of a code never changes meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BasisCode {
    /// The person works at the place permanently (ohje 3.1).
    WorksThereRegularly,
    /// The company's premises are in the person's home (ohje 3.4, first case).
    CompanyPremisesInHome,
    /// Working at home by agreement with no work space at the employer, and
    /// visiting the employer only occasionally (ohje 3.4; 10.7, third paragraph).
    AgreedHomeWorkOccasionalVisits,
    /// Mobile work: where work orders are fetched or tools and materials kept
    /// (ohje 3.5, TVL 72 b § 2 mom).
    MobileWorkBase,
    /// The person may choose between remote work and the employer's premises,
    /// so the premises stay the regular workplace (ohje 10.7, first paragraph;
    /// 3.4, last paragraph).
    RemoteWorkByChoice,
    /// The person works at the employer's premises weekly, or a week a month
    /// (ohje 5.2.2; 10.7 and KHO 2017:178).
    EmployerPremisesWeekly,
    /// Regular work at the employer's or its interest group's premises in
    /// another municipality than the primary regular workplace (ohje 4.1, 5.2.2).
    PremisesInAnotherMunicipality,
    /// A place visited irregularly, as needed (ohje 5.2.1, 5.2.2).
    OccasionalVisits,
    /// Fixed-term work at a place within the temporariness limit (ohje 5.2.2,
    /// last paragraphs; 5.3).
    FixedTermAssignment,
    /// A board member who does not otherwise work in the company (ohje 10.1).
    BoardMemberOnly,
    /// Mobile work with no regular workplace at all; trips start from home
    /// (ohje 3.5, last paragraph).
    MobileWorkNoBase,
    /// None of the above; `basis_explanation` says why.
    Other,
}

impl BasisCode {
    /// Every code, in the order the user guide lists them.
    pub const ALL: [BasisCode; 12] = [
        BasisCode::WorksThereRegularly,
        BasisCode::CompanyPremisesInHome,
        BasisCode::AgreedHomeWorkOccasionalVisits,
        BasisCode::MobileWorkBase,
        BasisCode::RemoteWorkByChoice,
        BasisCode::EmployerPremisesWeekly,
        BasisCode::PremisesInAnotherMunicipality,
        BasisCode::OccasionalVisits,
        BasisCode::FixedTermAssignment,
        BasisCode::BoardMemberOnly,
        BasisCode::MobileWorkNoBase,
        BasisCode::Other,
    ];

    /// The guidance passage the code rests on, as shown with the entry.
    pub fn guidance(self) -> &'static str {
        match self {
            BasisCode::WorksThereRegularly => "VH/630/00.01.00/2026 3.1",
            BasisCode::CompanyPremisesInHome => "VH/630/00.01.00/2026 3.4",
            BasisCode::AgreedHomeWorkOccasionalVisits => "VH/630/00.01.00/2026 3.4, 10.7",
            BasisCode::MobileWorkBase => "VH/630/00.01.00/2026 3.5; TVL 72 b § 2 mom",
            BasisCode::RemoteWorkByChoice => "VH/630/00.01.00/2026 10.7, 3.4",
            BasisCode::EmployerPremisesWeekly => "VH/630/00.01.00/2026 5.2.2, 10.7; KHO 2017:178",
            BasisCode::PremisesInAnotherMunicipality => {
                "VH/630/00.01.00/2026 4.1, 5.2.2; TVL 71 § 1 mom"
            }
            BasisCode::OccasionalVisits => "VH/630/00.01.00/2026 5.2.1, 5.2.2",
            BasisCode::FixedTermAssignment => "VH/630/00.01.00/2026 5.2.2, 5.3",
            BasisCode::BoardMemberOnly => "VH/630/00.01.00/2026 10.1",
            BasisCode::MobileWorkNoBase => "VH/630/00.01.00/2026 3.5",
            BasisCode::Other => "explanation recorded with the entry",
        }
    }

    /// The roles the code can justify. `Other` justifies any role.
    fn allowed_roles(self) -> &'static [WorkplaceRole] {
        use WorkplaceRole::*;
        match self {
            BasisCode::WorksThereRegularly
            | BasisCode::CompanyPremisesInHome
            | BasisCode::AgreedHomeWorkOccasionalVisits
            | BasisCode::MobileWorkBase
            | BasisCode::RemoteWorkByChoice
            | BasisCode::EmployerPremisesWeekly => &[RegularPrimary, RegularOther],
            BasisCode::PremisesInAnotherMunicipality => &[Secondary],
            BasisCode::OccasionalVisits | BasisCode::FixedTermAssignment => &[TrackedSite],
            BasisCode::BoardMemberOnly | BasisCode::MobileWorkNoBase => &[None],
            BasisCode::Other => &[RegularPrimary, RegularOther, Secondary, TrackedSite, None],
        }
    }

    /// The place kinds the code can describe, or `None` for any kind.
    fn allowed_place_kinds(self) -> Option<&'static [PlaceKind]> {
        use PlaceKind::*;
        match self {
            BasisCode::CompanyPremisesInHome => Some(&[Home]),
            BasisCode::AgreedHomeWorkOccasionalVisits => Some(&[Home, SelfArrangedSpace]),
            BasisCode::RemoteWorkByChoice | BasisCode::EmployerPremisesWeekly => {
                Some(&[CompanyPremises, GroupCompanyPremises])
            }
            _ => Option::None,
        }
    }
}

/// The claimant's relationship to the company. Several may hold at once (a
/// managing director who is also a board member).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relationship {
    Employee,
    ManagingDirector,
    BoardMember,
    /// An owner who works in the company without pay (ohje 10.4).
    UnpaidOwner,
    /// Another recipient of a personal wage or fee (EPL 13 §; ohje 10.4, 10.8).
    OtherFeeRecipient,
    /// A contractor or consultant paid on invoice. Accepted as input only to be
    /// refused: their travel costs belong on their own invoice and are never
    /// tax-free (ohje 10.5).
    Contractor,
}

/// Which kind of fact a row records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactKind {
    Workplace,
    Relationship,
    PartOwner,
}

impl FactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            FactKind::Workplace => "workplace",
            FactKind::Relationship => "relationship",
            FactKind::PartOwner => "part_owner",
        }
    }
}

/// What a fact says, by kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum FactBody {
    Workplace {
        /// The place, absent exactly when `role` is `none`.
        #[serde(default)]
        place_id: Option<String>,
        role: WorkplaceRole,
        basis: BasisCode,
        /// Required for basis `other`, optional otherwise.
        #[serde(default)]
        basis_explanation: Option<String>,
    },
    Relationship {
        relationship: Relationship,
    },
    /// Whether the person is a part-owner of the company (the Incomes
    /// Register's "osakasasemassa maksajaan").
    PartOwner {
        part_owner: bool,
    },
}

impl FactBody {
    pub fn kind(&self) -> FactKind {
        match self {
            FactBody::Workplace { .. } => FactKind::Workplace,
            FactBody::Relationship { .. } => FactKind::Relationship,
            FactBody::PartOwner { .. } => FactKind::PartOwner,
        }
    }
}

/// A fact with its validity. `valid_to` is the last day it holds, inclusive;
/// absent means open-ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactEntry {
    pub valid_from: Date,
    #[serde(default)]
    pub valid_to: Option<Date>,
    #[serde(rename = "fact")]
    pub body: FactBody,
}

impl FactEntry {
    fn holds_on(&self, date: Date) -> bool {
        self.valid_from <= date && self.valid_to.is_none_or(|to| date <= to)
    }

    fn overlaps(&self, other: &FactEntry) -> bool {
        let starts_before_other_ends = other.valid_to.is_none_or(|to| self.valid_from <= to);
        let other_starts_before_self_ends = self.valid_to.is_none_or(|to| other.valid_from <= to);
        starts_before_other_ends && other_starts_before_self_ends
    }
}

/// What a row does: assert a fact, or retract the fact it supersedes without
/// putting anything in its place (a retraction has the kind of its target).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactChange {
    Assert(FactEntry),
    Retract,
}

impl FactChange {
    pub fn entry(&self) -> Option<&FactEntry> {
        match self {
            FactChange::Assert(entry) => Some(entry),
            FactChange::Retract => None,
        }
    }
}

/// One stored, never-edited register row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimantFact {
    pub fact_id: i64,
    pub employee_id: String,
    pub change: FactChange,
    pub recorded_at: Instant,
    /// Who confirmed the fact (the user's statement is what is recorded).
    pub confirmed_by: String,
    /// sha256 of an evidence artifact, if one was attached.
    pub evidence_hash: Option<Hash>,
    /// The row this one corrects.
    pub supersedes: Option<i64>,
    /// Why the correction was made; present exactly when `supersedes` is.
    pub reason: Option<String>,
}

/// One stored, never-edited place. A place whose details were wrong is
/// replaced: a new place names it in `replaces`, and the entries that named the
/// old place are corrected to name the new one. A replaced place can no longer
/// be named by an entry or a trip, so a stale id cannot pass for another place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
    pub employee_id: String,
    pub place_id: String,
    pub kind: PlaceKind,
    pub label: String,
    /// Address or other location distances are measured from.
    pub location: String,
    pub municipality: String,
    /// The place this one replaces (the same place with corrected details).
    pub replaces: Option<String>,
    pub recorded_at: Instant,
    pub confirmed_by: String,
}

/// A correction's target and reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Supersede {
    pub fact_id: i64,
    pub reason: String,
}

/// A fact about to be recorded; the store adds `fact_id` and `recorded_at`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFact {
    pub employee_id: String,
    pub change: FactChange,
    pub confirmed_by: String,
    pub evidence_hash: Option<Hash>,
    pub supersedes: Option<Supersede>,
}

/// `claimant place add` input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewPlace {
    pub employee_id: String,
    pub place_id: String,
    pub kind: PlaceKind,
    pub label: String,
    pub location: String,
    pub municipality: String,
    /// The place this one replaces, when it records the same place with
    /// corrected details.
    #[serde(default)]
    pub replaces: Option<String>,
    pub confirmed_by: String,
}

/// `claimant add` input: one fact with its validity and confirmation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactInput {
    pub employee_id: String,
    pub valid_from: Date,
    #[serde(default)]
    pub valid_to: Option<Date>,
    pub fact: FactBody,
    pub confirmed_by: String,
    #[serde(default)]
    pub evidence_hash: Option<Hash>,
}

impl FactInput {
    pub fn into_new_fact(self) -> NewFact {
        NewFact {
            employee_id: self.employee_id,
            change: FactChange::Assert(FactEntry {
                valid_from: self.valid_from,
                valid_to: self.valid_to,
                body: self.fact,
            }),
            confirmed_by: self.confirmed_by,
            evidence_hash: self.evidence_hash,
            supersedes: None,
        }
    }
}

/// `claimant correct` input: the fact corrected, why, and either its
/// replacement or `"retract": true`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionInput {
    pub employee_id: String,
    pub fact_id: i64,
    pub reason: String,
    pub confirmed_by: String,
    #[serde(default)]
    pub evidence_hash: Option<Hash>,
    #[serde(default)]
    pub retract: bool,
    #[serde(default)]
    pub replacement: Option<FactEntry>,
}

impl CorrectionInput {
    pub fn into_new_fact(self) -> Result<NewFact, ClaimantRefusal> {
        let change = match (self.retract, self.replacement) {
            (true, None) => FactChange::Retract,
            (false, Some(entry)) => FactChange::Assert(entry),
            _ => {
                return Err(refuse(
                    RefusalCode::InvalidInput,
                    "a correction gives either \"replacement\" or \"retract\": true, not both \
                     and not neither",
                ))
            }
        };
        Ok(NewFact {
            employee_id: self.employee_id,
            change,
            confirmed_by: self.confirmed_by,
            evidence_hash: self.evidence_hash,
            supersedes: Some(Supersede {
                fact_id: self.fact_id,
                reason: self.reason,
            }),
        })
    }
}

/// Machine-readable refusal codes of the register.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalCode {
    /// No regular-workplace entry is in force on the trip's date.
    RegularWorkplaceUnknown,
    /// No relationship to the company is in force on the date.
    ClaimantRelationshipUnknown,
    /// A contractor or consultant cannot be a claimant.
    ContractorNotClaimant,
    /// The entry contradicts another entry in force on overlapping dates.
    ClaimantFactConflict,
    /// The place cannot be a secondary workplace.
    SecondaryWorkplaceInvalid,
    /// The basis code does not fit the role or the place.
    WorkplaceBasisMismatch,
    /// A trip names the person's home as a destination.
    TripToHome,
    PlaceNotFound,
    PlaceAlreadyRecorded,
    /// The place was replaced by a corrected one.
    PlaceReplaced,
    ClaimantFactNotFound,
    ClaimantFactSuperseded,
    /// The clock is behind the last recorded row, which would break the
    /// "as known at" order.
    ClaimantClockBehind,
    InvalidInput,
}

impl RefusalCode {
    pub fn as_str(self) -> &'static str {
        match self {
            RefusalCode::RegularWorkplaceUnknown => "REGULAR_WORKPLACE_UNKNOWN",
            RefusalCode::ClaimantRelationshipUnknown => "CLAIMANT_RELATIONSHIP_UNKNOWN",
            RefusalCode::ContractorNotClaimant => "CONTRACTOR_NOT_CLAIMANT",
            RefusalCode::ClaimantFactConflict => "CLAIMANT_FACT_CONFLICT",
            RefusalCode::SecondaryWorkplaceInvalid => "SECONDARY_WORKPLACE_INVALID",
            RefusalCode::WorkplaceBasisMismatch => "WORKPLACE_BASIS_MISMATCH",
            RefusalCode::PlaceNotFound => "PLACE_NOT_FOUND",
            RefusalCode::TripToHome => "TRIP_TO_HOME",
            RefusalCode::PlaceAlreadyRecorded => "PLACE_ALREADY_RECORDED",
            RefusalCode::PlaceReplaced => "PLACE_REPLACED",
            RefusalCode::ClaimantFactNotFound => "CLAIMANT_FACT_NOT_FOUND",
            RefusalCode::ClaimantFactSuperseded => "CLAIMANT_FACT_SUPERSEDED",
            RefusalCode::ClaimantClockBehind => "CLAIMANT_CLOCK_BEHIND",
            RefusalCode::InvalidInput => "CLAIMANT_INPUT_INVALID",
        }
    }
}

/// A register refusal: a code to branch on and an explanation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{}: {message}", code.as_str())]
pub struct ClaimantRefusal {
    pub code: RefusalCode,
    pub message: String,
}

impl ClaimantRefusal {
    pub fn error_code(&self) -> &'static str {
        self.code.as_str()
    }
}

fn refuse(code: RefusalCode, message: impl Into<String>) -> ClaimantRefusal {
    ClaimantRefusal {
        code,
        message: message.into(),
    }
}

fn require_text(field: &str, value: &str) -> Result<(), ClaimantRefusal> {
    if value.trim().is_empty() {
        return Err(refuse(
            RefusalCode::InvalidInput,
            format!("{field} must not be empty"),
        ));
    }
    Ok(())
}

/// A place id is a short slug, so it can stand in commands and snapshots
/// without carrying the address.
fn validate_place_id(place_id: &str) -> Result<(), ClaimantRefusal> {
    let ok = !place_id.is_empty()
        && place_id.len() <= 64
        && place_id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
    if !ok {
        return Err(refuse(
            RefusalCode::InvalidInput,
            format!(
                "place_id {place_id:?} must be 1-64 characters of a-z, 0-9 and '-' (for \
                 example \"home\" or \"office-tampere\")"
            ),
        ));
    }
    Ok(())
}

fn same_municipality(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

/// The newest `recorded_at` of a person's rows must not be later than `now`.
fn check_clock(
    now: Instant,
    places: &[Place],
    facts: &[ClaimantFact],
) -> Result<(), ClaimantRefusal> {
    let last = places
        .iter()
        .map(|p| p.recorded_at)
        .chain(facts.iter().map(|f| f.recorded_at))
        .max();
    if let Some(last) = last {
        if now < last {
            return Err(refuse(
                RefusalCode::ClaimantClockBehind,
                format!(
                    "the system clock ({now}) is behind the last register entry ({last}); \
                     fix the clock first, so that \"as known at\" stays in order"
                ),
            ));
        }
    }
    Ok(())
}

/// Refuse a place that was replaced, as known at `at` (`None`: now).
fn replaced_check(
    places: &[Place],
    place_id: &str,
    at: Option<Instant>,
) -> Result<(), ClaimantRefusal> {
    if let Some(by) = places
        .iter()
        .find(|p| p.replaces.as_deref() == Some(place_id) && at.is_none_or(|t| p.recorded_at <= t))
    {
        return Err(refuse(
            RefusalCode::PlaceReplaced,
            format!(
                "place {place_id:?} was replaced by {:?}; name the new place instead",
                by.place_id
            ),
        ));
    }
    Ok(())
}

/// The instant a query reads the register at: the one asked for, or now. A
/// query "as known now" refuses when the clock is behind the last recorded row,
/// because the rows recorded after the clock's time would silently drop out.
pub fn resolve_as_known_at(
    as_known_at: Option<Instant>,
    places: &[Place],
    facts: &[ClaimantFact],
    now: Instant,
) -> Result<Instant, ClaimantRefusal> {
    match as_known_at {
        Some(at) => Ok(at),
        None => {
            check_clock(now, places, facts)?;
            Ok(now)
        }
    }
}

/// Validate a new place against the person's existing places.
pub fn validate_place(
    new: &NewPlace,
    places: &[Place],
    facts: &[ClaimantFact],
    now: Instant,
) -> Result<(), ClaimantRefusal> {
    validate_place_id(&new.place_id)?;
    require_text("label", &new.label)?;
    require_text("location", &new.location)?;
    require_text("municipality", &new.municipality)?;
    require_text("confirmed_by", &new.confirmed_by)?;
    if places.iter().any(|p| p.place_id == new.place_id) {
        return Err(refuse(
            RefusalCode::PlaceAlreadyRecorded,
            format!(
                "place {:?} is already recorded for {}; places are never edited, so record \
                 the corrected place under a new id with \"replaces\": {:?} and correct the \
                 entries that name it",
                new.place_id, new.employee_id, new.place_id
            ),
        ));
    }
    if let Some(old) = &new.replaces {
        if !places.iter().any(|p| &p.place_id == old) {
            return Err(refuse(
                RefusalCode::PlaceNotFound,
                format!(
                    "place {old:?} to be replaced is not recorded for {}",
                    new.employee_id
                ),
            ));
        }
        replaced_check(places, old, None)?;
    }
    check_clock(now, places, facts)
}

/// Rows not superseded as known at `at`, excluding retractions: the facts the
/// register asserts at that time. `None` means as known now (every row).
pub fn known_at(facts: &[ClaimantFact], at: Option<Instant>) -> Vec<&ClaimantFact> {
    let visible: Vec<&ClaimantFact> = facts
        .iter()
        .filter(|f| at.is_none_or(|t| f.recorded_at <= t))
        .collect();
    visible
        .iter()
        .copied()
        .filter(|f| matches!(f.change, FactChange::Assert(_)))
        .filter(|f| !visible.iter().any(|g| g.supersedes == Some(f.fact_id)))
        .collect()
}

/// Validate a new fact (or correction) against the person's rows as known now.
pub fn validate_fact(
    new: &NewFact,
    places: &[Place],
    facts: &[ClaimantFact],
    now: Instant,
) -> Result<(), ClaimantRefusal> {
    require_text("confirmed_by", &new.confirmed_by)?;
    let current = known_at(facts, None);

    let target = match &new.supersedes {
        None => None,
        Some(sup) => {
            require_text("reason", &sup.reason)?;
            let target = facts
                .iter()
                .find(|f| f.fact_id == sup.fact_id && f.employee_id == new.employee_id)
                .ok_or_else(|| {
                    refuse(
                        RefusalCode::ClaimantFactNotFound,
                        format!(
                            "no fact {} for {} in the register",
                            sup.fact_id, new.employee_id
                        ),
                    )
                })?;
            if matches!(target.change, FactChange::Retract) {
                return Err(refuse(
                    RefusalCode::InvalidInput,
                    format!(
                        "fact {} is a retraction; to restore what it retracted, add the fact again",
                        target.fact_id
                    ),
                ));
            }
            if let Some(by) = facts.iter().find(|f| f.supersedes == Some(target.fact_id)) {
                return Err(refuse(
                    RefusalCode::ClaimantFactSuperseded,
                    format!(
                        "fact {} was already corrected by fact {}; correct fact {} instead",
                        target.fact_id, by.fact_id, by.fact_id
                    ),
                ));
            }
            if let (Some(old), Some(new)) = (target.change.entry(), new.change.entry()) {
                if old.body.kind() != new.body.kind() {
                    return Err(refuse(
                        RefusalCode::InvalidInput,
                        format!(
                            "fact {} is a {} fact; its correction cannot be a {} fact",
                            target.fact_id,
                            old.body.kind().as_str(),
                            new.body.kind().as_str()
                        ),
                    ));
                }
            }
            Some(target.fact_id)
        }
    };

    if let Some(entry) = new.change.entry() {
        if let Some(to) = entry.valid_to {
            if to < entry.valid_from {
                return Err(refuse(
                    RefusalCode::InvalidInput,
                    format!("valid_to {to} is before valid_from {}", entry.valid_from),
                ));
            }
        }
        let others: Vec<&FactEntry> = current
            .iter()
            .filter(|f| Some(f.fact_id) != target)
            .filter_map(|f| f.change.entry())
            .filter(|e| e.body.kind() == entry.body.kind() && e.overlaps(entry))
            .collect();
        match &entry.body {
            FactBody::Workplace { .. } => validate_workplace(entry, places, &others)?,
            FactBody::Relationship { relationship } => {
                if *relationship == Relationship::Contractor {
                    return Err(refuse(
                        RefusalCode::ContractorNotClaimant,
                        "a contractor or consultant cannot be a claimant: travel costs paid with \
                         a work compensation are never tax-free, and they belong on the \
                         contractor's own invoice (VH/630/00.01.00/2026 10.5)",
                    ));
                }
                if let Some(clash) = others.iter().find(|o| o.body == entry.body) {
                    return Err(conflict(entry, clash, "the same relationship"));
                }
            }
            FactBody::PartOwner { .. } => {
                if let Some(clash) = others.first() {
                    return Err(conflict(entry, clash, "a part-owner fact"));
                }
            }
        }
    }
    check_clock(now, places, facts)
}

fn conflict(entry: &FactEntry, clash: &FactEntry, what: &str) -> ClaimantRefusal {
    refuse(
        RefusalCode::ClaimantFactConflict,
        format!(
            "{what} is already in force from {}{} on overlapping dates (new entry {}{}); \
             correct the existing entry instead of adding a second one",
            clash.valid_from,
            clash
                .valid_to
                .map(|d| format!(" to {d}"))
                .unwrap_or_default(),
            entry.valid_from,
            entry
                .valid_to
                .map(|d| format!(" to {d}"))
                .unwrap_or_default(),
        ),
    )
}

fn validate_workplace(
    entry: &FactEntry,
    places: &[Place],
    others: &[&FactEntry],
) -> Result<(), ClaimantRefusal> {
    let FactBody::Workplace {
        place_id,
        role,
        basis,
        basis_explanation,
    } = &entry.body
    else {
        unreachable!("validate_workplace is called for workplace entries");
    };
    let place = match (role, place_id) {
        (WorkplaceRole::None, None) => None,
        (WorkplaceRole::None, Some(_)) => {
            return Err(refuse(
                RefusalCode::InvalidInput,
                "role \"none\" names no place; leave place_id out",
            ))
        }
        (_, None) => {
            return Err(refuse(
                RefusalCode::InvalidInput,
                format!("role {:?} needs a place_id", role.as_str()),
            ))
        }
        (_, Some(id)) => {
            let place = places.iter().find(|p| &p.place_id == id).ok_or_else(|| {
                refuse(
                    RefusalCode::PlaceNotFound,
                    format!("place {id:?} is not recorded for this person; add it first"),
                )
            })?;
            replaced_check(places, id, None)?;
            Some(place)
        }
    };

    if *basis == BasisCode::Other
        && basis_explanation
            .as_deref()
            .is_none_or(|text| text.trim().is_empty())
    {
        return Err(refuse(
            RefusalCode::WorkplaceBasisMismatch,
            "basis \"other\" needs basis_explanation saying why the place has this role",
        ));
    }
    if !basis.allowed_roles().contains(role) {
        return Err(refuse(
            RefusalCode::WorkplaceBasisMismatch,
            format!(
                "basis {} ({}) cannot justify role {:?}",
                serde_json::to_string(basis).unwrap_or_default(),
                basis.guidance(),
                role.as_str()
            ),
        ));
    }
    if let (Some(kinds), Some(place)) = (basis.allowed_place_kinds(), place) {
        if !kinds.contains(&place.kind) {
            return Err(refuse(
                RefusalCode::WorkplaceBasisMismatch,
                format!(
                    "basis {} ({}) does not describe a place of kind {}",
                    serde_json::to_string(basis).unwrap_or_default(),
                    basis.guidance(),
                    serde_json::to_string(&place.kind).unwrap_or_default()
                ),
            ));
        }
    }
    if *role == WorkplaceRole::Secondary {
        let place = place.expect("a secondary entry has a place");
        if !matches!(
            place.kind,
            PlaceKind::CompanyPremises | PlaceKind::GroupCompanyPremises
        ) {
            return Err(refuse(
                RefusalCode::SecondaryWorkplaceInvalid,
                "a secondary workplace is a permanent place of business of the employer or \
                 its interest group; a home or a customer's premises cannot be one \
                 (VH/630/00.01.00/2026 4.1, KHO 2017:178)",
            ));
        }
    }

    for other in others {
        let FactBody::Workplace {
            place_id: other_place,
            role: other_role,
            ..
        } = &other.body
        else {
            continue;
        };
        let clash = match (role, other_role) {
            (WorkplaceRole::RegularPrimary, WorkplaceRole::RegularPrimary) => {
                Some("a primary regular workplace")
            }
            (WorkplaceRole::None, WorkplaceRole::TrackedSite)
            | (WorkplaceRole::TrackedSite, WorkplaceRole::None) => None,
            (WorkplaceRole::None, _) => Some("a workplace entry"),
            (_, WorkplaceRole::None) => Some("\"no regular workplace\""),
            _ if place_id.is_some() && place_id == other_place => {
                Some("another role for the same place")
            }
            _ => None,
        };
        if let Some(what) = clash {
            return Err(conflict(entry, other, what));
        }
        let (secondary_place, primary_place) = match (role, other_role) {
            (WorkplaceRole::Secondary, WorkplaceRole::RegularPrimary) => {
                (place_id.as_deref(), other_place.as_deref())
            }
            (WorkplaceRole::RegularPrimary, WorkplaceRole::Secondary) => {
                (other_place.as_deref(), place_id.as_deref())
            }
            _ => continue,
        };
        let municipality = |id: Option<&str>| {
            places
                .iter()
                .find(|p| Some(p.place_id.as_str()) == id)
                .map(|p| p.municipality.as_str())
        };
        if let (Some(a), Some(b)) = (municipality(secondary_place), municipality(primary_place)) {
            if same_municipality(a, b) {
                return Err(refuse(
                    RefusalCode::SecondaryWorkplaceInvalid,
                    format!(
                        "a secondary workplace is in another municipality than the primary \
                         regular workplace, but both are in {a}; a second regular place in the \
                         same municipality is another regular workplace (role \
                         \"regular_other\") (VH/630/00.01.00/2026 4.1, 3.2)"
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// A workplace entry in force, with its place resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkplaceInForce {
    pub fact_id: i64,
    pub role: WorkplaceRole,
    pub place: Option<Place>,
    pub basis: BasisCode,
    pub basis_guidance: &'static str,
    pub basis_explanation: Option<String>,
    pub valid_from: Date,
    pub valid_to: Option<Date>,
}

/// A relationship or part-owner fact in force.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DatedValue<T> {
    pub fact_id: i64,
    pub value: T,
    pub valid_from: Date,
    pub valid_to: Option<Date>,
}

/// What the register says about a person on `date`, as known at `as_known_at`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisterView {
    pub employee_id: String,
    pub date: Date,
    pub as_known_at: Instant,
    /// Whether a regular-workplace entry (a regular workplace, or an explicit
    /// "none") is in force on `date`.
    pub regular_workplace_known: bool,
    pub workplaces: Vec<WorkplaceInForce>,
    pub relationships: Vec<DatedValue<Relationship>>,
    pub part_owner: Option<DatedValue<bool>>,
    /// sha256 over the rows and places this view was built from, so a trip can
    /// freeze it and a later check can tell whether the register changed.
    pub facts_digest: Hash,
}

/// Fold the person's rows into the view for `date` as known at `as_known_at`.
pub fn register_view(
    employee_id: &str,
    places: &[Place],
    facts: &[ClaimantFact],
    date: Date,
    as_known_at: Instant,
) -> RegisterView {
    let in_force: Vec<&ClaimantFact> = known_at(facts, Some(as_known_at))
        .into_iter()
        .filter(|f| f.change.entry().is_some_and(|e| e.holds_on(date)))
        .collect();
    let places_known: Vec<&Place> = places
        .iter()
        .filter(|p| p.recorded_at <= as_known_at)
        .collect();

    let mut workplaces = Vec::new();
    let mut relationships = Vec::new();
    let mut part_owner = None;
    let mut places_used: Vec<&Place> = Vec::new();
    for fact in &in_force {
        let Some(entry) = fact.change.entry() else {
            continue;
        };
        match &entry.body {
            FactBody::Workplace {
                place_id,
                role,
                basis,
                basis_explanation,
            } => {
                let place = place_id
                    .as_ref()
                    .and_then(|id| places_known.iter().find(|p| &p.place_id == id))
                    .copied();
                if let Some(place) = place {
                    places_used.push(place);
                }
                workplaces.push(WorkplaceInForce {
                    fact_id: fact.fact_id,
                    role: *role,
                    place: place.cloned(),
                    basis: *basis,
                    basis_guidance: basis.guidance(),
                    basis_explanation: basis_explanation.clone(),
                    valid_from: entry.valid_from,
                    valid_to: entry.valid_to,
                });
            }
            FactBody::Relationship { relationship } => relationships.push(DatedValue {
                fact_id: fact.fact_id,
                value: *relationship,
                valid_from: entry.valid_from,
                valid_to: entry.valid_to,
            }),
            FactBody::PartOwner { part_owner: value } => {
                part_owner = Some(DatedValue {
                    fact_id: fact.fact_id,
                    value: *value,
                    valid_from: entry.valid_from,
                    valid_to: entry.valid_to,
                })
            }
        }
    }
    let regular_workplace_known = workplaces
        .iter()
        .any(|w| w.role.settles_regular_workplace());
    RegisterView {
        employee_id: employee_id.to_string(),
        date,
        as_known_at,
        regular_workplace_known,
        workplaces,
        relationships,
        part_owner,
        facts_digest: digest(&in_force, &places_used),
    }
}

/// sha256 over the canonical JSON of the rows, in `fact_id` order, and the
/// places they name, in `place_id` order.
fn digest(facts: &[&ClaimantFact], places: &[&Place]) -> Hash {
    let mut facts = facts.to_vec();
    facts.sort_by_key(|f| f.fact_id);
    let mut places = places.to_vec();
    places.sort_by(|a, b| a.place_id.cmp(&b.place_id));
    places.dedup_by(|a, b| a.place_id == b.place_id);
    let bytes = serde_json::to_vec(&(facts, places)).expect("register rows serialize");
    Hash::from_bytes(Sha256::digest(&bytes).into())
}

/// What a place of work is for the claimant on a date, as the register
/// classifies it; the travel computation decides the allowances from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Workplace {
    /// Erityinen työntekemispaikka (TVL 72 § 1 mom): all scheduled allowances
    /// can be due. A destination the register does not name is one too, since
    /// every regular and secondary workplace must be recorded.
    SpecialWorkPlace,
    /// Toissijainen työpaikka (TVL 71 § 1 mom): travel and lodging costs only.
    SecondaryWorkplace,
    /// Varsinainen työpaikka (TVL 72 b §): no tax-free travel allowance.
    RegularWorkplace,
}

/// `claimant snapshot` input: a trip's person, start date and places.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotRequest {
    pub employee_id: String,
    /// The trip's start date: the nature of a place is judged by the
    /// circumstances when the trip is made (ohje 4.1).
    pub date: Date,
    /// The registered place the trip starts from, if it starts from one.
    #[serde(default)]
    pub start_place_id: Option<String>,
    /// The registered places the trip goes to. Destinations the register does
    /// not name are special work sites and are not listed.
    #[serde(default)]
    pub destination_place_ids: Vec<String>,
    /// Build the snapshot as known at this instant instead of now.
    #[serde(default)]
    pub as_known_at: Option<Instant>,
}

/// A place of a trip with its role and classification on the trip's date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TripPlace {
    pub place_id: String,
    pub kind: PlaceKind,
    /// The role in force for the place, if the register gives it one.
    pub role: Option<WorkplaceRole>,
    pub classification: Workplace,
    pub basis: Option<BasisCode>,
}

/// What the register says about one trip, ready to be frozen into it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TripSnapshot {
    pub register: RegisterView,
    pub start: Option<TripPlace>,
    pub destinations: Vec<TripPlace>,
}

impl TripSnapshot {
    /// The trip's workplace: a regular workplace if any destination is one (the
    /// computation refuses it), a secondary workplace if one is, and otherwise a
    /// special work site. `None` when a secondary workplace and a registered
    /// special work site are both destinations: their allowances differ, so the
    /// trip has to be split.
    pub fn workplace(&self) -> Option<Workplace> {
        let has = |w: Workplace| self.destinations.iter().any(|d| d.classification == w);
        if has(Workplace::RegularWorkplace) {
            Some(Workplace::RegularWorkplace)
        } else if has(Workplace::SecondaryWorkplace) {
            (!has(Workplace::SpecialWorkPlace)).then_some(Workplace::SecondaryWorkplace)
        } else {
            Some(Workplace::SpecialWorkPlace)
        }
    }
}

fn trip_place(
    view: &RegisterView,
    places: &[Place],
    place_id: &str,
) -> Result<TripPlace, ClaimantRefusal> {
    let place = places
        .iter()
        .find(|p| p.place_id == place_id && p.recorded_at <= view.as_known_at)
        .ok_or_else(|| {
            refuse(
                RefusalCode::PlaceNotFound,
                format!(
                    "place {place_id:?} is not recorded for {}",
                    view.employee_id
                ),
            )
        })?;
    replaced_check(places, place_id, Some(view.as_known_at))?;
    let entry = view
        .workplaces
        .iter()
        .find(|w| w.place.as_ref().is_some_and(|p| p.place_id == place_id));
    let role = entry.map(|w| w.role);
    let classification = match role {
        Some(WorkplaceRole::RegularPrimary | WorkplaceRole::RegularOther) => {
            Workplace::RegularWorkplace
        }
        Some(WorkplaceRole::Secondary) => Workplace::SecondaryWorkplace,
        Some(WorkplaceRole::TrackedSite | WorkplaceRole::None) | None => {
            Workplace::SpecialWorkPlace
        }
    };
    Ok(TripPlace {
        place_id: place_id.to_string(),
        kind: place.kind,
        role,
        classification,
        basis: entry.map(|w| w.basis),
    })
}

/// The register snapshot for a trip. Refuses when the person has no
/// relationship to the company or no regular-workplace entry on the date: the
/// program computes no tax-free travel allowance and no travel costs without
/// one (ADR-11 section 3).
pub fn trip_snapshot(
    request: &SnapshotRequest,
    places: &[Place],
    facts: &[ClaimantFact],
    now: Instant,
) -> Result<TripSnapshot, ClaimantRefusal> {
    let as_known_at = resolve_as_known_at(request.as_known_at, places, facts, now)?;
    let view = register_view(
        &request.employee_id,
        places,
        facts,
        request.date,
        as_known_at,
    );
    if view.relationships.is_empty() {
        return Err(refuse(
            RefusalCode::ClaimantRelationshipUnknown,
            format!(
                "no relationship to the company is recorded for {} on {}; tax-free travel \
                 allowances are paid only to a wage or fee recipient (EPL 13 §), so record \
                 the relationship first",
                request.employee_id, request.date
            ),
        ));
    }
    if !view.regular_workplace_known {
        return Err(refuse(
            RefusalCode::RegularWorkplaceUnknown,
            format!(
                "no regular workplace (varsinainen työpaikka) is recorded for {} on {}; \
                 ask the user where they work regularly and record the entry with its basis \
                 before computing tax-free travel allowances or travel costs",
                request.employee_id, request.date
            ),
        ));
    }
    let start = request
        .start_place_id
        .as_deref()
        .map(|id| trip_place(&view, places, id))
        .transpose()?;
    let destinations = request
        .destination_place_ids
        .iter()
        .map(|id| trip_place(&view, places, id))
        .collect::<Result<Vec<_>, _>>()?;
    // The home is where a trip starts and ends, never what it is made to: a
    // journey home is the return leg, and a home that is the regular workplace
    // is classified as one, which the travel computation refuses.
    if let Some(home) = destinations
        .iter()
        .find(|d| d.kind == PlaceKind::Home && d.classification != Workplace::RegularWorkplace)
    {
        return Err(refuse(
            RefusalCode::TripToHome,
            format!(
                "{:?} is the home of {}: a journey home is part of the trip, not a place it \
                 is made to; list the places the trip went to for work",
                home.place_id, request.employee_id
            ),
        ));
    }
    let has_primary = view
        .workplaces
        .iter()
        .any(|w| w.role == WorkplaceRole::RegularPrimary);
    if !has_primary
        && destinations
            .iter()
            .any(|d| d.classification == Workplace::SecondaryWorkplace)
    {
        return Err(refuse(
            RefusalCode::SecondaryWorkplaceInvalid,
            format!(
                "a secondary workplace needs a primary regular workplace in force, and none is \
                 recorded for {} on {} (VH/630/00.01.00/2026 4.1)",
                request.employee_id, request.date
            ),
        ));
    }
    Ok(TripSnapshot {
        register: view,
        start,
        destinations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        Date::parse(s).unwrap()
    }

    fn t(secs: i64) -> Instant {
        Instant::from_unix(1_800_000_000 + secs, 0).unwrap()
    }

    fn place(id: &str, kind: PlaceKind, municipality: &str) -> Place {
        Place {
            employee_id: "maija".into(),
            place_id: id.into(),
            kind,
            label: format!("{id} label"),
            location: format!("Esimerkkikatu 1, {municipality}"),
            municipality: municipality.into(),
            replaces: None,
            recorded_at: t(0),
            confirmed_by: "Maija Meikäläinen".into(),
        }
    }

    fn workplace(place: Option<&str>, role: WorkplaceRole, basis: BasisCode) -> FactBody {
        FactBody::Workplace {
            place_id: place.map(str::to_string),
            role,
            basis,
            basis_explanation: None,
        }
    }

    fn entry(body: FactBody, from: &str, to: Option<&str>) -> FactEntry {
        FactEntry {
            valid_from: d(from),
            valid_to: to.map(d),
            body,
        }
    }

    /// A tiny in-memory register that validates and appends like the store.
    struct Reg {
        places: Vec<Place>,
        facts: Vec<ClaimantFact>,
    }

    impl Reg {
        fn new() -> Self {
            Reg {
                places: vec![
                    place("home", PlaceKind::Home, "Tampere"),
                    place("office", PlaceKind::CompanyPremises, "Tampere"),
                    place("branch", PlaceKind::CompanyPremises, "Oulu"),
                    place("customer", PlaceKind::CustomerSite, "Turku"),
                ],
                facts: Vec::new(),
            }
        }

        fn record(
            &mut self,
            change: FactChange,
            supersedes: Option<i64>,
            at: Instant,
        ) -> Result<i64, ClaimantRefusal> {
            let new = NewFact {
                employee_id: "maija".into(),
                change,
                confirmed_by: "Maija Meikäläinen".into(),
                evidence_hash: None,
                supersedes: supersedes.map(|fact_id| Supersede {
                    fact_id,
                    reason: "corrected".into(),
                }),
            };
            validate_fact(&new, &self.places, &self.facts, at)?;
            let fact_id = self.facts.len() as i64 + 1;
            self.facts.push(ClaimantFact {
                fact_id,
                employee_id: new.employee_id,
                change: new.change,
                recorded_at: at,
                confirmed_by: new.confirmed_by,
                evidence_hash: None,
                supersedes: new.supersedes.as_ref().map(|s| s.fact_id),
                reason: new.supersedes.map(|s| s.reason),
            });
            Ok(fact_id)
        }

        fn add(&mut self, e: FactEntry, at: Instant) -> Result<i64, ClaimantRefusal> {
            self.record(FactChange::Assert(e), None, at)
        }

        fn employee(&mut self) {
            self.add(
                entry(
                    FactBody::Relationship {
                        relationship: Relationship::Employee,
                    },
                    "2024-01-01",
                    None,
                ),
                t(1),
            )
            .unwrap();
        }

        fn snapshot(
            &self,
            date: &str,
            start: Option<&str>,
            dests: &[&str],
            at: Option<Instant>,
        ) -> Result<TripSnapshot, ClaimantRefusal> {
            trip_snapshot(
                &SnapshotRequest {
                    employee_id: "maija".into(),
                    date: d(date),
                    start_place_id: start.map(str::to_string),
                    destination_place_ids: dests.iter().map(|s| s.to_string()).collect(),
                    as_known_at: at,
                },
                &self.places,
                &self.facts,
                t(1000),
            )
        }
    }

    #[test]
    fn a_backward_correction_changes_now_but_not_as_known_at_the_time() {
        let mut reg = Reg::new();
        reg.employee();
        // At t(10): the office has been the regular workplace since 2024.
        let office = reg
            .add(
                entry(
                    workplace(
                        Some("office"),
                        WorkplaceRole::RegularPrimary,
                        BasisCode::WorksThereRegularly,
                    ),
                    "2024-01-01",
                    None,
                ),
                t(10),
            )
            .unwrap();
        // At t(20) the user corrects backwards: from 2025-01-01 the company's
        // premises were in the home, so the office entry ends 2024-12-31...
        reg.record(
            FactChange::Assert(entry(
                workplace(
                    Some("office"),
                    WorkplaceRole::RegularPrimary,
                    BasisCode::WorksThereRegularly,
                ),
                "2024-01-01",
                Some("2024-12-31"),
            )),
            Some(office),
            t(20),
        )
        .unwrap();
        // ...and the home is the regular workplace from then on.
        reg.add(
            entry(
                workplace(
                    Some("home"),
                    WorkplaceRole::RegularPrimary,
                    BasisCode::CompanyPremisesInHome,
                ),
                "2025-01-01",
                None,
            ),
            t(21),
        )
        .unwrap();

        let now = register_view("maija", &reg.places, &reg.facts, d("2025-06-01"), t(100));
        let then = register_view("maija", &reg.places, &reg.facts, d("2025-06-01"), t(15));
        let now_place = now.workplaces[0].place.as_ref().unwrap();
        let then_place = then.workplaces[0].place.as_ref().unwrap();
        assert_eq!(now.workplaces.len(), 1);
        assert_eq!(now_place.place_id, "home");
        assert_eq!(then.workplaces.len(), 1);
        assert_eq!(then_place.place_id, "office");
        assert_ne!(now.facts_digest, then.facts_digest);

        // A trip to the office in 2025: as known now it is a special work site,
        // as known at t(15) it was the regular workplace and is refused.
        let snap_now = reg
            .snapshot("2025-06-01", Some("home"), &["office"], None)
            .unwrap();
        assert_eq!(
            snap_now.destinations[0].classification,
            Workplace::SpecialWorkPlace
        );
        assert_eq!(snap_now.workplace(), Some(Workplace::SpecialWorkPlace));
        let snap_then = reg
            .snapshot("2025-06-01", Some("home"), &["office"], Some(t(15)))
            .unwrap();
        assert_eq!(
            snap_then.destinations[0].classification,
            Workplace::RegularWorkplace
        );
        assert_eq!(snap_then.workplace(), Some(Workplace::RegularWorkplace));
    }

    #[test]
    fn no_entry_on_the_date_is_regular_workplace_unknown() {
        let mut reg = Reg::new();
        reg.employee();
        reg.add(
            entry(
                workplace(
                    Some("office"),
                    WorkplaceRole::RegularPrimary,
                    BasisCode::WorksThereRegularly,
                ),
                "2025-01-01",
                Some("2025-12-31"),
            ),
            t(10),
        )
        .unwrap();
        for date in ["2024-12-31", "2026-01-01"] {
            let err = reg.snapshot(date, None, &["customer"], None).unwrap_err();
            assert_eq!(err.error_code(), "REGULAR_WORKPLACE_UNKNOWN", "{date}");
        }
        assert!(reg
            .snapshot("2025-12-31", None, &["customer"], None)
            .is_ok());
        // An entry recorded later is not known at an earlier instant.
        let err = reg
            .snapshot("2025-06-01", None, &["customer"], Some(t(5)))
            .unwrap_err();
        assert_eq!(err.error_code(), "REGULAR_WORKPLACE_UNKNOWN");
    }

    #[test]
    fn an_explicit_none_settles_the_question() {
        let mut reg = Reg::new();
        reg.add(
            entry(
                FactBody::Relationship {
                    relationship: Relationship::BoardMember,
                },
                "2024-01-01",
                None,
            ),
            t(1),
        )
        .unwrap();
        reg.add(
            entry(
                workplace(None, WorkplaceRole::None, BasisCode::BoardMemberOnly),
                "2024-01-01",
                None,
            ),
            t(2),
        )
        .unwrap();
        let snap = reg
            .snapshot("2025-03-01", Some("home"), &["office"], None)
            .unwrap();
        assert!(snap.register.regular_workplace_known);
        assert_eq!(
            snap.destinations[0].classification,
            Workplace::SpecialWorkPlace
        );
        assert_eq!(snap.workplace(), Some(Workplace::SpecialWorkPlace));
    }

    #[test]
    fn without_a_relationship_the_snapshot_refuses_first() {
        let mut reg = Reg::new();
        let err = reg.snapshot("2025-03-01", None, &[], None).unwrap_err();
        assert_eq!(err.error_code(), "CLAIMANT_RELATIONSHIP_UNKNOWN");
        reg.employee();
        let err = reg.snapshot("2025-03-01", None, &[], None).unwrap_err();
        assert_eq!(err.error_code(), "REGULAR_WORKPLACE_UNKNOWN");
    }

    #[test]
    fn a_contractor_cannot_be_a_claimant() {
        let mut reg = Reg::new();
        let err = reg
            .add(
                entry(
                    FactBody::Relationship {
                        relationship: Relationship::Contractor,
                    },
                    "2025-01-01",
                    None,
                ),
                t(1),
            )
            .unwrap_err();
        assert_eq!(err.error_code(), "CONTRACTOR_NOT_CLAIMANT");
        assert!(reg.facts.is_empty());
    }

    #[test]
    fn a_secondary_workplace_classifies_the_trip_as_secondary() {
        let mut reg = Reg::new();
        reg.employee();
        reg.add(
            entry(
                workplace(
                    Some("office"),
                    WorkplaceRole::RegularPrimary,
                    BasisCode::WorksThereRegularly,
                ),
                "2024-01-01",
                None,
            ),
            t(10),
        )
        .unwrap();
        reg.add(
            entry(
                workplace(
                    Some("branch"),
                    WorkplaceRole::Secondary,
                    BasisCode::PremisesInAnotherMunicipality,
                ),
                "2024-01-01",
                None,
            ),
            t(11),
        )
        .unwrap();
        let snap = reg
            .snapshot("2025-03-01", Some("home"), &["branch"], None)
            .unwrap();
        assert_eq!(
            snap.destinations[0].classification,
            Workplace::SecondaryWorkplace
        );
        assert_eq!(snap.workplace(), Some(Workplace::SecondaryWorkplace));
        // A trip to the regular workplace is refused outright.
        let snap = reg
            .snapshot("2025-03-01", Some("home"), &["office"], None)
            .unwrap();
        assert_eq!(snap.workplace(), Some(Workplace::RegularWorkplace));
        // A customer site is a special work site.
        let snap = reg
            .snapshot("2025-03-01", Some("office"), &["customer"], None)
            .unwrap();
        assert_eq!(
            snap.start.as_ref().unwrap().role,
            Some(WorkplaceRole::RegularPrimary)
        );
        assert_eq!(snap.workplace(), Some(Workplace::SpecialWorkPlace));
    }

    #[test]
    fn a_trip_mixing_a_secondary_workplace_and_a_special_site_has_no_workplace() {
        let mut reg = Reg::new();
        reg.employee();
        reg.add(regular("office", BasisCode::WorksThereRegularly), t(10))
            .unwrap();
        reg.add(
            entry(
                workplace(
                    Some("branch"),
                    WorkplaceRole::Secondary,
                    BasisCode::PremisesInAnotherMunicipality,
                ),
                "2024-01-01",
                None,
            ),
            t(11),
        )
        .unwrap();
        reg.add(
            entry(
                workplace(
                    Some("customer"),
                    WorkplaceRole::TrackedSite,
                    BasisCode::OccasionalVisits,
                ),
                "2024-01-01",
                None,
            ),
            t(12),
        )
        .unwrap();
        let snap = reg
            .snapshot("2025-03-04", Some("home"), &["branch", "customer"], None)
            .unwrap();
        assert_eq!(snap.workplace(), None);
        // With the regular workplace among the destinations, it decides.
        let snap = reg
            .snapshot("2025-03-04", Some("home"), &["branch", "office"], None)
            .unwrap();
        assert_eq!(snap.workplace(), Some(Workplace::RegularWorkplace));
    }

    #[test]
    fn a_secondary_workplace_must_be_employer_premises_in_another_municipality() {
        let mut reg = Reg::new();
        reg.add(
            entry(
                workplace(
                    Some("office"),
                    WorkplaceRole::RegularPrimary,
                    BasisCode::WorksThereRegularly,
                ),
                "2024-01-01",
                None,
            ),
            t(10),
        )
        .unwrap();
        // The home cannot be secondary (KHO 2017:178).
        let err = reg
            .add(
                entry(
                    FactBody::Workplace {
                        place_id: Some("home".into()),
                        role: WorkplaceRole::Secondary,
                        basis: BasisCode::Other,
                        basis_explanation: Some("works from home on Fridays".into()),
                    },
                    "2024-01-01",
                    None,
                ),
                t(11),
            )
            .unwrap_err();
        assert_eq!(err.error_code(), "SECONDARY_WORKPLACE_INVALID");
        // Same municipality as the primary: not secondary.
        reg.places
            .push(place("office-2", PlaceKind::CompanyPremises, " tampere "));
        let err = reg
            .add(
                entry(
                    workplace(
                        Some("office-2"),
                        WorkplaceRole::Secondary,
                        BasisCode::PremisesInAnotherMunicipality,
                    ),
                    "2024-06-01",
                    None,
                ),
                t(12),
            )
            .unwrap_err();
        assert_eq!(err.error_code(), "SECONDARY_WORKPLACE_INVALID");
        // As another regular workplace it is fine.
        reg.add(
            entry(
                workplace(
                    Some("office-2"),
                    WorkplaceRole::RegularOther,
                    BasisCode::WorksThereRegularly,
                ),
                "2024-06-01",
                None,
            ),
            t(13),
        )
        .unwrap();
    }

    #[test]
    fn conflicting_entries_on_overlapping_dates_are_refused() {
        let mut reg = Reg::new();
        let office = reg
            .add(
                entry(
                    workplace(
                        Some("office"),
                        WorkplaceRole::RegularPrimary,
                        BasisCode::WorksThereRegularly,
                    ),
                    "2024-01-01",
                    Some("2024-12-31"),
                ),
                t(10),
            )
            .unwrap();
        // Two primaries overlapping on one day.
        let err = reg
            .add(
                entry(
                    workplace(
                        Some("home"),
                        WorkplaceRole::RegularPrimary,
                        BasisCode::CompanyPremisesInHome,
                    ),
                    "2024-12-31",
                    None,
                ),
                t(11),
            )
            .unwrap_err();
        assert_eq!(err.error_code(), "CLAIMANT_FACT_CONFLICT");
        // Adjacent is fine.
        reg.add(
            entry(
                workplace(
                    Some("home"),
                    WorkplaceRole::RegularPrimary,
                    BasisCode::CompanyPremisesInHome,
                ),
                "2025-01-01",
                None,
            ),
            t(12),
        )
        .unwrap();
        // "None" contradicts a regular workplace on the same dates.
        let err = reg
            .add(
                entry(
                    workplace(None, WorkplaceRole::None, BasisCode::MobileWorkNoBase),
                    "2024-06-01",
                    Some("2024-06-30"),
                ),
                t(13),
            )
            .unwrap_err();
        assert_eq!(err.error_code(), "CLAIMANT_FACT_CONFLICT");
        // The same place cannot hold two roles at once.
        let err = reg
            .add(
                entry(
                    workplace(
                        Some("office"),
                        WorkplaceRole::TrackedSite,
                        BasisCode::OccasionalVisits,
                    ),
                    "2024-06-01",
                    None,
                ),
                t(14),
            )
            .unwrap_err();
        assert_eq!(err.error_code(), "CLAIMANT_FACT_CONFLICT");
        // A correction is checked without the row it replaces.
        reg.record(
            FactChange::Assert(entry(
                workplace(
                    Some("office"),
                    WorkplaceRole::RegularPrimary,
                    BasisCode::EmployerPremisesWeekly,
                ),
                "2024-01-01",
                Some("2024-12-31"),
            )),
            Some(office),
            t(15),
        )
        .unwrap();
        // Part-owner facts never overlap; relationships may differ but not repeat.
        let po = |v| FactBody::PartOwner { part_owner: v };
        reg.add(entry(po(true), "2024-01-01", None), t(16)).unwrap();
        let err = reg
            .add(entry(po(false), "2025-01-01", None), t(17))
            .unwrap_err();
        assert_eq!(err.error_code(), "CLAIMANT_FACT_CONFLICT");
        let rel = |r| FactBody::Relationship { relationship: r };
        reg.add(
            entry(rel(Relationship::ManagingDirector), "2024-01-01", None),
            t(18),
        )
        .unwrap();
        reg.add(
            entry(rel(Relationship::BoardMember), "2024-01-01", None),
            t(19),
        )
        .unwrap();
        let err = reg
            .add(
                entry(rel(Relationship::BoardMember), "2025-01-01", None),
                t(20),
            )
            .unwrap_err();
        assert_eq!(err.error_code(), "CLAIMANT_FACT_CONFLICT");
    }

    #[test]
    fn basis_codes_fit_their_roles_and_places() {
        let mut reg = Reg::new();
        let cases = [
            (
                workplace(
                    Some("office"),
                    WorkplaceRole::RegularPrimary,
                    BasisCode::OccasionalVisits,
                ),
                "WORKPLACE_BASIS_MISMATCH",
            ),
            (
                workplace(
                    Some("office"),
                    WorkplaceRole::RegularPrimary,
                    BasisCode::CompanyPremisesInHome,
                ),
                "WORKPLACE_BASIS_MISMATCH",
            ),
            (
                workplace(
                    Some("home"),
                    WorkplaceRole::RegularPrimary,
                    BasisCode::RemoteWorkByChoice,
                ),
                "WORKPLACE_BASIS_MISMATCH",
            ),
            (
                workplace(
                    Some("office"),
                    WorkplaceRole::RegularPrimary,
                    BasisCode::Other,
                ),
                "WORKPLACE_BASIS_MISMATCH",
            ),
            (
                workplace(
                    Some("nowhere"),
                    WorkplaceRole::TrackedSite,
                    BasisCode::OccasionalVisits,
                ),
                "PLACE_NOT_FOUND",
            ),
            (
                workplace(
                    None,
                    WorkplaceRole::RegularPrimary,
                    BasisCode::WorksThereRegularly,
                ),
                "CLAIMANT_INPUT_INVALID",
            ),
            (
                workplace(
                    Some("home"),
                    WorkplaceRole::None,
                    BasisCode::BoardMemberOnly,
                ),
                "CLAIMANT_INPUT_INVALID",
            ),
        ];
        for (body, code) in cases {
            let err = reg
                .add(entry(body.clone(), "2025-01-01", None), t(10))
                .unwrap_err();
            assert_eq!(err.error_code(), code, "{body:?}");
        }
        // Every code names a guidance passage and at least one role.
        for code in BasisCode::ALL {
            assert!(!code.guidance().is_empty());
            assert!(!code.allowed_roles().is_empty());
        }
    }

    #[test]
    fn corrections_follow_the_chain() {
        let mut reg = Reg::new();
        let first = reg
            .add(
                entry(
                    workplace(
                        Some("office"),
                        WorkplaceRole::RegularPrimary,
                        BasisCode::WorksThereRegularly,
                    ),
                    "2024-01-01",
                    None,
                ),
                t(10),
            )
            .unwrap();
        let retraction = reg.record(FactChange::Retract, Some(first), t(11)).unwrap();
        // A row is corrected once; a retraction is not corrected.
        let err = reg
            .record(FactChange::Retract, Some(first), t(12))
            .unwrap_err();
        assert_eq!(err.error_code(), "CLAIMANT_FACT_SUPERSEDED");
        let err = reg
            .record(FactChange::Retract, Some(retraction), t(12))
            .unwrap_err();
        assert_eq!(err.error_code(), "CLAIMANT_INPUT_INVALID");
        let err = reg
            .record(FactChange::Retract, Some(99), t(12))
            .unwrap_err();
        assert_eq!(err.error_code(), "CLAIMANT_FACT_NOT_FOUND");
        // Retracted now, in force as known before the retraction.
        let now = register_view("maija", &reg.places, &reg.facts, d("2025-01-01"), t(100));
        assert!(!now.regular_workplace_known);
        let before = register_view("maija", &reg.places, &reg.facts, d("2025-01-01"), t(10));
        assert!(before.regular_workplace_known);
        // A clock behind the last row refuses.
        let err = reg
            .add(
                entry(
                    workplace(
                        Some("home"),
                        WorkplaceRole::RegularPrimary,
                        BasisCode::CompanyPremisesInHome,
                    ),
                    "2024-01-01",
                    None,
                ),
                t(5),
            )
            .unwrap_err();
        assert_eq!(err.error_code(), "CLAIMANT_CLOCK_BEHIND");
    }

    fn regular(place: &str, basis: BasisCode) -> FactEntry {
        entry(
            workplace(Some(place), WorkplaceRole::RegularPrimary, basis),
            "2024-01-01",
            None,
        )
    }

    #[test]
    fn the_home_is_never_a_trip_destination() {
        let mut reg = Reg::new();
        reg.employee();
        reg.add(regular("office", BasisCode::WorksThereRegularly), t(10))
            .unwrap();
        // Office to home is the commute, not a trip to a special work site.
        let err = reg
            .snapshot("2025-03-04", Some("office"), &["home"], None)
            .unwrap_err();
        assert_eq!(err.error_code(), "TRIP_TO_HOME");

        // When the home is the regular workplace, it is refused as such.
        let mut reg = Reg::new();
        reg.employee();
        reg.add(regular("home", BasisCode::CompanyPremisesInHome), t(10))
            .unwrap();
        let snap = reg
            .snapshot("2025-03-04", Some("customer"), &["home"], None)
            .unwrap();
        assert_eq!(snap.workplace(), Some(Workplace::RegularWorkplace));
    }

    #[test]
    fn a_replaced_place_can_no_longer_be_named() {
        let mut reg = Reg::new();
        reg.employee();
        let office = reg
            .add(regular("office", BasisCode::WorksThereRegularly), t(10))
            .unwrap();
        // The office's address was mistyped: record it again as office-2.
        let new = NewPlace {
            employee_id: "maija".into(),
            place_id: "office-2".into(),
            kind: PlaceKind::CompanyPremises,
            label: "office".into(),
            location: "Toimistotie 2, Tampere".into(),
            municipality: "Tampere".into(),
            replaces: Some("office".into()),
            confirmed_by: "Maija Meikäläinen".into(),
        };
        validate_place(&new, &reg.places, &reg.facts, t(20)).unwrap();
        reg.places.push(Place {
            employee_id: new.employee_id.clone(),
            place_id: new.place_id.clone(),
            kind: new.kind,
            label: new.label.clone(),
            location: new.location.clone(),
            municipality: new.municipality.clone(),
            replaces: new.replaces.clone(),
            recorded_at: t(20),
            confirmed_by: new.confirmed_by.clone(),
        });
        // A trip naming the old id is refused instead of passing for a special
        // work site; as known before the replacement it was the regular workplace.
        let err = reg
            .snapshot("2025-03-04", Some("home"), &["office"], None)
            .unwrap_err();
        assert_eq!(err.error_code(), "PLACE_REPLACED");
        let before = reg
            .snapshot("2025-03-04", Some("home"), &["office"], Some(t(15)))
            .unwrap();
        assert_eq!(
            before.destinations[0].classification,
            Workplace::RegularWorkplace
        );
        // A new entry cannot name the old place; the correction names the new one.
        let err = reg
            .add(
                entry(
                    workplace(
                        Some("office"),
                        WorkplaceRole::TrackedSite,
                        BasisCode::OccasionalVisits,
                    ),
                    "2026-01-01",
                    None,
                ),
                t(21),
            )
            .unwrap_err();
        assert_eq!(err.error_code(), "PLACE_REPLACED");
        reg.record(
            FactChange::Assert(regular("office-2", BasisCode::WorksThereRegularly)),
            Some(office),
            t(22),
        )
        .unwrap();
        let snap = reg
            .snapshot("2025-03-04", Some("home"), &["office-2"], None)
            .unwrap();
        assert_eq!(snap.workplace(), Some(Workplace::RegularWorkplace));
        // A place is replaced once, and only a recorded place can be replaced.
        let again = NewPlace {
            place_id: "office-3".into(),
            ..new.clone()
        };
        let err = validate_place(&again, &reg.places, &reg.facts, t(30)).unwrap_err();
        assert_eq!(err.error_code(), "PLACE_REPLACED");
        let ghost = NewPlace {
            place_id: "office-4".into(),
            replaces: Some("nowhere".into()),
            ..new
        };
        let err = validate_place(&ghost, &reg.places, &reg.facts, t(30)).unwrap_err();
        assert_eq!(err.error_code(), "PLACE_NOT_FOUND");
    }

    #[test]
    fn a_query_as_known_now_refuses_a_clock_behind_the_register() {
        let mut reg = Reg::new();
        reg.employee();
        reg.add(regular("office", BasisCode::WorksThereRegularly), t(10))
            .unwrap();
        let request = SnapshotRequest {
            employee_id: "maija".into(),
            date: d("2025-03-04"),
            start_place_id: None,
            destination_place_ids: vec![],
            as_known_at: None,
        };
        // Now is t(5), before the entry recorded at t(10): the entry would
        // silently drop out of "as known now", so the query refuses.
        let err = trip_snapshot(&request, &reg.places, &reg.facts, t(5)).unwrap_err();
        assert_eq!(err.error_code(), "CLAIMANT_CLOCK_BEHIND");
        assert!(trip_snapshot(&request, &reg.places, &reg.facts, t(10)).is_ok());
        // An explicit instant is a historical question and is answered.
        let at = resolve_as_known_at(Some(t(5)), &reg.places, &reg.facts, t(5)).unwrap();
        assert_eq!(at, t(5));
    }

    #[test]
    fn correction_input_needs_exactly_one_of_replacement_and_retract() {
        let base = CorrectionInput {
            employee_id: "maija".into(),
            fact_id: 1,
            reason: "wrong date".into(),
            confirmed_by: "Maija Meikäläinen".into(),
            evidence_hash: None,
            retract: false,
            replacement: None,
        };
        assert!(base.clone().into_new_fact().is_err());
        let both = CorrectionInput {
            retract: true,
            replacement: Some(entry(
                FactBody::PartOwner { part_owner: true },
                "2025-01-01",
                None,
            )),
            ..base.clone()
        };
        assert!(both.into_new_fact().is_err());
        let retract = CorrectionInput {
            retract: true,
            ..base
        };
        let new = retract.into_new_fact().unwrap();
        assert_eq!(new.change, FactChange::Retract);
    }

    #[test]
    fn fact_input_is_parsed_strictly() {
        let input: FactInput = serde_json::from_str(
            r#"{"employee_id":"maija","valid_from":"2024-01-01",
                "fact":{"kind":"workplace","place_id":"home","role":"regular_primary",
                        "basis":"company_premises_in_home"},
                "confirmed_by":"Maija Meikäläinen"}"#,
        )
        .unwrap();
        assert_eq!(input.valid_to, None);
        assert_eq!(input.fact.kind(), FactKind::Workplace);
        for bad in [
            // An unknown basis code.
            r#"{"employee_id":"m","valid_from":"2024-01-01","confirmed_by":"x",
                "fact":{"kind":"workplace","role":"regular_primary","basis":"made_up"}}"#,
            // An unknown field inside the fact.
            r#"{"employee_id":"m","valid_from":"2024-01-01","confirmed_by":"x",
                "fact":{"kind":"part_owner","part_owner":true,"note":"x"}}"#,
            // An unknown top-level field.
            r#"{"employee_id":"m","valid_from":"2024-01-01","confirmed_by":"x","extra":1,
                "fact":{"kind":"part_owner","part_owner":true}}"#,
            // A non-canonical date.
            r#"{"employee_id":"m","valid_from":"2024-1-01","confirmed_by":"x",
                "fact":{"kind":"part_owner","part_owner":true}}"#,
        ] {
            assert!(serde_json::from_str::<FactInput>(bad).is_err(), "{bad}");
        }
    }
}
