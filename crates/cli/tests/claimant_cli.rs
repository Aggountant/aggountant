//! `aggountant claimant`: the claimant register for expense claims, and the
//! guarantee that a company without claims sees nothing of it.

mod common;

use std::path::{Path, PathBuf};
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
    assert_eq!(error["error_kind"], "user");
    error["error_code"].as_str().unwrap().to_string()
}

fn add_employee(home: &Path) {
    ok(
        home,
        &["employee", "add"],
        &json!({
            "employee_id": "maija",
            "name": "Maija Meikäläinen",
            "hetu": "131052-308T",
            "iban": "FI2112345600000785",
            "tax_card": {"withholding_pct": "20.00"}
        }),
    );
}

fn place(id: &str, kind: &str, location: &str, municipality: &str) -> Value {
    json!({
        "employee_id": "maija",
        "place_id": id,
        "kind": kind,
        "label": id,
        "location": location,
        "municipality": municipality,
        "confirmed_by": "Maija Meikäläinen"
    })
}

fn fact(from: &str, to: Option<&str>, fact: Value) -> Value {
    json!({
        "employee_id": "maija",
        "valid_from": from,
        "valid_to": to,
        "fact": fact,
        "confirmed_by": "Maija Meikäläinen"
    })
}

fn trip(date: &str, start: &str, destinations: &[&str]) -> Value {
    json!({
        "employee_id": "maija",
        "date": date,
        "start_place_id": start,
        "destination_place_ids": destinations
    })
}

