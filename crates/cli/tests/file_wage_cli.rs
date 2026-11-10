use aggountant_domain::TaxCard;
use aggountant_ledger::{EmployeeState, Ledger};
use serde_json::{json, Value};
use std::{
    io::Write,
    process::{Command, Stdio},
};
fn run(home: &std::path::Path, args: &[&str], input: &str) -> (i32, String, String) {
    let mut c = Command::new(env!("CARGO_BIN_EXE_aggountant"));
    c.arg("--home")
        .arg(home)
        .arg("--json")
        .args(args)
        .env_remove("AGGOUNTANT_VERO_CREDENTIALS_FILE")
        .env_remove("AGGOUNTANT_BROKER_SCRIPT")
        .env("XDG_CONFIG_HOME", home.join("config"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut c = c.spawn().unwrap();
    c.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let o = c.wait_with_output().unwrap();
    (
        o.status.code().unwrap(),
        String::from_utf8(o.stdout).unwrap(),
        String::from_utf8(o.stderr).unwrap(),
    )
}
#[test]
fn offline_preview_start_dryrun_approval_guard_and_manual_fallback() {
    let home = tempfile::tempdir().unwrap();
    let ledger = Ledger::init(home.path()).unwrap();
    ledger
        .upsert_employee(&EmployeeState {
            employee_id: "emp-1".into(),
            name: "Fictional Employee".into(),
            hetu: "010101A123N".into(),
            iban: "FI2112345600000785".into(),
            tax_card: TaxCard {
                withholding_pct: "0".into(),
            },
        })
        .unwrap();
    let report=json!({"business_id":"1234567-1","source_reference":"fictional-expense","employee_id":"emp-1","payment_date":"2026-10-25","period_start":"2026-09-01","period_end":"2026-09-30","joint_owner":true,"contact_name":"Fictional Contact","contact_phone":"+358 40 1234567","lines":[{"income_type":"311","amount":"120.00","kilometers":200},{"income_type":"331","amount":"54.00","allowance_code":3}]}).to_string();
    let (code, out, err) = run(
        home.path(),
        &[
            "file-wage",
            "preview",
            "--timestamp",
            "2026-10-25T12:00:00Z",
        ],
        &report,
    );
    assert_eq!(code, 0, "{err}");
    let p: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(p["schema_version"], 1);
    assert_eq!(p["deadline"], "2026-10-30");
    assert_eq!(p["report"]["joint_owner"], true);
    assert!(!out.contains("010101A123N"));
    assert!(!home.path().join("process").exists());
    let (code, _, err) = run(
        home.path(),
        &[
            "file-wage",
            "start",
            "--timestamp",
            "2026-10-25T12:00:00Z",
            "--dry-run",
        ],
        &report,
    );
    assert_eq!(code, 0, "{err}");
    assert!(!home.path().join("process").exists());
    let (code, out, err) = run(
        home.path(),
        &["file-wage", "start", "--timestamp", "2026-10-25T12:00:00Z"],
        &report,
    );
    assert_eq!(code, 0, "{err}");
    let started: Value = serde_json::from_str(&out).unwrap();
    let id = started["run_id"].as_str().unwrap();
    assert_eq!(started["status"], "awaiting_gate");
    let (code, _, err) = run(
        home.path(),
        &[
            "file-wage",
            "approve",
            id,
            "--record-hash",
            p["record_hash"].as_str().unwrap(),
        ],
        "",
    );
    assert_eq!(code, 1);
    assert!(err.contains("TULOREKISTERI_NOT_READY"));
    let (code, out, err) = run(
        home.path(),
        &[
            "file-wage",
            "complete-manual",
            id,
            "--ir-delivery-id",
            "11111111222233334444555555555555",
            "--ir-report-id",
            "22222222222233334444555555555555",
        ],
        "",
    );
    assert_eq!(code, 0, "{err}");
    let done: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(done["status"], "completed");
}
