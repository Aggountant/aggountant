//! `claim`: expense claims. `claim compute` computes the scheduled travel
//! allowances of a claimant's trips on stdin, with each trip's workplace
//! classified by the claimant's regular-workplace register, and writes
//! nothing. The draft commands (`claim trip`, `claim line`, `claim show`,
//! `claim list`, `claim discard`) are in `claim_draft`.

use std::path::Path;

use clap::Subcommand;
use serde_json::{json, Value};

use aggountant_domain::travel::fi::{compute_claim, ClaimInput};
use aggountant_ledger::Ledger;
use aggountant_model::Instant;

use crate::{claim_draft, parse_stdin_json, read_stdin_text, CliError};

#[derive(Subcommand, Debug)]
pub(crate) enum ClaimCmd {
    /// Compute the scheduled tax-free travel allowances of trips (read-only
    /// preview): travel days, per diems, meal, night and mileage allowances,
    /// Incomes Register lines and proposed postings, with the rate-table
    /// edition used. Refusals are part of the result (exit 0).
    #[command(after_help = CLAIM_COMPUTE)]
    Compute,
    /// The trips of a claim draft: add, revise or remove.
    Trip {
        #[command(subcommand)]
        cmd: claim_draft::TripCmd,
    },
    /// The receipt lines of a claim draft: add, revise or remove.
    Line {
        #[command(subcommand)]
        cmd: claim_draft::LineCmd,
    },
    /// Show a claim draft with its open questions, refusals, checks and
    /// computed allowances, as JSON or as the readable Markdown claim.
    /// Writes nothing.
    Show {
        #[arg(long)]
        claim: String,
        #[arg(long, value_enum, default_value = "json")]
        format: claim_draft::Format,
    },
    /// List the open claim drafts.
    List,
    /// Discard a claim draft. Its id, last revision hash and the reason are
    /// kept in the discard log; its content is removed.
    Discard {
        #[arg(long)]
        claim: String,
        #[arg(long)]
        reason: String,
    },
}

pub(crate) fn run(cmd: &ClaimCmd, home: &Path) -> Result<Value, CliError> {
    match cmd {
        ClaimCmd::Compute => compute(&read_stdin_text()?, home),
        ClaimCmd::Trip { cmd } => claim_draft::trip(cmd, home),
        ClaimCmd::Line { cmd } => claim_draft::line(cmd, home),
        ClaimCmd::Show { claim, format } => claim_draft::show(claim, *format, home),
        ClaimCmd::List => claim_draft::list(home),
        ClaimCmd::Discard { claim, reason } => claim_draft::discard(claim, reason, home),
    }
}

/// The preview of the claim in `input`, classified by the register in `home`.
pub(crate) fn compute(input: &str, home: &Path) -> Result<Value, CliError> {
    let input: ClaimInput = parse_stdin_json(input)?;
    let register = Ledger::open(home)?.claimant_history(&input.employee_id)?;
    let computed = compute_claim(&input, &register.places, &register.facts, Instant::now())?;
    let mut value = json!({ "schema_version": 1 });
    let object = value.as_object_mut().expect("object");
    if let Value::Object(fields) = serde_json::to_value(computed).expect("computation serializes") {
        object.extend(fields);
    }
    Ok(value)
}

pub(crate) fn human_line(cmd: &ClaimCmd, value: &Value) -> String {
    match cmd {
        ClaimCmd::Trip { .. }
        | ClaimCmd::Line { .. }
        | ClaimCmd::Show { .. }
        | ClaimCmd::List
        | ClaimCmd::Discard { .. } => claim_draft::human_line(value),
        ClaimCmd::Compute => {
            let totals = &value["totals"];
            let amounts = format!(
                "total {} € (per diems {}, meal {}, night {}, mileage {} for {} km)",
                totals["total"].as_str().unwrap_or("?"),
                totals["daily_allowances"].as_str().unwrap_or("?"),
                totals["meal_allowances"].as_str().unwrap_or("?"),
                totals["night_allowances"].as_str().unwrap_or("?"),
                totals["mileage"].as_str().unwrap_or("?"),
                totals["mileage_km"].as_i64().unwrap_or(0),
            );
            let refusals: Vec<&str> = value["refusals"]
                .as_array()
                .map(|r| r.iter().filter_map(|x| x["code"].as_str()).collect())
                .unwrap_or_default();
            if refusals.is_empty() {
                format!("claim preview: {amounts}; nothing written")
            } else {
                format!(
                    "claim preview refused ({}): {amounts}; nothing written",
                    refusals.join(", ")
                )
            }
        }
    }
}

