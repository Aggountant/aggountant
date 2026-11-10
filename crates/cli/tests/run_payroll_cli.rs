//! S4 done-criterion: the `run-payroll` Run runs **end to end via the CLI**,
//! including the partial-failure path (one employee's payment times out, the rest
//! succeed, `NeedsAttention` surfaces, `handle-payment-failure` recovers). The
//! authority transport is supplied through `$AGGOUNTANT_BROKER_SCRIPT` (v1 ships no live
//! adapter). Also exercises the F7 `employee` master-data commands.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
}

fn run(
    home: &std::path::Path,
    script: Option<&std::path::Path>,
    args: &[&str],
    stdin: &str,
) -> (i32, String, String) {
    let mut cmd = bin();
    cmd.arg("--home").arg(home).arg("--json");
    if let Some(s) = script {
        cmd.env("AGGOUNTANT_BROKER_SCRIPT", s);
    }
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

fn employee_json(id: &str) -> String {
    format!(
        r#"{{
          "employee_id": "{id}",
          "name": "Test {id}",
          "hetu": "131052-308T",
          "iban": "FI2112345600000785",
          "tax_card": {{ "withholding_pct": "20.00" }}
        }}"#
    )
}

const INTENT: &str = r#"{
  "payment_date": "2026-02-27",
  "employees": [
    {"employee_id": "emp-1", "gross": {"amount": "3000.00", "currency": "EUR"}, "tulolaji": "201"},
    {"employee_id": "emp-2", "gross": {"amount": "3500.00", "currency": "EUR"}, "tulolaji": "201"}
  ]
}"#;

#[test]
fn run_payroll_end_to_end_with_partial_failure_via_cli() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, None, &["init"], "").0, 0);

    // F7 master-data: add two employees.
    for id in ["emp-1", "emp-2"] {
        let (code, _, err) = run(home, None, &["employee", "add"], &employee_json(id));
        assert_eq!(code, 0, "{err}");
    }
    let (code, stdout, _) = run(home, None, &["employee", "list"], "");
    assert_eq!(code, 0);
    let listed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(listed["employees"].as_array().unwrap().len(), 2);

    // Start → compute the batch, park on ApproveRun.
    let (code, stdout, err) = run(home, None, &["run-payroll", "start"], INTENT);
    assert_eq!(code, 0, "{err}");
    let started: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(started["status"], "awaiting_gate");
    let run_id = started["run_id"].as_str().unwrap().to_string();

    // Approve: emp-1 pays; emp-2's payment times out → NeedsAttention.
    let approve_script = home.join("approve.json");
    std::fs::write(
        &approve_script,
        r#"{
          "pankki-maksu": [{"status": 200, "body": {"receipt": "P1"}}, {"timeout": true}],
          "tulorekisteri": [{"status": 200, "body": {"receipt": "R1"}}]
        }"#,
    )
    .unwrap();
    let (code, stdout, err) = run(
        home,
        Some(&approve_script),
        &[
            "run-payroll",
            "approve-run",
            "--run-id",
            &run_id,
            "--answer",
            "approve",
        ],
        "",
    );
    assert_eq!(code, 0, "{err}");
    let after: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(after["status"], "needs_attention");

    // Recover: emp-2's payment meets a 409 (idempotent), then its report + the
    // monthly report complete → Completed.
    let retry_script = home.join("retry.json");
    std::fs::write(
        &retry_script,
        r#"{
          "pankki-maksu": [{"status": 409, "body": {"receipt": "P2-DUP", "idempotency_key": "pay:emp-2:2026-02-27"}}],
          "tulorekisteri": [{"status": 200, "body": {"receipt": "R2"}}, {"status": 200, "body": {"receipt": "ERIL"}}]
        }"#,
    )
    .unwrap();
    let (code, stdout, err) = run(
        home,
        Some(&retry_script),
        &[
            "run-payroll",
            "handle-payment-failure",
            "--run-id",
            &run_id,
            "--employee",
            "emp-2",
        ],
        "",
    );
    assert_eq!(code, 0, "{err}");
    let recovered: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(recovered["status"], "completed");

    // Two payroll vouchers sealed; the chain verifies.
    let (code, stdout, _) = run(home, None, &["voucher", "verify"], "");
    assert_eq!(code, 0);
    let verify: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(verify["voucher_count"], 2);

    // YTD advanced through the Ledger (durable master-data read/write), year-scoped
    // to the payment date's tax year (2026). The record itself no longer carries a
    // YTD scalar — it is derived on request for the given year.
    let (_, stdout, _) = run(
        home,
        None,
        &[
            "employee",
            "show",
            "--employee",
            "emp-2",
            "--tax-year",
            "2026",
        ],
        "",
    );
    let emp2: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(emp2["ytd_gross"]["amount"], "3500.00");
    assert_eq!(emp2["tax_year"], 2026);
    // A show without --tax-year returns identity only, no YTD scalar.
    let (_, stdout, _) = run(home, None, &["employee", "show", "--employee", "emp-2"], "");
    let plain: Value = serde_json::from_str(&stdout).unwrap();
    assert!(plain.get("ytd_gross").is_none());
}
