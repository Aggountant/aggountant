//! `aggountant claim trip|line|show|list|discard`: collecting an expense
//! claim draft, with Esimerkki Oy's synthetic claimant Maija Meikäläinen.

mod common;

use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn run(home: &Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aggountant"));
    command.arg("--home").arg(home).arg("--json").args(args);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    common::feed_stdin(&mut child, stdin.as_bytes());
    let output = child.wait_with_output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn ok(home: &Path, args: &[&str], stdin: &Value) -> Value {
    let (code, stdout, stderr) = run(home, args, &stdin.to_string());
    assert_eq!(code, 0, "{args:?}: {stderr}");
    serde_json::from_str(&stdout).unwrap()
}

fn refused(home: &Path, args: &[&str], stdin: &Value) -> String {
    let (code, stdout, stderr) = run(home, args, &stdin.to_string());
    assert_eq!(code, 1, "{args:?} should be refused: {stdout}");
    let error: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(error["error_kind"], "user", "{stderr}");
    error["error_code"].as_str().unwrap().to_string()
}

/// A home with Maija: an employee whose regular workplace is the Helsinki
/// office, with her home and a customer site in Tampere registered.
fn claimant_home() -> tempfile::TempDir {
    let dir = common::initialized_home();
    let home = dir.path();
    ok(
        home,
        &["employee", "add"],
        &json!({"employee_id": "maija", "name": "Maija Meikäläinen", "hetu": "131052-308T",
                "iban": "FI2112345600000785", "tax_card": {"withholding_pct": "20.00"}}),
    );
    for (id, kind, municipality) in [
        ("home", "home", "Helsinki"),
        ("office", "company_premises", "Helsinki"),
        ("customer", "customer_site", "Tampere"),
    ] {
        ok(
            home,
            &["claimant", "place", "add"],
            &json!({"employee_id": "maija", "place_id": id, "kind": kind, "label": id,
                    "location": format!("Esimerkkikatu 1, {municipality}"),
                    "municipality": municipality, "confirmed_by": "Maija Meikäläinen"}),
        );
    }
    for fact in [
        json!({"kind": "relationship", "relationship": "employee"}),
        json!({"kind": "workplace", "role": "regular_primary", "place_id": "office",
               "basis": "works_there_regularly"}),
    ] {
        ok(
            home,
            &["claimant", "add"],
            &json!({"employee_id": "maija", "valid_from": "2020-01-01", "fact": fact,
                    "confirmed_by": "Maija Meikäläinen"}),
        );
    }
    dir
}

/// A complete day trip to the customer in Tampere on `date`, by own car.
fn trip(date: &str) -> Value {
    json!({
        "source": "user_statement",
        "purpose": "Asiakastapaaminen",
        "destination": "Asiakas Oy, Tampere",
        "route": "Helsinki–Tampere–Helsinki",
        "travel_mode": "oma auto",
        "trip": {
            "kind": "domestic",
            "start": format!("{date}T07:30:00+02:00"),
            "end": format!("{date}T19:15:00+02:00"),
            "places": {"start_place_id": "home", "destination_place_ids": ["customer"]},
            "distances_km": {"from_departure": "172", "from_home": "172", "from_regular_workplace": "170"},
            "days": [{"free_meals": 0, "company_paid_meals": []}]
        },
        "drives": [{"drive": {"date": date, "km": "344", "vehicle": "car"},
                    "own_or_controlled_vehicle": true}]
    })
}

fn attach(home: &Path, content: &str) -> String {
    let file = home.join(format!("{}.txt", content.len()));
    std::fs::write(&file, content).unwrap();
    let out = ok(
        home,
        &["artifact", "attach", "--file", file.to_str().unwrap()],
        &json!(null),
    );
    std::fs::remove_file(file).unwrap();
    out["artifact_ref"].as_str().unwrap().to_string()
}

fn line(hash: &str, date: &str, amount: &str, trip: Option<&str>) -> Value {
    json!({"source": "user_statement", "receipt_hash": hash, "receipt_date": date,
           "amount": amount, "description": "Pysäköinti", "trip_id": trip})
}

fn codes(findings: &Value) -> Vec<String> {
    findings
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["code"].as_str().unwrap().to_string())
        .collect()
}

