//! S2 done-criterion: the thin `close-period` Run runs **end to end via the CLI**
//! (Open → Closing → ApproveClose → Sealed), and the ADR-6 gate holds through the
//! binary boundary — a bare `voucher seal` into the Closing period is refused
//! `PERIOD_CLOSING` with exit code 1.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
}

fn run(home: &std::path::Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut cmd = bin();
    cmd.arg("--home").arg(home).arg("--json");
    cmd.args(args);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn aggountant");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

const SEAL_INPUT: &str = r#"{
  "draft": {
    "document_date": "2026-02-10",
    "posting_date": "2026-02-10",
    "description": "Hosting",
    "postings": [
      { "account": "7690", "amount": { "amount": "100.00", "currency": "EUR" }, "side": "debit" },
      { "account": "1910", "amount": { "amount": "100.00", "currency": "EUR" }, "side": "credit" }
    ]
  },
  "natural_key": "co:receipt-1"
}"#;

#[test]
fn close_period_run_end_to_end_via_cli() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();

    assert_eq!(run(home, &["init"], "").0, 0);

    // A voucher seals into the Open period.
    let (code, _, err) = run(home, &["voucher", "seal"], SEAL_INPUT);
    assert_eq!(code, 0, "seal into Open period must succeed: {err}");

    // Start the close-period Run: it transitions Open→Closing and parks on the gate.
    let (code, stdout, _) = run(
        home,
        &["run", "start", "close-period", "--period", "2026-Q1"],
        "",
    );
    assert_eq!(code, 0);
    let started: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(started["status"], "awaiting_gate");
    let run_id = started["run_id"].as_str().unwrap().to_string();

    // period show → Closing.
    let (code, stdout, _) = run(home, &["period", "show", "--period", "2026-Q1"], "");
    assert_eq!(code, 0);
    let view: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(view["state"], "closing");
    assert_eq!(view["owning_run_id"], run_id);

    // The ADR-6 gate through the CLI: a bare seal into the Closing period is refused.
    let (code, _, stderr) = run(
        home,
        &["voucher", "seal"],
        &SEAL_INPUT.replace("receipt-1", "receipt-2"),
    );
    assert_eq!(code, 1, "a user error exits 1");
    let e: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(e["error_code"], "PERIOD_CLOSING");

    // run show reflects the derived state.
    let (code, stdout, _) = run(home, &["run", "show", "--run-id", &run_id], "");
    assert_eq!(code, 0);
    let shown: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(shown["status"], "awaiting_gate");

    // Answer the gate → SealPeriod → Sealed.
    let (code, stdout, _) = run(
        home,
        &[
            "run",
            "answer",
            "--run-id",
            &run_id,
            "--gate",
            "ApproveClose",
            "--answer",
            "approve",
        ],
        "",
    );
    assert_eq!(code, 0);
    let done: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(done["status"], "completed");

    // period show → Sealed; chain verifies.
    let (code, stdout, _) = run(home, &["period", "show", "--period", "2026-Q1"], "");
    assert_eq!(code, 0);
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["state"],
        "sealed"
    );

    let (code, stdout, _) = run(home, &["voucher", "verify"], "");
    assert_eq!(code, 0);
    let report: Value = serde_json::from_str(&stdout).unwrap();
    assert!(report["ok"].as_bool().unwrap());
    assert_eq!(report["period_count"], 1);

    // run list shows the run.
    let (code, stdout, _) = run(home, &["run", "list"], "");
    assert_eq!(code, 0);
    let list: Value = serde_json::from_str(&stdout).unwrap();
    assert!(list["runs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r == &run_id));
}

#[test]
fn run_migrate_requires_confirm() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let (_, stdout, _) = run(
        home,
        &["run", "start", "close-period", "--period", "2026-Q1"],
        "",
    );
    let run_id = serde_json::from_str::<Value>(&stdout).unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (code, _, stderr) = run(home, &["run", "migrate", "--run-id", &run_id], "");
    assert_eq!(code, 1);
    assert_eq!(
        serde_json::from_str::<Value>(&stderr).unwrap()["error_code"],
        "CONFIRM_REQUIRED"
    );
}

#[test]
fn cli_closes_july_without_locking_august_and_rejects_loose_month_syntax() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);

    let july_input = SEAL_INPUT
        .replace("2026-02-10", "2026-07-31")
        .replace("receipt-1", "july-receipt");
    assert_eq!(run(home, &["voucher", "seal"], &july_input).0, 0);
    let (code, stdout, error) = run(
        home,
        &["run", "start", "close-period", "--period", "2026-07"],
        "",
    );
    assert_eq!(code, 0, "{error}");
    let run_id = serde_json::from_str::<Value>(&stdout).unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        run(
            home,
            &[
                "run",
                "answer",
                "--run-id",
                &run_id,
                "--gate",
                "ApproveClose",
                "--answer",
                "approve",
            ],
            "",
        )
        .0,
        0
    );

    let august_input = SEAL_INPUT
        .replace("2026-02-10", "2026-08-01")
        .replace("receipt-1", "august-receipt");
    assert_eq!(run(home, &["voucher", "seal"], &august_input).0, 0);

    let (code, _, error) = run(home, &["period", "show", "--period", "2026-7"], "");
    assert_eq!(code, 1);
    assert_eq!(
        serde_json::from_str::<Value>(&error).unwrap()["error_code"],
        "INVALID_PERIOD_ID"
    );
}
