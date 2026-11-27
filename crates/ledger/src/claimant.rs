//! The claimant register (ADR-11 sections 1 and 3): the people who may file an
//! expense claim, their dated relationship and part-owner facts, and their
//! regular-workplace entries.
//!
//! The rows live in the personal-data store (`masterdata/pii.sqlite`), next to
//! the employee record they belong to, because they are personal data (a home
//! as a workplace, routes) and must stay erasable: they never go into git or
//! the fiscal store, and erasing the person's employee row cascades them away.
//! The rules are the Domain's (`aggountant_domain::workplace`); this module
//! runs them inside the store's writer lock and stamps `recorded_at`.

use serde::Serialize;

use aggountant_domain::workplace::{
    register_view, trip_snapshot, validate_fact, validate_place, ClaimantFact, NewFact, NewPlace,
    Place, RegisterView, SnapshotRequest, TripSnapshot,
};
use aggountant_model::{Date, Instant};

use crate::error::LedgerError;
use crate::store::ClaimantStore;
use crate::Ledger;

/// Every row the register holds for one person, in recording order.
#[derive(Debug, Clone, Serialize)]
pub struct ClaimantHistory {
    pub employee_id: String,
    pub places: Vec<Place>,
    pub facts: Vec<ClaimantFact>,
}

impl Ledger {
    /// Record a place for a person. Places are never edited.
    pub fn add_claimant_place(&self, new: &NewPlace, now: Instant) -> Result<Place, LedgerError> {
        let place = Place {
            employee_id: new.employee_id.clone(),
            place_id: new.place_id.clone(),
            kind: new.kind,
            label: new.label.clone(),
            location: new.location.clone(),
            municipality: new.municipality.clone(),
            recorded_at: now,
            confirmed_by: new.confirmed_by.clone(),
        };
        self.pii.append_claimant_place(&place, &|rows| {
            Ok(validate_place(new, &rows.places, &rows.facts, now)?)
        })?;
        Ok(place)
    }

    /// Record a fact or a correction, validated against the person's register
    /// as it stands. A correction supersedes its target; nothing is edited.
    pub fn record_claimant_fact(
        &self,
        new: &NewFact,
        now: Instant,
    ) -> Result<ClaimantFact, LedgerError> {
        self.pii.append_claimant_fact(new, now, &|rows| {
            Ok(validate_fact(new, &rows.places, &rows.facts, now)?)
        })
    }

    /// Every row the register holds for a person, superseded ones included.
    pub fn claimant_history(&self, employee_id: &str) -> Result<ClaimantHistory, LedgerError> {
        let rows = self.pii.claimant_rows(employee_id)?;
        Ok(ClaimantHistory {
            employee_id: employee_id.to_string(),
            places: rows.places,
            facts: rows.facts,
        })
    }

    /// What was in force for a person on `date`, as known at `as_known_at`
    /// (now when `None`).
    pub fn claimant_view(
        &self,
        employee_id: &str,
        date: Date,
        as_known_at: Option<Instant>,
        now: Instant,
    ) -> Result<RegisterView, LedgerError> {
        let rows = self.pii.claimant_rows(employee_id)?;
        Ok(register_view(
            employee_id,
            &rows.places,
            &rows.facts,
            date,
            as_known_at.unwrap_or(now),
        ))
    }

    /// The register snapshot for one trip, to be frozen into it.
    pub fn claimant_trip_snapshot(
        &self,
        request: &SnapshotRequest,
        now: Instant,
    ) -> Result<TripSnapshot, LedgerError> {
        let rows = self.pii.claimant_rows(&request.employee_id)?;
        Ok(trip_snapshot(request, &rows.places, &rows.facts, now)?)
    }
}

#[cfg(test)]
mod tests {
    use aggountant_domain::workplace::{
        BasisCode, CorrectionInput, FactBody, FactChange, FactEntry, FactInput, PlaceKind,
        Relationship, WorkplaceRole,
    };
    use aggountant_domain::TaxCard;

    use super::*;
    use crate::EmployeeState;

    fn t(secs: i64) -> Instant {
        Instant::from_unix(1_800_000_000 + secs, 0).unwrap()
    }

    fn d(s: &str) -> Date {
        Date::parse(s).unwrap()
    }

    fn ledger() -> (tempfile::TempDir, Ledger) {
        let dir = tempfile::tempdir().unwrap();
        let ledger = Ledger::init(dir.path()).unwrap();
        ledger
            .upsert_employee(&EmployeeState {
                employee_id: "maija".into(),
                name: "Maija Meikäläinen".into(),
                hetu: "131052-308T".into(),
                iban: "FI2112345600000785".into(),
                tax_card: TaxCard {
                    withholding_pct: "20.00".into(),
                },
            })
            .unwrap();
        (dir, ledger)
    }

    fn add_place(ledger: &Ledger, id: &str, kind: PlaceKind, at: Instant) {
        ledger
            .add_claimant_place(
                &NewPlace {
                    employee_id: "maija".into(),
                    place_id: id.into(),
                    kind,
                    label: id.into(),
                    location: "Esimerkkikatu 1, Tampere".into(),
                    municipality: "Tampere".into(),
                    confirmed_by: "Maija Meikäläinen".into(),
                },
                at,
            )
            .unwrap();
    }

    fn fact(from: &str, to: Option<&str>, body: FactBody) -> NewFact {
        FactInput {
            employee_id: "maija".into(),
            valid_from: d(from),
            valid_to: to.map(d),
            fact: body,
            confirmed_by: "Maija Meikäläinen".into(),
            evidence_hash: None,
        }
        .into_new_fact()
    }