#[test]
fn a_single_trip_claim_is_built_shown_and_discarded() {
    let dir = claimant_home();
    let home = dir.path();
    let added = ok(
        home,
        &["claim", "trip", "add", "--new-for", "maija"],
        &trip("2026-03-10"),
    );
    assert_eq!(added["claim_id"], "maija-2026-03-10");
    assert_eq!(added["written"]["action"], "trip_added");
    assert_eq!(added["written"]["trip_id"], "t1");
    assert_eq!(added["revision"], 1);
    assert_eq!(added["open_questions"], json!([]));
    assert_eq!(added["refusals"], json!([]));
    assert_eq!(added["ready_for_approval"], true);
    assert_eq!(added["allowances"]["totals"]["daily_allowances"], "54.00");
    assert_eq!(added["trips"][0]["workplace"], "special_work_place");
    assert_eq!(
        added["path"],
        "intake/expense-claims/maija-2026-03-10/draft.json"
    );

    let receipt = attach(home, "Pysäköintikuitti, Esimerkkiparkki, 12,40 €");
    let with_line = ok(
        home,
        &["claim", "line", "add", "--claim", "maija-2026-03-10"],
        &line(&receipt, "2026-03-10", "12.40", Some("t1")),
    );
    assert_eq!(with_line["revision"], 2);
    assert_eq!(with_line["totals"]["receipts"], "12.40");

    let shown = ok(
        home,
        &["claim", "show", "--claim", "maija-2026-03-10"],
        &json!(null),
    );
    assert_eq!(shown["revision_hash"], with_line["revision_hash"]);
    let markdown = ok(
        home,
        &[
            "claim",
            "show",
            "--claim",
            "maija-2026-03-10",
            "--format",
            "markdown",
        ],
        &json!(null),
    )["markdown"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(markdown.starts_with("# Expense claim draft maija-2026-03-10"));
    assert!(markdown.contains("| Receipt lines | 12.40 |"), "{markdown}");
    assert!(markdown.contains("erityinen työntekemispaikka"));

    let listed = ok(home, &["claim", "list"], &json!(null));
    assert_eq!(listed["count"], 1);
    assert_eq!(listed["drafts"][0]["trips"], 1);

    assert_eq!(
        refused(
            home,
            &[
                "claim",
                "discard",
                "--claim",
                "maija-2026-03-10",
                "--reason",
                " "
            ],
            &json!(null)
        ),
        "CLAIM_INPUT_INVALID"
    );
    let discarded = ok(
        home,
        &[
            "claim",
            "discard",
            "--claim",
            "maija-2026-03-10",
            "--reason",
            "Tehty uudelleen",
        ],
        &json!(null),
    );
    assert_eq!(
        discarded["discarded"]["last_revision_hash"],
        with_line["revision_hash"]
    );
    assert!(!home.join("intake/expense-claims/maija-2026-03-10").exists());
    assert_eq!(ok(home, &["claim", "list"], &json!(null))["count"], 0);
    let log = std::fs::read_to_string(home.join("intake/expense-claims/discarded.jsonl")).unwrap();
    assert!(log.contains("Tehty uudelleen"));
    assert!(
        !log.contains("Asiakastapaaminen"),
        "the log keeps no content"
    );
}

#[test]
fn a_monthly_claim_collects_trips_over_the_month_with_open_questions() {
    let dir = claimant_home();
    let home = dir.path();
    // The first trip is recorded with its meals unanswered.
    let mut first = trip("2026-03-03");
    first["trip"]["days"] = json!([]);
    let started = ok(
        home,
        &[
            "claim",
            "trip",
            "add",
            "--new-for",
            "maija",
            "--claim-id",
            "maija-2026-03",
        ],
        &first,
    );
    assert_eq!(started["claim_id"], "maija-2026-03");
    assert_eq!(
        codes(&started["open_questions"]),
        [
            "TRAVEL_FREE_MEALS_UNANSWERED",
            "TRAVEL_COMPANY_PAID_MEALS_UNANSWERED"
        ]
    );
    assert_eq!(started["open_questions"][0]["trip_id"], "t1");
    assert_eq!(started["open_questions"][0]["day"], 1);
    assert_eq!(started["ready_for_approval"], false);

    // The answer is a revision of the whole record.
    let answered = ok(
        home,
        &[
            "claim",
            "trip",
            "revise",
            "--claim",
            "maija-2026-03",
            "--trip",
            "t1",
        ],
        &trip("2026-03-03"),
    );
    assert_eq!(answered["open_questions"], json!([]));
    assert_eq!(
        answered["trips"][0]["recorded_at"],
        started["trips"][0]["recorded_at"]
    );
    assert!(answered["trips"][0]["revised_at"].is_string());

    for date in ["2026-03-12", "2026-03-24"] {
        ok(
            home,
            &["claim", "trip", "add", "--claim", "maija-2026-03"],
            &trip(date),
        );
    }
    let receipt = attach(home, "Junalippu Helsinki–Tampere 2026-03-12");
    let full = ok(
        home,
        &[
            "claim",
            "line",
            "add",
            "--claim",
            "maija-2026-03",
            "--line",
            "train",
        ],
        &line(&receipt, "2026-03-12", "38.90", Some("t2")),
    );
    assert_eq!(full["trips"].as_array().unwrap().len(), 3);
    assert_eq!(full["allowances"]["totals"]["daily_allowances"], "162.00");
    assert_eq!(full["ready_for_approval"], true, "{full}");

    // A trip with a receipt is not removed; the line goes first.
    assert_eq!(
        refused(
            home,
            &[
                "claim",
                "trip",
                "remove",
                "--claim",
                "maija-2026-03",
                "--trip",
                "t2"
            ],
            &json!(null)
        ),
        "CLAIM_TRIP_HAS_LINES"
    );
    ok(
        home,
        &[
            "claim",
            "line",
            "remove",
            "--claim",
            "maija-2026-03",
            "--line",
            "train",
        ],
        &json!(null),
    );
    let removed = ok(
        home,
        &[
            "claim",
            "trip",
            "remove",
            "--claim",
            "maija-2026-03",
            "--trip",
            "t2",
        ],
        &json!(null),
    );
    assert_eq!(removed["trips"].as_array().unwrap().len(), 2);
    assert_eq!(removed["revision"], 7);

    // A second draft overlapping a trip of the first is refused at approval,
    // and the same receipt twice is refused.
    let other = ok(
        home,
        &["claim", "trip", "add", "--new-for", "maija"],
        &trip("2026-03-24"),
    );
    assert_eq!(other["claim_id"], "maija-2026-03-24");
    assert_eq!(codes(&other["refusals"]), ["TRIP_OVERLAP"]);
    ok(
        home,
        &["claim", "line", "add", "--claim", "maija-2026-03"],
        &line(&receipt, "2026-03-24", "38.90", None),
    );
    let dup = ok(
        home,
        &["claim", "line", "add", "--claim", "maija-2026-03-24"],
        &line(&receipt, "2026-03-24", "38.90", None),
    );
    assert_eq!(
        codes(&dup["refusals"]),
        ["TRIP_OVERLAP", "RECEIPT_DUPLICATE"]
    );
}

#[test]
fn writes_refuse_what_does_not_validate_and_write_nothing() {
    let dir = claimant_home();
    let home = dir.path();
    let mut backwards = trip("2026-03-10");
    backwards["trip"]["end"] = json!("2026-03-10T06:00:00+02:00");
    assert_eq!(
        refused(
            home,
            &["claim", "trip", "add", "--new-for", "maija"],
            &backwards
        ),
        "TRAVEL_INPUT_INVALID"
    );
    assert!(
        !home.join("intake/expense-claims").exists(),
        "nothing written"
    );
    let mut unknown = trip("2026-03-10");
    unknown["mileage_paid"] = json!("100.00");
    assert_eq!(
        refused(
            home,
            &["claim", "trip", "add", "--new-for", "maija"],
            &unknown
        ),
        "INVALID_INPUT"
    );
    assert_eq!(
        refused(
            home,
            &["claim", "trip", "add", "--new-for", "pekka"],
            &trip("2026-03-10")
        ),
        "EMPLOYEE_NOT_FOUND"
    );
    assert_eq!(
        refused(
            home,
            &["claim", "trip", "add", "--claim", "nope"],
            &trip("2026-03-10")
        ),
        "CLAIM_DRAFT_NOT_FOUND"
    );
    assert_eq!(
        refused(home, &["claim", "show", "--claim", "../x"], &json!(null)),
        "CLAIM_ID_INVALID"
    );
    let (code, _, _) = run(
        home,
        &["claim", "trip", "add", "--claim", "a", "--new-for", "maija"],
        &trip("2026-03-10").to_string(),
    );
    assert_eq!(code, 1, "--claim and --new-for exclude each other");
    let mut missing_artifact = line(&"ab".repeat(32), "2026-03-10", "5.00", None);
    missing_artifact["description"] = json!("Taksi");
    assert_eq!(
        refused(
            home,
            &["claim", "line", "add", "--new-for", "maija"],
            &missing_artifact
        ),
        "ARTIFACT_NOT_FOUND"
    );
    assert!(
        !home.join("intake/expense-claims").exists(),
        "nothing written"
    );
}

#[test]
fn a_hand_edited_draft_that_fails_validation_is_refused_on_read() {
    let dir = claimant_home();
    let home = dir.path();
    ok(
        home,
        &["claim", "trip", "add", "--new-for", "maija"],
        &trip("2026-03-10"),
    );
    let file = home.join("intake/expense-claims/maija-2026-03-10/draft.json");
    let original = std::fs::read_to_string(&file).unwrap();
    for edited in [
        original.replace("\"kind\": \"domestic\"", "\"kind\": \"cruise\""),
        original.replace(
            "\"source\": \"user_statement\"",
            "\"source\": \"user_statement\", \"paid_total\": \"999\"",
        ),
        original.replace("\"revision\": 1", "\"revision\": 0"),
        original.replace("2026-03-10T19:15:00+02:00", "2026-03-10T06:15:00+02:00"),
        "{".to_string(),
    ] {
        assert_ne!(edited, original);
        std::fs::write(&file, &edited).unwrap();
        assert_eq!(
            refused(
                home,
                &["claim", "show", "--claim", "maija-2026-03-10"],
                &json!(null)
            ),
            "CLAIM_DRAFT_INVALID"
        );
        assert_eq!(
            refused(
                home,
                &["claim", "trip", "add", "--claim", "maija-2026-03-10"],
                &trip("2026-03-20")
            ),
            "CLAIM_DRAFT_INVALID"
        );
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            edited,
            "not rewritten"
        );
        let listed = ok(home, &["claim", "list"], &json!(null));
        assert!(listed["drafts"][0]["invalid"]
            .as_str()
            .unwrap()
            .contains("CLAIM_DRAFT_INVALID"));
    }
    // Another draft of the claimant says it could not compare with it.
    let other = ok(
        home,
        &["claim", "trip", "add", "--new-for", "maija"],
        &trip("2026-03-20"),
    );
    assert_eq!(
        codes(&other["checks"]),
        ["TRIP_RECORDED_AFTERWARDS", "CLAIM_DRAFT_UNREADABLE"]
    );
    // An invalid draft can still be discarded.
    ok(
        home,
        &[
            "claim",
            "discard",
            "--claim",
            "maija-2026-03-10",
            "--reason",
            "Muokattu käsin",
        ],
        &json!(null),
    );
}

