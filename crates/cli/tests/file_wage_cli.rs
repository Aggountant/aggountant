mod common;

use aggountant_domain::TaxCard;
use aggountant_ledger::{EmployeeState, Ledger};
use serde_json::{json, Value};
use std::process::{Command, Stdio};
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
    common::feed_stdin(&mut c, input.as_bytes());
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
    assert!(p["sensitive"]["record_hash"]
        .as_str()
        .unwrap()
        .contains("personal identity code"));
    assert!(!out.contains("010101A123N"));
    assert!(!home.path().join("process").exists());
    let (code, out, err) = run(
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
    let dry: Value = serde_json::from_str(&out).unwrap();
    assert!(dry["would"][0]["preview"]["sensitive"]["record_hash"].is_string());
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
    assert_eq!(done["effects"][0]["outcome"]["state"], "accepted");
    // The recorded receipt is read back offline with `run show`, without a profile
    // or credential.
    let (code, out, err) = run(home.path(), &["run", "show", "--run-id", id], "");
    assert_eq!(code, 0, "{err}");
    let shown: Value = serde_json::from_str(&out).unwrap();
    let receipt = &shown["effects"][0]["outcome"]["receipt"];
    assert_eq!(receipt["authority"], "tulorekisteri");
    assert_eq!(
        receipt["ir_delivery_id"],
        "11111111222233334444555555555555"
    );
    assert_eq!(receipt["ir_report_id"], "22222222222233334444555555555555");
    assert_eq!(receipt["source"], "e_service");

    // Evidence for an already accepted report is refused with its own code; the
    // references are not dropped behind exit code 0.
    let (code, out, err) = run(
        home.path(),
        &[
            "file-wage",
            "complete-manual",
            id,
            "--ir-delivery-id",
            "33333333222233334444555555555555",
            "--ir-report-id",
            "22222222222233334444555555555555",
        ],
        "",
    );
    assert_eq!(code, 1, "{out}");
    let refused: Value = serde_json::from_str(&err).unwrap();
    assert_eq!(refused["error_code"], "WAGE_REPORT_ALREADY_ACCEPTED");
    assert_eq!(refused["error_kind"], "user");
    assert!(refused["message"]
        .as_str()
        .unwrap()
        .contains("nothing was recorded"));

    // A correction is previewed offline: a cancellation reads no stdin, a
    // replacement reads the whole corrected report.
    let at = "2026-11-02T08:00:00Z";
    let (code, out, err) = run(
        home.path(),
        &[
            "file-wage",
            "correction",
            "preview",
            id,
            "--action",
            "cancel",
            "--timestamp",
            at,
        ],
        "",
    );
    assert_eq!(code, 0, "{err}");
    let cancel: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(cancel["schema_version"], 1);
    assert_eq!(cancel["action"], "cancellation");
    assert_eq!(cancel["ir_report_id"], "22222222222233334444555555555555");
    assert!(cancel["corrects_version"].is_null());
    assert!(cancel.get("sensitive").is_none());
    let corrected = report.replace(
        r#""amount":"120.00","income_type":"311","kilometers":200"#,
        r#""amount":"96.00","income_type":"311","kilometers":160"#,
    );
    assert_ne!(corrected, report);
    let replace = ["--action", "replace", "--timestamp", at];
    let (code, out, err) = run(
        home.path(),
        &[&["file-wage", "correction", "preview", id], &replace[..]].concat(),
        &corrected,
    );
    assert_eq!(code, 0, "{err}");
    let preview: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(preview["action"], "replacement");
    assert_eq!(preview["report"]["lines"][0]["kilometers"], 160);
    assert_eq!(preview["previous_report"]["lines"][0]["kilometers"], 200);
    assert!(preview["sensitive"]["record_hash"].is_string());
    assert!(!out.contains("010101A123N"));
    let hash = preview["record_hash"].as_str().unwrap();
    let (code, _, err) = run(
        home.path(),
        &[&["file-wage", "correction", "preview", id], &replace[..]].concat(),
        &report,
    );
    assert_eq!(code, 1);
    assert!(err.contains("WAGE_CORRECTION_UNCHANGED"), "{err}");
    let (code, out, err) = run(
        home.path(),
        &[
            &["file-wage", "correction", "submit", id],
            &replace[..],
            &["--record-hash", hash, "--dry-run"],
        ]
        .concat(),
        &corrected,
    );
    assert_eq!(code, 0, "{err}");
    let dry: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(dry["would"][0]["matches_approval"], true);
    // Without a profile and credential nothing is sent and nothing is recorded.
    let (code, _, err) = run(
        home.path(),
        &[
            &["file-wage", "correction", "submit", id],
            &replace[..],
            &["--record-hash", hash],
        ]
        .concat(),
        &corrected,
    );
    assert_eq!(code, 1);
    assert!(err.contains("TULOREKISTERI_NOT_READY"), "{err}");
    let (code, out, err) = run(home.path(), &["file-wage", "correction", "status", id], "");
    assert_eq!(code, 0, "{err}");
    let status: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(status["state"], "accepted");
    assert_eq!(status["filings"].as_array().unwrap().len(), 1);

    // The e-service fallback: a replacement made there is recorded from evidence.
    let manual = |version: &str| {
        run(
            home.path(),
            &[
                "file-wage",
                "correction",
                "complete-manual",
                id,
                "--action",
                "replace",
                "--ir-report-id",
                "22222222222233334444555555555555",
                "--report-version",
                version,
            ],
            "",
        )
    };
    let (code, out, err) = manual("2");
    assert_eq!(code, 0, "{err}");
    let status: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(status["state"], "replaced");
    assert_eq!(status["current"]["version"], 2);
    assert_eq!(status["current"]["source"], "e_service");
    assert_eq!(status["report_is_current"], false);
    assert_eq!(status["filings"][1]["kind"], "replacement");
    assert_eq!(status["filings"][1]["sent_by_aggountant"], false);
    let (code, _, err) = manual("2");
    assert_eq!(code, 1);
    assert!(err.contains("WAGE_CORRECTION_INVALID_EVIDENCE"), "{err}");
    // `run show` of the report now says its receipt is not the current one.
    let (code, out, err) = run(home.path(), &["run", "show", "--run-id", id], "");
    assert_eq!(code, 0, "{err}");
    let shown: Value = serde_json::from_str(&out).unwrap();
    let outcome = &shown["effects"][0]["outcome"];
    assert_eq!(outcome["state"], "superseded");
    assert_eq!(
        outcome["next_command"],
        format!("aggountant file-wage correction status {id}")
    );
    let correction_run = status["filings"][1]["run_id"].as_str().unwrap();
    let (code, out, err) = run(
        home.path(),
        &["run", "show", "--run-id", correction_run],
        "",
    );
    assert_eq!(code, 0, "{err}");
    let shown: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(shown["status"], "completed");
    assert_eq!(shown["effects"][0]["outcome"]["receipt"]["version"], 2);
}

