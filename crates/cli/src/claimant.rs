//! `aggountant claimant`: the claimant register (ADR-11 sections 1 and 3) in
//! the personal-data store. Thin: every rule is the Domain's, every write goes
//! through the Ledger.

use std::path::Path;

use clap::Subcommand;
use serde_json::{json, Value};

use aggountant_domain::workplace::{
    trip_scope, CorrectionInput, FactInput, NewPlace, SnapshotRequest,
};
use aggountant_ledger::Ledger;
use aggountant_model::Instant;

use crate::{parse_cli_date, read_stdin_json, CliError};

#[derive(Subcommand, Debug)]
pub(crate) enum ClaimantCmd {
    /// Record a place of a person (home, company premises, customer site …) from
    /// stdin JSON. Places are never edited.
    Place {
        #[command(subcommand)]
        cmd: PlaceCmd,
    },
    /// Record a dated fact confirmed by the user from stdin JSON: a workplace
    /// entry (regular workplace, secondary workplace, tracked site or "none"),
    /// a relationship to the company, or part-ownership.
    #[command(after_help = crate::input_help::CLAIMANT_ADD)]
    Add,
    /// Correct a recorded fact from stdin JSON: a new fact supersedes it, or a
    /// retraction removes it. Nothing is edited; the old row stays in history.
    #[command(after_help = crate::input_help::CLAIMANT_CORRECT)]
    Correct,
    /// Show what is in force for a person on a date, as known now or as known
    /// at an earlier instant.
    Show {
        #[arg(long)]
        employee: String,
        /// The date (YYYY-MM-DD) the facts must hold on.
        #[arg(long)]
        date: String,
        /// Show the register as it was known at this RFC 3339 instant.
        #[arg(long)]
        as_known_at: Option<String>,
    },
    /// List every place and fact recorded for a person, corrections included.
    History {
        #[arg(long)]
        employee: String,
    },
    /// Classify a trip's places from stdin JSON and show what the trip may be
    /// reimbursed for. Refuses without a relationship or a regular-workplace
    /// entry on the date, and for a trip to a regular workplace.
    #[command(after_help = crate::input_help::CLAIMANT_SNAPSHOT)]
    Snapshot,
}

#[derive(Subcommand, Debug)]
pub(crate) enum PlaceCmd {
    /// Record a place from stdin JSON.
    #[command(after_help = crate::input_help::CLAIMANT_PLACE_ADD)]
    Add,
}

pub(crate) fn run(cmd: &ClaimantCmd, home: &Path) -> Result<Value, CliError> {
    let ledger = Ledger::open(home)?;
    let now = Instant::now();
    match cmd {
        ClaimantCmd::Place { cmd: PlaceCmd::Add } => {
            let input: NewPlace = read_stdin_json()?;
            Ok(json!({ "place": ledger.add_claimant_place(&input, now)? }))
        }
        ClaimantCmd::Add => {
            let input: FactInput = read_stdin_json()?;
            let fact = ledger.record_claimant_fact(&input.into_new_fact(), now)?;
            Ok(json!({ "fact": fact }))
        }
        ClaimantCmd::Correct => {
            let input: CorrectionInput = read_stdin_json()?;
            let new = input
                .into_new_fact()
                .map_err(aggountant_ledger::LedgerError::from)?;
            Ok(json!({ "fact": ledger.record_claimant_fact(&new, now)? }))
        }
        ClaimantCmd::Show {
            employee,
            date,
            as_known_at,
        } => {
            let date = parse_cli_date(date)?;
            let as_known_at = as_known_at
                .as_deref()
                .map(Instant::parse)
                .transpose()
                .map_err(|e| CliError::user(e.error_code(), e.to_string()))?;
            let view = ledger.claimant_view(employee, date, as_known_at, now)?;
            Ok(serde_json::to_value(view).expect("register view serializes"))
        }
        ClaimantCmd::History { employee } => {
            let history = ledger.claimant_history(employee)?;
            Ok(serde_json::to_value(history).expect("register history serializes"))
        }
        ClaimantCmd::Snapshot => {
            let request: SnapshotRequest = read_stdin_json()?;
            let snapshot = ledger.claimant_trip_snapshot(&request, now)?;
            let scope = trip_scope(&snapshot).map_err(aggountant_ledger::LedgerError::from)?;
            Ok(json!({ "snapshot": snapshot, "scope": scope }))
        }
    }
}

pub(crate) fn human_line(cmd: &ClaimantCmd, value: &Value) -> String {
    let fact_line = |value: &Value| {
        let fact = &value["fact"];
        match fact.get("supersedes").and_then(Value::as_i64) {
            Some(old) => format!(
                "recorded fact {} for {} (corrects fact {old})",
                fact["fact_id"], fact["employee_id"]
            ),
            None => format!(
                "recorded fact {} for {}",
                fact["fact_id"], fact["employee_id"]
            ),
        }
    };
    match cmd {
        ClaimantCmd::Place { .. } => format!(
            "recorded place {} for {}",
            value["place"]["place_id"], value["place"]["employee_id"]
        ),
        ClaimantCmd::Add | ClaimantCmd::Correct => fact_line(value),
        ClaimantCmd::Show { .. } => format!(
            "{} on {}: regular workplace {}; {} workplace entr{}, {} relationship(s)",
            value["employee_id"].as_str().unwrap_or("?"),
            value["date"].as_str().unwrap_or("?"),
            if value["regular_workplace_known"] == true {
                "recorded"
            } else {
                "not recorded"
            },
            value["workplaces"].as_array().map_or(0, Vec::len),
            if value["workplaces"].as_array().map_or(0, Vec::len) == 1 {
                "y"
            } else {
                "ies"
            },
            value["relationships"].as_array().map_or(0, Vec::len),
        ),
        ClaimantCmd::History { .. } => format!(
            "{}: {} place(s), {} fact row(s)",
            value["employee_id"].as_str().unwrap_or("?"),
            value["places"].as_array().map_or(0, Vec::len),
            value["facts"].as_array().map_or(0, Vec::len),
        ),
        ClaimantCmd::Snapshot => format!(
            "trip on {}: scope {}",
            value["snapshot"]["register"]["date"]
                .as_str()
                .unwrap_or("?"),
            value["scope"].as_str().unwrap_or("?")
        ),
    }
}