fn git(home: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(home)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn no_draft_content_reaches_git_even_when_the_root_gitignore_predates_the_directory() {
    let dir = claimant_home();
    let home = dir.path();
    // A root .gitignore written before claim drafts existed: it knows
    // nothing about intake/.
    std::fs::write(
        home.join(".gitignore"),
        "/fiscal.sqlite\n/fiscal.sqlite-*\n/artifacts/\n/masterdata/pii.sqlite\n/masterdata/pii.sqlite-*\n",
    )
    .unwrap();
    let root_before = std::fs::read(home.join(".gitignore")).unwrap();
    assert!(git(home, &["init", "-q"]).status.success());
    ok(
        home,
        &["claim", "trip", "add", "--new-for", "maija"],
        &trip("2026-03-10"),
    );
    let receipt = attach(home, "Hotellilasku");
    ok(
        home,
        &["claim", "line", "add", "--claim", "maija-2026-03-10"],
        &line(&receipt, "2026-03-10", "99.00", Some("t1")),
    );
    ok(
        home,
        &[
            "claim",
            "discard",
            "--claim",
            "maija-2026-03-10",
            "--reason",
            "Testi",
        ],
        &json!(null),
    );
    ok(
        home,
        &["claim", "trip", "add", "--new-for", "maija"],
        &trip("2026-03-11"),
    );
    assert_eq!(std::fs::read(home.join(".gitignore")).unwrap(), root_before);
    let status = git(
        home,
        &[
            "status",
            "--porcelain",
            "--untracked-files=all",
            "--ignored=no",
        ],
    );
    let status = String::from_utf8(status.stdout).unwrap();
    assert!(
        !status.contains("intake/"),
        "draft files are visible to git:\n{status}"
    );
    for path in [
        "intake/expense-claims/maija-2026-03-11/draft.json",
        "intake/expense-claims/discarded.jsonl",
        "intake/expense-claims/.gitignore",
    ] {
        assert!(home.join(path).exists(), "{path}");
        assert!(
            git(home, &["check-ignore", "-q", path]).status.success(),
            "{path} not ignored"
        );
    }

    // Operator rules in the directory's own .gitignore are kept, and the
    // ignore-everything rule is put back last.
    let own = home.join("intake/expense-claims/.gitignore");
    std::fs::write(&own, "# operator\n!keep.txt").unwrap();
    ok(
        home,
        &["claim", "trip", "add", "--claim", "maija-2026-03-11"],
        &trip("2026-03-12"),
    );
    let rules = std::fs::read_to_string(&own).unwrap();
    assert!(rules.starts_with("# operator\n!keep.txt\n"), "{rules}");
    assert!(rules.trim_end().ends_with("\n*"), "{rules}");
    assert!(git(
        home,
        &[
            "check-ignore",
            "-q",
            "intake/expense-claims/maija-2026-03-11/draft.json"
        ]
    )
    .status
    .success());
}

#[test]
fn a_receipt_missing_from_the_artifact_store_is_refused_without_blocking_other_writes() {
    let dir = claimant_home();
    let home = dir.path();
    ok(
        home,
        &["claim", "trip", "add", "--new-for", "maija"],
        &trip("2026-03-10"),
    );
    let receipt = attach(home, "Pysäköintikuitti");
    ok(
        home,
        &["claim", "line", "add", "--claim", "maija-2026-03-10"],
        &line(&receipt, "2026-03-10", "12.40", Some("t1")),
    );
    // A home restored without its attachment files.
    std::fs::remove_dir_all(home.join("artifacts")).unwrap();
    std::fs::create_dir(home.join("artifacts")).unwrap();
    let shown = ok(
        home,
        &["claim", "show", "--claim", "maija-2026-03-10"],
        &json!(null),
    );
    assert_eq!(codes(&shown["refusals"]), ["ARTIFACT_MISSING"]);
    assert_eq!(shown["refusals"][0]["line_id"], "l1");
    assert_eq!(shown["ready_for_approval"], false);
    let other = ok(
        home,
        &["claim", "trip", "add", "--claim", "maija-2026-03-10"],
        &trip("2026-03-11"),
    );
    assert_eq!(codes(&other["refusals"]), ["ARTIFACT_MISSING"]);
}