    fn workplace(place: &str, basis: BasisCode) -> FactBody {
        FactBody::Workplace {
            place_id: Some(place.into()),
            role: WorkplaceRole::RegularPrimary,
            basis,
            basis_explanation: None,
        }
    }

    #[test]
    fn the_register_round_trips_and_answers_as_known_at() {
        let (dir, ledger) = ledger();
        add_place(&ledger, "office", PlaceKind::CompanyPremises, t(1));
        add_place(&ledger, "home", PlaceKind::Home, t(1));
        ledger
            .record_claimant_fact(
                &fact(
                    "2024-01-01",
                    None,
                    FactBody::Relationship {
                        relationship: Relationship::ManagingDirector,
                    },
                ),
                t(2),
            )
            .unwrap();
        let office = ledger
            .record_claimant_fact(
                &fact(
                    "2024-01-01",
                    None,
                    workplace("office", BasisCode::WorksThereRegularly),
                ),
                t(3),
            )
            .unwrap();
        // Backward correction recorded at t(10): the office ended 2024-12-31.
        let correction = CorrectionInput {
            employee_id: "maija".into(),
            fact_id: office.fact_id,
            reason: "the company moved into the home at the turn of the year".into(),
            confirmed_by: "Maija Meikäläinen".into(),
            evidence_hash: None,
            retract: false,
            replacement: Some(FactEntry {
                valid_from: d("2024-01-01"),
                valid_to: Some(d("2024-12-31")),
                body: workplace("office", BasisCode::WorksThereRegularly),
            }),
        }
        .into_new_fact()
        .unwrap();
        ledger.record_claimant_fact(&correction, t(10)).unwrap();
        ledger
            .record_claimant_fact(
                &fact(
                    "2025-01-01",
                    None,
                    workplace("home", BasisCode::CompanyPremisesInHome),
                ),
                t(11),
            )
            .unwrap();

        // Reopen the home: the rows come back from disk.
        drop(ledger);
        let ledger = Ledger::open(dir.path()).unwrap();
        let history = ledger.claimant_history("maija").unwrap();
        assert_eq!(history.places.len(), 2);
        assert_eq!(history.facts.len(), 4);
        assert_eq!(history.facts[2].supersedes, Some(office.fact_id));

        let date = d("2025-03-01");
        let now = ledger.claimant_view("maija", date, None, t(100)).unwrap();
        let then = ledger
            .claimant_view("maija", date, Some(t(5)), t(100))
            .unwrap();
        let place_of = |v: &RegisterView| v.workplaces[0].place.clone().unwrap().place_id;
        assert_eq!(place_of(&now), "home");
        assert_eq!(place_of(&then), "office");

        // The snapshot as known at t(5) classifies the office as regular.
        let snapshot = ledger
            .claimant_trip_snapshot(
                &SnapshotRequest {
                    employee_id: "maija".into(),
                    date,
                    start_place_id: Some("home".into()),
                    destination_place_ids: vec!["office".into()],
                    as_known_at: Some(t(5)),
                },
                t(100),
            )
            .unwrap();
        let err = aggountant_domain::workplace::trip_scope(&snapshot).unwrap_err();
        assert_eq!(err.error_code(), "TRIP_TO_REGULAR_WORKPLACE");
    }

    #[test]
    fn refusals_keep_their_codes_and_write_nothing() {
        let (_dir, ledger) = ledger();
        let err = ledger
            .record_claimant_fact(
                &fact(
                    "2025-01-01",
                    None,
                    FactBody::Relationship {
                        relationship: Relationship::Contractor,
                    },
                ),
                t(1),
            )
            .unwrap_err();
        assert_eq!(err.error_code(), "CONTRACTOR_NOT_CLAIMANT");
        let err = ledger
            .claimant_trip_snapshot(
                &SnapshotRequest {
                    employee_id: "maija".into(),
                    date: d("2025-01-01"),
                    start_place_id: None,
                    destination_place_ids: vec![],
                    as_known_at: None,
                },
                t(2),
            )
            .unwrap_err();
        assert_eq!(err.error_code(), "CLAIMANT_RELATIONSHIP_UNKNOWN");
        assert!(ledger.claimant_history("maija").unwrap().facts.is_empty());
        // An unknown person is not in the register.
        let err = ledger.claimant_history("ghost").unwrap_err();
        assert_eq!(err.error_code(), "EMPLOYEE_NOT_FOUND");
        let mut ghost = fact("2025-01-01", None, FactBody::PartOwner { part_owner: true });
        ghost.employee_id = "ghost".into();
        let err = ledger.record_claimant_fact(&ghost, t(3)).unwrap_err();
        assert_eq!(err.error_code(), "EMPLOYEE_NOT_FOUND");
    }

    #[test]
    fn a_retraction_is_stored_with_its_target_kind() {
        let (_dir, ledger) = ledger();
        let owner = ledger
            .record_claimant_fact(
                &fact("2024-01-01", None, FactBody::PartOwner { part_owner: true }),
                t(1),
            )
            .unwrap();
        let retraction = CorrectionInput {
            employee_id: "maija".into(),
            fact_id: owner.fact_id,
            reason: "recorded for the wrong person".into(),
            confirmed_by: "Maija Meikäläinen".into(),
            evidence_hash: None,
            retract: true,
            replacement: None,
        }
        .into_new_fact()
        .unwrap();
        let stored = ledger.record_claimant_fact(&retraction, t(2)).unwrap();
        assert_eq!(stored.change, FactChange::Retract);
        let kind: String = ledger
            .pii
            .conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT kind FROM claimant_fact WHERE fact_id = ?1",
                [stored.fact_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(kind, "part_owner");
        let view = ledger
            .claimant_view("maija", d("2025-01-01"), None, t(3))
            .unwrap();
        assert!(view.part_owner.is_none());
    }
}
