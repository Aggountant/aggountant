//! `close-year answer` never assumes an answer: approving `ApproveStatements`
//! seals the fiscal year irreversibly, so the approval cannot be given by
//! omitting `--answer`. Every close-year command says what comes next, and its
//! help is written in plain terms.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use aggountant_ledger::Ledger;
use aggountant_model::PeriodId;

fn invoke(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(args)
        .env_remove("AGGOUNTANT_HOME")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn answer_without_an_explicit_answer_is_a_usage_error() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path().to_str().unwrap();
    assert_eq!(invoke(&["--home", home, "init"]).status.code(), Some(0));

    let output = invoke(&[
        "--home",
        home,
        "close-year",
        "answer",
        "--run-id",
        "cy-does-not-exist",
        "--gate",
        "ApproveStatements",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--answer"), "stderr: {stderr}");
    // Refused by the parser, before any run is looked up.
    assert!(!stderr.contains("RUN_NOT_FOUND"), "stderr: {stderr}");
}

#[test]
fn answer_help_names_the_gates_and_the_permanent_approvals() {
    let output = invoke(&["close-year", "answer", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = String::from_utf8_lossy(&output.stdout);
    for gate in [
        "ApproveAdjustments",
        "ApproveTaseErittelyt",
        "ApproveLiitetiedot",
        "ApproveStatements",
    ] {
        assert!(help.contains(gate), "help does not name {gate}:\n{help}");
    }
    assert!(help.contains("seals the fiscal"), "{help}");
    assert!(help.contains("no default"), "{help}");
    assert!(!help.contains("[default: approve]"), "{help}");
    for flag in ["--run-id <RUN_ID>", "--gate <GATE>", "--answer <ANSWER>"] {
        let line = help
            .lines()
            .find(|line| line.trim_start().starts_with(flag))
            .unwrap_or_else(|| panic!("no {flag} in help:\n{help}"));
        assert!(
            line.trim() != flag,
            "{flag} has no help text on its line:\n{help}"
        );
    }
}

#[test]
fn help_uses_plain_descriptions_not_design_labels() {
    for args in [
        &["close-year", "--help"][..],
        &["close-year", "start", "--help"],
        &["close-year", "answer", "--help"],
        &["close-year", "seal-correction", "--help"],
        &["close-year", "resume", "--help"],
        &["close-year", "show", "--help"],
    ] {
        let output = invoke(args);
        assert_eq!(output.status.code(), Some(0));
        let help = String::from_utf8_lossy(&output.stdout);
        for label in ["RF6", "S6", "D7", "SealYear", "reconcil"] {
            assert!(!help.contains(label), "{args:?} help uses {label}:\n{help}");
        }
    }
    let help =
        String::from_utf8_lossy(&invoke(&["close-year", "seal-correction", "--help"]).stdout)
            .to_string();
    assert!(help.contains("reclassification"), "{help}");
}

#[test]
fn start_and_answer_name_the_open_gate_and_the_next_command() {
    let home = tempfile::tempdir().unwrap();
    let home_str = home.path().to_str().unwrap();
    assert_eq!(invoke(&["--home", home_str, "init"]).status.code(), Some(0));
    let ledger = Ledger::open(home.path()).unwrap();
    for q in 1..=4 {
        let period = PeriodId::new(2026, q).unwrap();
        let auth = ledger.begin_close(&period, "cp-test", "test").unwrap();
        ledger.seal_period(&period, &auth, "test").unwrap();
    }

    let mut child = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(["--home", home_str, "--json", "close-year", "start"])
        .env_remove("AGGOUNTANT_HOME")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"year": 2026}"#)
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let started: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let run_id = started["run_id"].as_str().unwrap().to_string();
    assert_eq!(
        started["next_command"],
        format!("aggountant close-year show --run-id {run_id}")
    );
    let message = started["message"].as_str().unwrap();
    assert!(
        message.contains("Open gate: ApproveAdjustments"),
        "{message}"
    );

    let shown: serde_json::Value = serde_json::from_slice(
        &invoke(&[
            "--home",
            home_str,
            "--json",
            "close-year",
            "show",
            "--run-id",
            &run_id,
        ])
        .stdout,
    )
    .unwrap();
    assert_eq!(
        shown["next_command"],
        format!(
            "aggountant close-year answer --run-id {run_id} --gate ApproveAdjustments --answer '<approve|reject>'"
        )
    );

    let answered: serde_json::Value = serde_json::from_slice(
        &invoke(&[
            "--home",
            home_str,
            "--json",
            "close-year",
            "answer",
            "--run-id",
            &run_id,
            "--gate",
            "ApproveAdjustments",
            "--answer",
            "reject",
        ])
        .stdout,
    )
    .unwrap();
    assert_eq!(answered["status"], "rejected");
    assert!(answered["next_command"].is_null());
    assert!(answered["message"].as_str().unwrap().contains("rejected"));
}

fn invoke_with_stdin(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(args)
        .env_remove("AGGOUNTANT_HOME")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn start_dry_run_shows_rates_and_depreciation_without_starting_a_run() {
    let home_dir = tempfile::tempdir().unwrap();
    let home = home_dir.path().to_str().unwrap();
    assert_eq!(invoke(&["--home", home, "init"]).status.code(), Some(0));
    let input = r#"{"year": 2026, "assets": [{
        "asset_id": "machine-1", "asset_class": "koneet-ja-kalusto",
        "expense_account": "6000", "accumulated_account": "1160",
        "acquisition_cost": {"amount": "10000.00", "currency": "EUR"},
        "book_value_start": {"amount": "8000.00", "currency": "EUR"}}]}"#;

    let output = invoke_with_stdin(
        &["--home", home, "--json", "close-year", "start", "--dry-run"],
        input,
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["dry_run"], true);
    // The months of 2026 are open, so a start is blocked; the figures still show.
    assert_eq!(value["would_start"], false);
    assert!(value["blockers"].as_array().is_some_and(|b| !b.is_empty()));
    let line = &value["depreciation"][0];
    assert_eq!(line["method"], "reducing-balance");
    assert_eq!(line["rate_percent"], "25.0");
    assert_eq!(line["amount"]["amount"], "2000.00");
    let asset = &value["asset_register"]["assets"][0];
    assert_eq!(asset["source"], "input");
    assert_eq!(asset["book_value_end"]["amount"], "6000.00");
    assert!(value["opening_register_year"].is_null());
    assert!(value["message"]
        .as_str()
        .unwrap()
        .contains("nothing was written"));

    // No run was created.
    let runs = home_dir.path().join("process/runs");
    assert_eq!(std::fs::read_dir(runs).map_or(0, |d| d.count()), 0);

    // An unknown field is refused in a dry run as it is in a start.
    let output = invoke_with_stdin(
        &["--home", home, "--json", "close-year", "start", "--dry-run"],
        r#"{"year": 2026, "asset": []}"#,
    );
    assert_ne!(output.status.code(), Some(0));
}

#[test]
fn start_help_explains_the_register_and_the_dry_run() {
    let output = invoke(&["close-year", "start", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = String::from_utf8_lossy(&output.stdout);
    for needle in [
        "--dry-run",
        "removed_assets",
        "THE ASSET REGISTER",
        "opening_register_year",
    ] {
        assert!(
            help.contains(needle),
            "help does not mention {needle}:\n{help}"
        );
    }
}