#[test]
fn the_register_records_corrects_and_refuses() {
    let home = common::initialized_home();
    let home = home.path();
    add_employee(home);
    for p in [
        place("home", "home", "Esimerkkikatu 7, 33100 Tampere", "Tampere"),
        place(
            "office",
            "company_premises",
            "Toimistotie 1, 33100 Tampere",
            "Tampere",
        ),
        place(
            "branch",
            "company_premises",
            "Haaratie 2, 90100 Oulu",
            "Oulu",
        ),
        place(
            "customer",
            "customer_site",
            "Asiakaskuja 3, 20100 Turku",
            "Turku",
        ),
    ] {
        ok(home, &["claimant", "place", "add"], &p);
    }
    // A place is never recorded twice.
    assert_eq!(
        refused(
            home,
            &["claimant", "place", "add"],
            &place("home", "home", "Toinen katu 1", "Tampere")
        ),
        "PLACE_ALREADY_RECORDED"
    );

    // Without a relationship, then without a workplace entry, a trip refuses.
    let to_customer = trip("2025-03-04", "home", &["customer"]);
    assert_eq!(
        refused(home, &["claimant", "snapshot"], &to_customer),
        "CLAIMANT_RELATIONSHIP_UNKNOWN"
    );
    assert_eq!(
        refused(
            home,
            &["claimant", "add"],
            &fact(
                "2024-01-01",
                None,
                json!({"kind": "relationship", "relationship": "contractor"})
            )
        ),
        "CONTRACTOR_NOT_CLAIMANT"
    );
    ok(
        home,
        &["claimant", "add"],
        &fact(
            "2024-01-01",
            None,
            json!({"kind": "relationship", "relationship": "managing_director"}),
        ),
    );
    ok(
        home,
        &["claimant", "add"],
        &fact(
            "2024-01-01",
            None,
            json!({"kind": "part_owner", "part_owner": true}),
        ),
    );
    assert_eq!(
        refused(home, &["claimant", "snapshot"], &to_customer),
        "REGULAR_WORKPLACE_UNKNOWN"
    );

    // The office is the regular workplace from 2024.
    let office = ok(
        home,
        &["claimant", "add"],
        &fact(
            "2024-01-01",
            None,
            json!({"kind": "workplace", "role": "regular_primary", "place_id": "office",
                   "basis": "employer_premises_weekly"}),
        ),
    );
    let office_id = office["fact"]["fact_id"].as_i64().unwrap();
    let known_before_correction = office["fact"]["recorded_at"].as_str().unwrap().to_string();
    let snapshot = ok(home, &["claimant", "snapshot"], &to_customer);
    assert_eq!(snapshot["scope"], "full");
    assert_eq!(
        snapshot["snapshot"]["destinations"][0]["classification"],
        "special_work_site"
    );
    assert_eq!(
        snapshot["snapshot"]["register"]["part_owner"]["value"],
        true
    );
    assert_eq!(
        refused(
            home,
            &["claimant", "snapshot"],
            &trip("2025-03-04", "home", &["office"])
        ),
        "TRIP_TO_REGULAR_WORKPLACE"
    );

    // Oulu is a secondary workplace: travel and lodging only.
    ok(
        home,
        &["claimant", "add"],
        &fact(
            "2024-01-01",
            None,
            json!({"kind": "workplace", "role": "secondary", "place_id": "branch",
                   "basis": "premises_in_another_municipality"}),
        ),
    );
    let snapshot = ok(
        home,
        &["claimant", "snapshot"],
        &trip("2025-03-04", "office", &["branch"]),
    );
    assert_eq!(snapshot["scope"], "travel_and_lodging_only");
    // The home cannot be a secondary workplace.
    assert_eq!(
        refused(
            home,
            &["claimant", "add"],
            &fact(
                "2024-01-01",
                None,
                json!({"kind": "workplace", "role": "secondary", "place_id": "home",
                       "basis": "other", "basis_explanation": "Fridays at home"}),
            )
        ),
        "SECONDARY_WORKPLACE_INVALID"
    );

    // Backward correction: the office ended 2024-12-31 and the company's
    // premises have been in the home since 2025-01-01.
    std::thread::sleep(std::time::Duration::from_millis(5));
    ok(
        home,
        &["claimant", "correct"],
        &json!({
            "employee_id": "maija",
            "fact_id": office_id,
            "reason": "The office lease ended at the turn of the year.",
            "confirmed_by": "Maija Meikäläinen",
            "replacement": {
                "valid_from": "2024-01-01",
                "valid_to": "2024-12-31",
                "fact": {"kind": "workplace", "role": "regular_primary",
                         "place_id": "office", "basis": "employer_premises_weekly"}
            }
        }),
    );
    ok(
        home,
        &["claimant", "add"],
        &fact(
            "2025-01-01",
            None,
            json!({"kind": "workplace", "role": "regular_primary", "place_id": "home",
                   "basis": "company_premises_in_home"}),
        ),
    );
    // The same row is corrected only once.
    assert_eq!(
        refused(
            home,
            &["claimant", "correct"],
            &json!({"employee_id": "maija", "fact_id": office_id, "reason": "again",
                    "confirmed_by": "Maija Meikäläinen", "retract": true})
        ),
        "CLAIMANT_FACT_SUPERSEDED"
    );

    let show = |extra: &[&str]| {
        let mut args = vec![
            "claimant",
            "show",
            "--employee",
            "maija",
            "--date",
            "2025-03-04",
        ];
        args.extend_from_slice(extra);
        ok(home, &args, &json!({}))
    };
    let now = show(&[]);
    let then = show(&["--as-known-at", &known_before_correction]);
    let primary = |view: &Value| {
        view["workplaces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|w| w["role"] == "regular_primary")
            .map(|w| w["place"]["place_id"].clone())
    };
    assert_eq!(primary(&now), Some(json!("home")));
    assert_eq!(primary(&then), Some(json!("office")));
    assert_ne!(now["facts_digest"], then["facts_digest"]);

    // As known now, a trip to the old office is a trip to a special work site.
    let snapshot = ok(
        home,
        &["claimant", "snapshot"],
        &trip("2025-03-04", "home", &["office"]),
    );
    assert_eq!(snapshot["scope"], "full");

    let history = ok(
        home,
        &["claimant", "history", "--employee", "maija"],
        &json!({}),
    );
    assert_eq!(history["places"].as_array().unwrap().len(), 4);
    assert_eq!(history["facts"].as_array().unwrap().len(), 6);

    // Strict input and unknown people.
    let (code, _, stderr) = run(
        home,
        &["claimant", "add"],
        r#"{"employee_id":"maija","valid_from":"2025-01-01","confirmed_by":"x","note":1,
            "fact":{"kind":"part_owner","part_owner":false}}"#,
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("INVALID_INPUT"), "{stderr}");
    let (code, _, stderr) = run(home, &["claimant", "history", "--employee", "ghost"], "");
    assert_eq!(code, 1);
    assert!(stderr.contains("EMPLOYEE_NOT_FOUND"), "{stderr}");

    // The register is personal data: nothing of it outside the PII store.
    for file in files(home) {
        let rel = file
            .strip_prefix(home)
            .unwrap()
            .to_string_lossy()
            .to_string();
        if rel.starts_with("masterdata/pii.sqlite") {
            continue;
        }
        let bytes = std::fs::read(&file).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains("Esimerkkikatu 7"),
            "address leaked into {rel}"
        );
    }
    let gitignore = std::fs::read_to_string(home.join(".gitignore")).unwrap();
    assert!(gitignore.contains("/masterdata/pii.sqlite"));
}