#[test]
fn employer_report_preview_missing_manual_evidence_and_offline_guards() {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    let h = home.path();
    let ok = |args: &[&str]| {
        let (code, out, err) = run(h, args, "");
        assert_eq!(code, 0, "{args:?}: {err}");
        serde_json::from_str::<Value>(&out).unwrap()
    };
    let fails = |args: &[&str]| {
        let (code, _, err) = run(h, args, "");
        assert_eq!(code, 1, "{args:?}");
        serde_json::from_str::<Value>(&err).unwrap()["error_code"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let preview = [
        "file-wage",
        "employer-report",
        "preview",
        "--month",
        "2026-10",
        "--payments",
        "none",
        "--timestamp",
        "2026-11-02T08:00:00Z",
    ];
    assert_eq!(fails(&preview), "EMPLOYER_REPORT_NOT_CONFIGURED");
    std::fs::create_dir_all(h.join(".aggountant")).unwrap();
    std::fs::write(
        h.join(".aggountant/tulorekisteri.json"),
        json!({"schema_version":1,"mode":"production","business_id":"1234567-1","reporting_permission_active":true,"credentials_file":null,
            "employer_report":{"registered_from":"2026-08","contact_name":"Fictional Contact","contact_phone":"+358 40 1234567","pension_insurances":[]}})
            .to_string(),
    )
    .unwrap();
    let p = ok(&preview);
    assert_eq!(p["schema_version"], 1);
    assert_eq!(p["deadline"], "2026-11-05");
    assert_eq!(p["report"]["content"]["kind"], "no_wages_payable");
    let hash = p["record_hash"].as_str().unwrap().to_string();
    let mut wages = preview;
    wages[6] = "wages";
    assert_eq!(fails(&wages), "EMPLOYER_REPORT_WAGES_UNSUPPORTED");

    let mut submit: Vec<&str> = preview.to_vec();
    submit[2] = "submit";
    submit.extend(["--record-hash", &hash, "--dry-run"]);
    let dry = ok(&submit);
    assert_eq!(dry["would"][0]["matches_approval"], true);
    assert!(!h.join("process").exists());
    submit.pop();
    // No credential: refused before anything is written or sent.
    assert_eq!(fails(&submit), "MISSING_CREDENTIAL");
    assert!(!h.join("process").exists());

    let missing = ok(&[
        "file-wage",
        "employer-report",
        "missing",
        "--as-of",
        "2026-11-16",
    ]);
    let months: Vec<_> = missing["months"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["month"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(months, ["2026-08", "2026-09", "2026-10"]);

    let done = ok(&[
        "file-wage",
        "employer-report",
        "complete-manual",
        "--month",
        "2026-08",
        "--ir-report-id",
        "22222222222233334444555555555555",
        "--report-version",
        "1",
    ]);
    assert_eq!(done["state"], "accepted");
    assert_eq!(done["current"]["source"], "e_service");
    let status = ok(&[
        "file-wage",
        "employer-report",
        "status",
        "--month",
        "2026-08",
    ]);
    assert_eq!(status["state"], "accepted");
    let missing = ok(&[
        "file-wage",
        "employer-report",
        "missing",
        "--as-of",
        "2026-11-16",
    ]);
    assert_eq!(missing["months"].as_array().unwrap().len(), 2);
    let mut again = preview;
    again[4] = "2026-08";
    assert_eq!(fails(&again), "EMPLOYER_REPORT_ALREADY_FILED");
}
