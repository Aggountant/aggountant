//! `claim trip|line|show|list|discard`: the expense-claim draft (ADR-11
//! sections 1, 2 and 4). Thin: the draft's rules are the Domain's
//! (`aggountant_domain::expense_claim`), its files are the process crate's,
//! and this module turns them into JSON or the Markdown view.

use std::fmt::Write as _;
use std::path::Path;

use clap::{Args, Subcommand, ValueEnum};
use serde_json::{json, Value};

use aggountant_domain::expense_claim::{Finding, LineRecord, TripRecord};
use aggountant_domain::travel::fi::Workplace;
use aggountant_model::Instant;
use aggountant_process::expense_claim::{
    self as drafts, ClaimError, DraftTarget, DraftView, Written,
};

use crate::{parse_stdin_json, read_stdin_text, CliError};

#[derive(Subcommand, Debug)]
pub(crate) enum TripCmd {
    /// Add a trip from stdin JSON to an open draft (--claim) or to a new
    /// draft for a claimant (--new-for). Answers with the draft's open
    /// questions, refusals and checks.
    #[command(after_help = TRIP_INPUT)]
    Add {
        #[command(flatten)]
        target: Target,
        /// The trip's id in the draft (default t1, t2, …).
        #[arg(long)]
        trip: Option<String>,
        /// The id of the new draft (with --new-for).
        #[arg(long, requires = "new_for")]
        claim_id: Option<String>,
    },
    /// Replace a trip's record from stdin JSON (the whole record, as for
    /// `claim trip add`); its recording time stays.
    #[command(after_help = TRIP_INPUT)]
    Revise {
        #[arg(long)]
        claim: String,
        #[arg(long)]
        trip: String,
    },
    /// Remove a trip that no receipt line belongs to.
    Remove {
        #[arg(long)]
        claim: String,
        #[arg(long)]
        trip: String,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum LineCmd {
    /// Add a receipt line from stdin JSON to an open draft (--claim) or to a
    /// new draft for a claimant (--new-for).
    #[command(after_help = LINE_INPUT)]
    Add {
        #[command(flatten)]
        target: Target,
        /// The line's id in the draft (default l1, l2, …).
        #[arg(long)]
        line: Option<String>,
        /// The id of the new draft (with --new-for).
        #[arg(long, requires = "new_for")]
        claim_id: Option<String>,
    },
    /// Replace a receipt line from stdin JSON (the whole record).
    #[command(after_help = LINE_INPUT)]
    Revise {
        #[arg(long)]
        claim: String,
        #[arg(long)]
        line: String,
    },
    /// Remove a receipt line.
    Remove {
        #[arg(long)]
        claim: String,
        #[arg(long)]
        line: String,
    },
}

/// The draft a new trip or line goes to.
#[derive(Args, Debug)]
#[group(required = true, multiple = false, id = "target")]
pub(crate) struct Target {
    /// An open draft.
    #[arg(long)]
    claim: Option<String>,
    /// Start a new draft for this claimant (an employee_id in the claimant
    /// register). Its id is the claimant and the first date, e.g.
    /// maija-2026-03-10, unless --claim-id names it.
    #[arg(long, value_name = "EMPLOYEE_ID")]
    new_for: Option<String>,
}

impl Target {
    fn draft_target<'a>(&'a self, claim_id: Option<&'a str>) -> DraftTarget<'a> {
        match (&self.claim, &self.new_for) {
            (Some(claim), _) => DraftTarget::Claim(claim),
            (None, Some(employee_id)) => DraftTarget::New {
                employee_id,
                claim_id,
            },
            (None, None) => unreachable!("clap requires --claim or --new-for"),
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    Json,
    Markdown,
}

impl From<ClaimError> for CliError {
    fn from(e: ClaimError) -> Self {
        match e {
            ClaimError::Ledger(e) => e.into(),
            ClaimError::Refused { code, message } => CliError::user(code, message),
            ClaimError::Io(_) => CliError::system(e.error_code(), e.to_string()),
        }
    }
}

pub(crate) fn trip(cmd: &TripCmd, home: &Path) -> Result<Value, CliError> {
    let now = Instant::now();
    let (view, written) = match cmd {
        TripCmd::Add {
            target,
            trip,
            claim_id,
        } => {
            let record: TripRecord = parse_stdin_json(&read_stdin_text()?)?;
            let target = target.draft_target(claim_id.as_deref());
            drafts::add_trip(home, target, trip.as_deref(), record, now)?
        }
        TripCmd::Revise { claim, trip } => {
            let record: TripRecord = parse_stdin_json(&read_stdin_text()?)?;
            drafts::revise_trip(home, claim, trip, record, now)?
        }
        TripCmd::Remove { claim, trip } => drafts::remove_trip(home, claim, trip, now)?,
    };
    Ok(written_json(&view, &written))
}

pub(crate) fn line(cmd: &LineCmd, home: &Path) -> Result<Value, CliError> {
    let now = Instant::now();
    let (view, written) = match cmd {
        LineCmd::Add {
            target,
            line,
            claim_id,
        } => {
            let record: LineRecord = parse_stdin_json(&read_stdin_text()?)?;
            let target = target.draft_target(claim_id.as_deref());
            drafts::add_line(home, target, line.as_deref(), record, now)?
        }
        LineCmd::Revise { claim, line } => {
            let record: LineRecord = parse_stdin_json(&read_stdin_text()?)?;
            drafts::revise_line(home, claim, line, record, now)?
        }
        LineCmd::Remove { claim, line } => drafts::remove_line(home, claim, line, now)?,
    };
    Ok(written_json(&view, &written))
}

pub(crate) fn show(claim: &str, format: Format, home: &Path) -> Result<Value, CliError> {
    let view = drafts::show(home, claim, Instant::now())?;
    Ok(match format {
        Format::Json => view_json(&view),
        Format::Markdown => json!({
            "schema_version": 1,
            "claim_id": view.draft.claim_id,
            "revision_hash": view.revision_hash,
            "markdown": markdown(&view),
        }),
    })
}

pub(crate) fn list(home: &Path) -> Result<Value, CliError> {
    let drafts = drafts::list(home)?;
    Ok(json!({ "schema_version": 1, "count": drafts.len(), "drafts": drafts }))
}

pub(crate) fn discard(claim: &str, reason: &str, home: &Path) -> Result<Value, CliError> {
    let record = drafts::discard(home, claim, reason, Instant::now())?;
    Ok(json!({ "schema_version": 1, "discarded": record }))
}

fn written_json(view: &DraftView, written: &Written) -> Value {
    let mut value = view_json(view);
    value["written"] = serde_json::to_value(written).expect("written serializes");
    value
}

/// The JSON view of a draft: its content, the review and the next step.
fn view_json(view: &DraftView) -> Value {
    let review = &view.review;
    let trips: Vec<Value> = view
        .draft
        .trips
        .iter()
        .map(|t| {
            let status = review.trips.iter().find(|s| s.trip_id == t.trip_id);
            json!({
                "trip_id": t.trip_id,
                "recorded_at": t.recorded_at,
                "revised_at": t.revised_at,
                "recorded_afterwards": status.map(|s| s.recorded_afterwards),
                "workplace": status.and_then(|s| s.workplace),
                "record": t.record,
            })
        })
        .collect();
    json!({
        "schema_version": 1,
        "claim_id": view.draft.claim_id,
        "employee_id": view.draft.employee_id,
        "revision": view.draft.revision,
        "revision_hash": view.revision_hash,
        "path": view.path,
        "ready_for_approval": review.ready_for_approval,
        "open_questions": review.open_questions,
        "refusals": review.refusals,
        "checks": review.checks,
        "totals": review.totals,
        "trips": trips,
        "lines": view.draft.lines,
        "allowances": review.allowances,
        "next_step": next_step(view),
    })
}

fn next_step(view: &DraftView) -> String {
    let r = &view.review;
    let id = &view.draft.claim_id;
    if !r.open_questions.is_empty() {
        format!("Answer the open questions with `claim trip revise --claim {id}` or `claim line revise --claim {id}`.")
    } else if !r.refusals.is_empty() {
        "Resolve the refusals: each names what blocks approval and what settles it.".into()
    } else {
        format!("The draft is complete; review it with `claim show --claim {id} --format markdown`. Approving and sealing a claim is not available yet.")
    }
}

pub(crate) fn human_line(value: &Value) -> String {
    if let Some(markdown) = value["markdown"].as_str() {
        return markdown.trim_end().to_string();
    }
    if let Some(record) = value.get("discarded") {
        return format!(
            "claim draft {} discarded",
            record["claim_id"].as_str().unwrap_or("?")
        );
    }
    if let Some(drafts) = value["drafts"].as_array() {
        if drafts.is_empty() {
            return "no open claim drafts".into();
        }
        return drafts
            .iter()
            .map(|d| match d["invalid"].as_str() {
                Some(why) => format!("{}: invalid: {why}", d["claim_id"].as_str().unwrap_or("?")),
                None => format!(
                    "{} ({}): {} trips, {} lines",
                    d["claim_id"].as_str().unwrap_or("?"),
                    d["employee_id"].as_str().unwrap_or("?"),
                    d["trips"],
                    d["lines"]
                ),
            })
            .collect::<Vec<_>>()
            .join("\n");
    }
    let count = |key: &str| value[key].as_array().map_or(0, Vec::len);
    let action = value["written"]["action"]
        .as_str()
        .map(|a| format!("{}; ", a.replace('_', " ")))
        .unwrap_or_default();
    format!(
        "claim draft {} revision {}: {action}{} open questions, {} refusals, {} checks; total {} €",
        value["claim_id"].as_str().unwrap_or("?"),
        value["revision"],
        count("open_questions"),
        count("refusals"),
        count("checks"),
        value["totals"]["total"].as_str().unwrap_or("?"),
    )
}

// ---------------------------------------------------------------------------
// The Markdown view
// ---------------------------------------------------------------------------

/// The readable claim, rendered on request and never stored.
fn markdown(view: &DraftView) -> String {
    let draft = &view.draft;
    let r = &view.review;
    let mut out = String::new();
    let _ = writeln!(out, "# Expense claim draft {}\n", draft.claim_id);
    let _ = writeln!(out, "- Claimant: {}", draft.employee_id);
    let _ = writeln!(
        out,
        "- Revision {} (sha256 {})",
        draft.revision, view.revision_hash
    );
    let status = if r.ready_for_approval {
        "complete; amounts are final only when the claim is approved".to_string()
    } else {
        format!(
            "not ready: {} open questions, {} refusals",
            r.open_questions.len(),
            r.refusals.len()
        )
    };
    let _ = writeln!(out, "- Status: {status}\n");

    for (title, findings) in [
        ("Open questions", &r.open_questions),
        ("Refusals", &r.refusals),
        ("Checks", &r.checks),
    ] {
        if findings.is_empty() {
            continue;
        }
        let _ = writeln!(out, "## {title}\n");
        for f in findings {
            let _ = writeln!(
                out,
                "- `{}`{}: {}",
                f.code,
                subject(f),
                one_line(&f.message)
            );
        }
        out.push('\n');
    }

    if !draft.trips.is_empty() {
        let _ = writeln!(out, "## Trips\n");
    }
    for trip in &draft.trips {
        let rec = &trip.record;
        let computed = r.allowances.as_ref().and_then(|c| {
            c.trips
                .iter()
                .find(|t| t.id.as_deref() == Some(trip.trip_id.as_str()))
        });
        let status = r.trips.iter().find(|s| s.trip_id == trip.trip_id);
        let _ = writeln!(
            out,
            "### {}: {}\n",
            trip.trip_id,
            text_or(&rec.purpose, "purpose not given")
        );
        let when = match computed {
            Some(c) => format!(
                "{} – {} (Finnish time)",
                c.start_finnish_time, c.end_finnish_time
            ),
            None => format!(
                "{} – {}",
                rec.trip["start"].as_str().unwrap_or("?"),
                rec.trip["end"].as_str().unwrap_or("?")
            ),
        };
        let _ = writeln!(out, "- When: {when}");
        let _ = writeln!(
            out,
            "- Destination: {}",
            text_or(&rec.destination, "not given")
        );
        if let Some(route) = &rec.route {
            let _ = writeln!(out, "- Route: {}", one_line(route));
        }
        let _ = writeln!(
            out,
            "- Mode of travel: {}",
            text_or(&rec.travel_mode, "not given")
        );
        let workplace = match status.and_then(|s| s.workplace) {
            Some(Workplace::SpecialWorkPlace) => "erityinen työntekemispaikka (special work site)",
            Some(Workplace::SecondaryWorkplace) => "toissijainen työpaikka (secondary workplace)",
            Some(Workplace::RegularWorkplace) => "varsinainen työpaikka (regular workplace)",
            None => "not classified (see the refusals and questions)",
        };
        let _ = writeln!(out, "- Workplace: {workplace}");
        let mut recorded = format!(
            "{}; recorded {}",
            source_name(rec.source),
            trip.recorded_at.to_whole_seconds_z()
        );
        if let Some(revised) = trip.revised_at {
            let _ = write!(recorded, ", revised {}", revised.to_whole_seconds_z());
        }
        if status.is_some_and(|s| s.recorded_afterwards) {
            recorded.push_str("; **recorded afterwards**");
        }
        let _ = writeln!(out, "- Source: {recorded}");
        let evidence =
            rec.evidence.len() + rec.drives.iter().map(|d| d.evidence.len()).sum::<usize>();
        if evidence > 0 {
            let _ = writeln!(out, "- Evidence: {evidence} attached files");
        }
        if let Some(note) = &rec.note {
            let _ = writeln!(out, "- Note: {}", one_line(note));
        }
        out.push('\n');
        if let Some(c) = computed {
            if !c.days.is_empty() {
                let _ = writeln!(
                    out,
                    "| Day | From | Until | Per diem | Free meals | Amount |\n|---|---|---|---|---|---|"
                );
                for d in &c.days {
                    let _ = writeln!(
                        out,
                        "| {} | {} | {} | {} | {} | {} |",
                        d.day,
                        d.from_finnish_time,
                        d.until_finnish_time,
                        d.basis.replace('_', " "),
                        d.free_meals.map_or("unanswered".into(), |n| n.to_string()),
                        d.amount.as_deref().unwrap_or("–"),
                    );
                }
                out.push('\n');
            }
            if !c.drives.is_empty() {
                let _ = writeln!(
                    out,
                    "| Drive | km | Vehicle | Own or controlled | Amount |\n|---|---|---|---|---|"
                );
                for (d, stored) in c.drives.iter().zip(&rec.drives) {
                    let vehicle = serde_json::to_value(d.vehicle)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_string))
                        .unwrap_or_default();
                    let own = match stored.own_or_controlled_vehicle {
                        Some(true) => "yes",
                        Some(false) => "no",
                        None => "unanswered",
                    };
                    let _ = writeln!(
                        out,
                        "| {} | {} | {} | {own} | {} |",
                        d.date,
                        d.km,
                        vehicle.replace('_', " "),
                        d.amount_exact.as_deref().unwrap_or("–"),
                    );
                }
                out.push('\n');
            }
        }
    }

    if !draft.lines.is_empty() {
        let _ = writeln!(
            out,
            "## Receipt lines\n\n| Line | Date | Description | Trip | Receipt | Amount |\n|---|---|---|---|---|---|"
        );
        for line in &draft.lines {
            let rec = &line.record;
            let receipt = match (&rec.receipt_hash, &rec.no_receipt_reason) {
                (Some(hash), _) => format!("sha256 {}…", &hash.to_hex()[..12]),
                (None, Some(reason)) => format!("none: {}", cell(reason)),
                (None, None) => "missing".into(),
            };
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {receipt} | {} |",
                line.line_id,
                rec.receipt_date,
                cell(&rec.description),
                rec.trip_id.as_deref().unwrap_or("–"),
                rec.amount,
            );
        }
        out.push('\n');
    }

    let _ = writeln!(
        out,
        "## Totals\n\n| | € |\n|---|---|\n| Scheduled allowances | {} |\n| Receipt lines | {} |\n| Total | {} |",
        r.totals.allowances, r.totals.receipts, r.totals.total
    );
    if let Some(c) = &r.allowances {
        let decisions: Vec<String> = c
            .rate_table
            .editions_used
            .iter()
            .map(|e| e.year.to_string())
            .collect();
        if !decisions.is_empty() {
            let _ = writeln!(
                out,
                "\nAllowances computed with the Tax Administration's travel allowance decision for {}.",
                decisions.join(" and ")
            );
        }
    }
    out
}

fn subject(f: &Finding) -> String {
    let mut parts = Vec::new();
    if let Some(trip) = &f.trip_id {
        parts.push(format!("trip {trip}"));
    }
    if let Some(day) = f.day {
        parts.push(format!("day {day}"));
    }
    if let Some(line) = &f.line_id {
        parts.push(format!("line {line}"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    }
}

fn source_name(source: aggountant_domain::expense_claim::Source) -> &'static str {
    use aggountant_domain::expense_claim::Source::*;
    match source {
        UserStatement => "user statement",
        ImportedRecord => "imported record",
        ConfirmedReconstruction => "reconstruction confirmed by the user",
    }
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_or(value: &Option<String>, missing: &str) -> String {
    value
        .as_deref()
        .map(one_line)
        .unwrap_or_else(|| format!("_{missing}_"))
}

fn cell(text: &str) -> String {
    one_line(text).replace('|', "\\|")
}

pub(crate) const TRIP_INPUT: &str = r#"INPUT (JSON on stdin)
  One trip record. Required: source ("user_statement", "imported_record" or
  "confirmed_reconstruction": the agent's reconstruction, for example from a
  calendar, that the user confirmed drive by drive; a proposal the user has
  not confirmed is not written) and trip: the trip as `claim compute --help`
  describes one, without id (the draft gives it from trip_id) and drives.
  The travel claim's statutory content (ennakkoperintäasetus 17 §): purpose,
  destination, travel_mode, and route when there are drives; a missing one
  is an open question, not an error.
  Answers the draft asks for and never takes as "none" when left out:
  trip.places.start_place_id (null when the trip started from a place not in
  the register), trip.places.destination_place_ids ([] when the trip went
  only to places the register does not name), trip.days[].free_meals and trip.days[].company_paid_meals
  ([] when none) for each computed per diem day, and each drive's
  own_or_controlled_vehicle.
  drives: [{drive: {date, km, vehicle, ...} as in `claim compute --help`,
  own_or_controlled_vehicle (true/false), evidence: [sha256, ...]}].
  Optional: evidence (sha256 of files attached with `artifact attach`, such
  as tickets or a meeting invitation), note (free text).
  The computed allowances are never typed in; trip.paid may only lower them.
  Revise replaces the whole record; run `claim show --claim ID` to get the
  current one.
OUTPUT
  The draft: open_questions, refusals and checks [{code, message, trip_id,
  line_id, day, field}], ready_for_approval, totals, trips, lines, the
  computed allowances (as `claim compute`), next_step, and written. A
  question or refusal blocks approval, never the write. Malformed input is
  INVALID_INPUT, CLAIM_INPUT_INVALID or TRAVEL_INPUT_INVALID; nothing is then
  written.

Example JSON (synthetic trip):
```json
{"source":"user_statement","purpose":"Asiakastapaaminen","destination":"Asiakas Oy, Tampere","route":"Helsinki–Tampere–Helsinki","travel_mode":"oma auto","trip":{"kind":"domestic","start":"2026-03-10T07:30:00+02:00","end":"2026-03-10T19:15:00+02:00","places":{"start_place_id":"home","destination_place_ids":["customer"]},"distances_km":{"from_departure":"172","from_home":"172","from_regular_workplace":"170"},"days":[{"free_meals":1,"company_paid_meals":[]}]},"drives":[{"drive":{"date":"2026-03-10","km":"344.6","vehicle":"car"},"own_or_controlled_vehicle":true}]}
```
  aggountant --home ./accounting --json claim trip add --new-for maija < trip.json
  aggountant --home ./accounting --json claim trip revise --claim maija-2026-03-10 --trip t1 < trip.json"#;

pub(crate) const LINE_INPUT: &str = r#"INPUT (JSON on stdin)
  One receipt line: a cost the claimant paid that the company reimburses.
  Required: source ("user_statement", "imported_record" or
  "confirmed_reconstruction"), receipt_date (YYYY-MM-DD), amount (euros paid,
  a decimal string such as "12.40"), description.
  receipt_hash: the sha256 `artifact attach --file <receipt>` returned; the
  receipt goes to the artifact store first. Without one, no_receipt_reason
  says why; with neither, the line has an open question.
  Optional: trip_id (the trip the cost belongs to: its costs follow the
  trip's workplace classification), cost_kind ("travel", "lodging" or
  "other"; asked for a cost on a trip to a secondary workplace), note.
  The VAT treatment is decided when the claim is approved, as
  capture-receipt decides it for the same receipt.
OUTPUT
  The draft, as for `claim trip add`. A receipt already attached to a
  sealed voucher or to another line is a refusal.

Example JSON (synthetic receipt; receipt_hash is what `artifact attach` returned):
```json
{"source":"user_statement","receipt_hash":"9f2c9f2c9f2c9f2c9f2c9f2c9f2c9f2c9f2c9f2c9f2c9f2c9f2c9f2c9f2c9f2c","receipt_date":"2026-03-10","amount":"12.40","description":"Pysäköinti, Tampere","trip_id":"t1"}
```
  aggountant --home ./accounting --json claim line add --claim maija-2026-03-10 < line.json
  aggountant --home ./accounting --json claim line revise --claim maija-2026-03-10 --line l1 < line.json"#;