/// Every file under `dir`, recursively, in sorted order.
fn files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in std::fs::read_dir(&path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn copy_dir(from: &Path, to: &Path) {
    for file in files(from) {
        let target = to.join(file.strip_prefix(from).unwrap());
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(&file, &target).unwrap();
    }
}

/// The SQLite stores (and their WAL files), which a command may touch.
fn is_store(path: &Path) -> bool {
    let name = path.to_string_lossy();
    name.contains("pii.sqlite") || name.contains("fiscal.sqlite")
}

/// The personal-data store's v1 rows and table definitions, as text.
fn pii_v1_dump(home: &Path) -> Vec<String> {
    let conn = rusqlite::Connection::open(home.join("masterdata/pii.sqlite")).unwrap();
    let mut out = Vec::new();
    for sql in [
        "SELECT employee_id || '|' || hex(sealed) FROM employee ORDER BY employee_id",
        "SELECT employee_id || '|' || tax_year || '|' || natural_key || '|' || gross_amount \
         FROM ytd_earnings ORDER BY 1",
        "SELECT name || '|' || sql FROM sqlite_master \
         WHERE name IN ('employee', 'ytd_earnings') ORDER BY name",
    ] {
        let mut stmt = conn.prepare(sql).unwrap();
        out.extend(
            stmt.query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .map(Result::unwrap),
        );
    }
    out
}

fn pii_version(home: &Path) -> i64 {
    let conn = rusqlite::Connection::open(home.join("masterdata/pii.sqlite")).unwrap();
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap()
}

/// Rewind the personal-data store to v1, the schema of every home created
/// before the claimant register: drop what v2 added and restamp the version.
fn rewind_pii_to_v1(home: &Path) {
    let conn = rusqlite::Connection::open(home.join("masterdata/pii.sqlite")).unwrap();
    conn.execute_batch(
        "DROP TRIGGER claimant_place_append_only;
         DROP TRIGGER claimant_fact_append_only;
         DROP INDEX claimant_fact_by_employee;
         DROP TABLE claimant_fact;
         DROP TABLE claimant_place;
         PRAGMA user_version = 1;
         PRAGMA wal_checkpoint(TRUNCATE);",
    )
    .unwrap();
}

/// The existing commands whose output must not change.
const READ_COMMANDS: &[&[&str]] = &[
    &["chart", "list"],
    &[
        "voucher",
        "list",
        "--posting-from",
        "2026-01-01",
        "--posting-to",
        "2026-12-31",
    ],
    &["voucher", "show", "1"],
    &["voucher", "verify"],
    &["period", "show", "--period", "2026-03"],
    &["employee", "list"],
    &["employee", "show", "--employee", "maija"],
    &["knowledge", "company", "list"],
];

fn read_outputs(home: &Path) -> Vec<(i32, String, String)> {
    READ_COMMANDS
        .iter()
        .map(|args| run(home, args, ""))
        .collect()
}

/// Product owner rule for every claim slice: a company that does not use
/// expense claims sees no questions, no output changes and no slowdown. A
/// home from before the register (pii schema v1) is copied; the copy is
/// opened by this binary, which migrates it, and existing commands answer
/// exactly as before, while the v1 data stays byte for byte the same.
#[test]
fn a_home_without_claims_upgrades_with_unchanged_output() {
    let original = common::initialized_home();
    let original = original.path();
    add_employee(original);
    let draft = json!({
        "document_date": "2026-03-14", "posting_date": "2026-03-14",
        "description": "Hosting invoice",
        "postings": [
            {"account": "7690", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "debit"},
            {"account": "1910", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "credit"}
        ]
    });
    ok(
        original,
        &["voucher", "seal"],
        &json!({"draft": draft, "natural_key": "esimerkki:1", "actor": "tester"}),
    );
    let before = read_outputs(original);
    for (args, (code, _, stderr)) in READ_COMMANDS.iter().zip(&before) {
        assert_eq!(*code, 0, "{args:?}: {stderr}");
    }

    rewind_pii_to_v1(original);
    assert_eq!(pii_version(original), 1);
    let v1_rows = pii_v1_dump(original);
    let copy = tempfile::tempdir().unwrap();
    let copy = copy.path();
    copy_dir(original, copy);
    let files_before: Vec<(PathBuf, Vec<u8>)> = files(copy)
        .into_iter()
        .filter(|f| !is_store(f))
        .map(|f| {
            let bytes = std::fs::read(&f).unwrap();
            (f, bytes)
        })
        .collect();

    let after = read_outputs(copy);
    assert_eq!(
        after, before,
        "existing commands answer the same after the upgrade"
    );
    for (_, stdout, _) in &after {
        let lower = stdout.to_lowercase();
        assert!(!lower.contains("claimant") && !lower.contains("workplace"));
    }
    assert_eq!(pii_version(copy), 2);
    assert_eq!(
        pii_v1_dump(copy),
        v1_rows,
        "v1 data unchanged by the migration"
    );
    let conn = rusqlite::Connection::open(copy.join("masterdata/pii.sqlite")).unwrap();
    for table in ["claimant_place", "claimant_fact"] {
        let n: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "{table} is empty in a home without claims");
    }
    // No file outside the stores was added or changed.
    let files_after: Vec<(PathBuf, Vec<u8>)> = files(copy)
        .into_iter()
        .filter(|f| !is_store(f))
        .map(|f| {
            let bytes = std::fs::read(&f).unwrap();
            (f, bytes)
        })
        .collect();
    assert_eq!(files_after, files_before);
}