pub(crate) const CLAIM_COMPUTE: &str = r#"INPUT (JSON on stdin)
  {"employee_id":"...","trips":[trip, ...]}: the claimant (a person in the
  claimant register, `claimant add`) and 1..100 trips of one claim. Each trip:
  Required: kind ("domestic" or "foreign"); start and end (RFC 3339 with an
  offset, e.g. 2026-03-10T08:00:00+02:00: leaving and returning home or to the
  workplace); places {start_place_id, destination_place_ids}: the registered
  places the trip starts from and goes to for work (a destination the register
  does not name is a special work site and is not listed). The register
  classifies the trip for its start date, as `claimant snapshot` shows:
  erityinen työntekemispaikka (all allowances), toissijainen työpaikka
  (mileage only) or varsinainen työpaikka (refused,
  TRAVEL_TO_REGULAR_WORKPLACE); without a relationship or a regular-workplace
  entry on that date the trip is refused (CLAIMANT_RELATIONSHIP_UNKNOWN,
  REGULAR_WORKPLACE_UNKNOWN) and computes nothing; distances_km {from_departure, from_home,
  from_regular_workplace} as decimal strings along the shortest route
  (from_regular_workplace is required, null only when the register records no
  regular workplace for the claimant, else TRAVEL_INPUT_INVALID;
  a per diem needs over 15 km from where the trip started and over 5 km from
  both).
  Foreign trips: destination_country (ISO 3166 code) and optional
  destination_region (a city or region the decision lists: london, edinburgh,
  hong-kong, istanbul, moscow, st-petersburg, new-york, los-angeles,
  washington, azores, madeira, canary-islands).
  days: answers per computed per diem day, day 1 first. free_meals (count of
  free meals: employer-arranged, or included in a ticket, a course package
  or abroad the hotel room price; a breakfast is not one; required, 0 when
  none), company_paid_meals (["12.40"]: the company's share of meals the
  traveller bought on the company's account), end_country and end_region
  (foreign trips: where each full 24-hour day ended, FI for Finland), lodging
  ("provided", "self_arranged" or "night_travel_or_work"; asked when
  night_allowance is true and the day has 4 hours between 21 and 07 Finnish
  time; night allowance on a foreign trip is refused).
  Run once without days to see the computed days (trips[].days, and for a
  foreign trip trips[].full_day_ends_finnish_time), then answer them.
  Optional: id; leisure {counted_from, counted_until, outbound_travel_minutes,
  return_travel_minutes} when leisure is combined with the trip;
  meal_allowance {usual_eating_place_available, lunch_benefit_usable,
  employer_arranged_meal, meals (1, or 2 when the working day with breaks is
  over 11 hours)} for a trip without per diem; night_allowance (true to claim night
  travel allowances); drives [{date, km (at most 2 decimals), vehicle ("car",
  "motorboat_up_to_50hp", "motorboat_over_50hp", "snowmobile", "atv",
  "motorcycle", "moped", "other"), surcharges (car: "trailer", "caravan",
  "heavy_load", "heavy_equipment", "dog"), forest_road_km, passengers,
  car_benefit}]; situations (each refuses the trip:
  "starts_from_holiday_home", "workplace_changes",
  "border_crossing_allowance", "posting", "work_abroad",
  "special_industry_daily_journey"); paid
  {mileage, daily_allowances, meal_allowance, night_allowances} when the
  company pays less than the maximum (more is refused).
OUTPUT
  approvable, rate_table (table hash and the decisions used), trips (days and
  drives with exact amounts), totals (rounded once per claim, half away from
  zero), incomes_register_lines (311, 331), postings (one proposed voucher per
  month a trip ends in), refusals [{code, message, trip, day}] and notes on
  the rules the result depends on. A refusal blocks approval; the command
  still exits 0. Malformed input is TRAVEL_INPUT_INVALID or INVALID_INPUT; an
  employee_id not in the register is EMPLOYEE_NOT_FOUND. Nothing is written.

Example JSON (synthetic trip):
```json
{"employee_id":"maija","trips":[{"id":"tampere-2026-03","kind":"domestic","start":"2026-03-10T07:30:00+02:00","end":"2026-03-10T19:15:00+02:00","places":{"start_place_id":"home","destination_place_ids":["customer"]},"distances_km":{"from_departure":"172","from_home":"172","from_regular_workplace":"170"},"days":[{"free_meals":1}],"drives":[{"date":"2026-03-10","km":"344.6","vehicle":"car"}]}]}
```
  aggountant --home ./accounting --json claim compute < trip.json"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// Record the synthetic claimant of the examples in `home`: Maija, an
    /// employee whose regular workplace is the office, with her home and a
    /// customer site registered.
    pub(crate) fn record_claimant(home: &Path) {
        use aggountant_domain::workplace::{FactInput, NewPlace};
        let ledger = Ledger::open(home).unwrap();
        ledger
            .upsert_employee(&aggountant_ledger::EmployeeState {
                employee_id: "maija".into(),
                name: "Maija Meikäläinen".into(),
                hetu: "131052-308T".into(),
                iban: "FI2112345600000785".into(),
                tax_card: aggountant_domain::TaxCard {
                    withholding_pct: "0.00".into(),
                },
            })
            .unwrap();
        let now = Instant::now();
        for (id, kind) in [
            ("home", "home"),
            ("office", "company_premises"),
            ("customer", "customer_site"),
        ] {
            let place: NewPlace = serde_json::from_value(json!({
                "employee_id": "maija", "place_id": id, "kind": kind, "label": id,
                "location": "Esimerkkikatu 1, Tampere", "municipality": "Tampere",
                "confirmed_by": "Maija Meikäläinen"
            }))
            .unwrap();
            ledger.add_claimant_place(&place, now).unwrap();
        }
        for fact in [
            json!({"kind": "relationship", "relationship": "employee"}),
            json!({"kind": "workplace", "role": "regular_primary", "place_id": "office",
                   "basis": "works_there_regularly"}),
        ] {
            let input: FactInput = serde_json::from_value(json!({
                "employee_id": "maija", "valid_from": "2020-01-01", "fact": fact,
                "confirmed_by": "Maija Meikäläinen"
            }))
            .unwrap();
            ledger
                .record_claimant_fact(&input.into_new_fact(), now)
                .unwrap();
        }
    }

    #[test]
    fn example_computes() {
        let example = CLAIM_COMPUTE
            .split_once("```json\n")
            .and_then(|(_, rest)| rest.split_once("\n```"))
            .unwrap()
            .0;
        let dir = tempfile::tempdir().unwrap();
        Ledger::init(dir.path()).unwrap();
        record_claimant(dir.path());
        let value = compute(example, dir.path()).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["approvable"], true);
        assert_eq!(value["totals"]["daily_allowances"], "54.00");
        assert_eq!(value["totals"]["mileage"], "189.53");
        assert_eq!(value["totals"]["mileage_km"], 345);
    }

    /// Companies that do not use expense claims see no change (ADR-11): in a
    /// synthetic home without claims, the existing commands never read the
    /// travel rate table, and a claim preview writes nothing, so their output
    /// and the home's files are identical before and after it. The read count
    /// is per thread, so tests running in parallel cannot disturb it.
    #[test]
    fn commands_unrelated_to_claims_do_not_read_the_rate_table() {
        use aggountant_domain::travel::fi::rate_table_reads_on_this_thread as reads;
        use clap::Parser;

        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("books");
        let home = home.to_str().unwrap();
        let run = |args: &[&str]| -> Value {
            let mut argv = vec!["aggountant", "--json", "--home", home];
            argv.extend_from_slice(args);
            let cli = crate::Cli::try_parse_from(argv).unwrap();
            crate::run(&cli).unwrap_or_else(|e| panic!("{args:?}: {} {}", e.code, e.message))
        };
        let commands: &[&[&str]] = &[
            &["chart", "list"],
            &["period", "show", "--period", "2026-03"],
            &["fiscal-year", "list"],
            &["obligation", "list"],
            &["intake", "list"],
            &["knowledge", "company", "list"],
            &[
                "voucher",
                "list",
                "--posting-from",
                "2026-01-01",
                "--posting-to",
                "2026-12-31",
            ],
            &[
                "report",
                "statements",
                "--period",
                "2026-03",
                "--company",
                "Esimerkki Oy",
            ],
            &["explain"],
            &["skill", "list"],
        ];

        let start = reads();
        run(&["init"]);
        record_claimant(std::path::Path::new(home));
        let before: Vec<Value> = commands.iter().map(|c| run(c)).collect();
        assert_eq!(
            reads(),
            start,
            "a command unrelated to claims read the travel rate table"
        );
        let files_before = tree(dir.path());

        compute(
            r#"{"employee_id":"maija","trips":[{"kind":"domestic","start":"2026-03-10T08:00:00+02:00","end":"2026-03-10T19:00:00+02:00","places":{"start_place_id":"home","destination_place_ids":[]},"distances_km":{"from_departure":"80","from_home":"80","from_regular_workplace":"78"},"days":[{"free_meals":0}]}]}"#,
            std::path::Path::new(home),
        )
        .unwrap();
        assert!(reads() > start, "the claim preview reads the table");

        let after_preview = reads();
        let after: Vec<Value> = commands.iter().map(|c| run(c)).collect();
        assert_eq!(reads(), after_preview);
        assert_eq!(before, after);
        assert_eq!(
            files_before,
            tree(dir.path()),
            "the preview wrote into the home"
        );

        // Listing the drafts of a company without claims finds none and
        // creates nothing: no draft directory, no .gitignore, no question.
        assert_eq!(run(&["claim", "list"])["count"], 0);
        assert_eq!(files_before, tree(dir.path()), "claim list wrote");
        assert!(!std::path::Path::new(home)
            .join(aggountant_process::expense_claim::CLAIM_DRAFT_DIR)
            .exists());
    }

    /// Every file under `root` with its content.
    fn tree(root: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    out.push((path.clone(), std::fs::read(&path).unwrap()));
                }
            }
        }
        out.sort();
        out
    }
}
