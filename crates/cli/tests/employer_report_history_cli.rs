//! `file-wage employer-report import-transcript` and `missing` end to end. The
//! YTJ `registration` command needs the network, so only its help is checked
//! here; its logic is tested in the process crate behind a recorded seam.
use aggountant_ledger::Ledger;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

fn run(home: &Path, args: &[&str]) -> (i32, Value) {
    let o = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .arg("--home")
        .arg(home)
        .arg("--json")
        .args(args)
        .env_remove("AGGOUNTANT_VERO_CREDENTIALS_FILE")
        .env_remove("AGGOUNTANT_BROKER_SCRIPT")
        .env("XDG_CONFIG_HOME", home.join("config"))
        .output()
        .unwrap();
    let out = if o.stdout.is_empty() {
        o.stderr
    } else {
        o.stdout
    };
    (
        o.status.code().unwrap(),
        serde_json::from_slice(&out).unwrap_or_else(|_| json!(String::from_utf8_lossy(&out))),
    )
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../process/tests/fixtures/employer_report")
        .join(name)
}

#[test]
fn imported_transcript_months_leave_missing_and_a_strange_file_is_refused() {
    let home = tempfile::tempdir().unwrap();
    Ledger::init(home.path()).unwrap();
    std::fs::create_dir_all(home.path().join(".aggountant")).unwrap();
    std::fs::write(
        home.path().join(".aggountant/tulorekisteri.json"),
        json!({"schema_version":1,"mode":"production","business_id":"1234567-1","reporting_permission_active":true,"credentials_file":null,
            "employer_report":{"registered_from":"2025-01","contact_name":"Fictional Contact","contact_phone":"+358 40 1234567","pension_insurances":[]}})
            .to_string(),
    )
    .unwrap();
    let csv = fixture("transcript-630-2025.csv");
    let csv = csv.to_str().unwrap();
    let missing = &[
        "file-wage",
        "employer-report",
        "missing",
        "--as-of",
        "2026-03-10",
    ];

    let (code, before) = run(home.path(), missing);
    assert_eq!(code, 0, "{before}");
    assert_eq!(before["months"].as_array().unwrap().len(), 14);

    let ir = &["file-wage", "employer-report", "import-transcript"];
    let (code, dry) = run(home.path(), &[&ir[..], &[csv, "--dry-run"]].concat());
    assert_eq!(code, 0, "{dry}");
    assert_eq!(dry["dry_run"], true);
    assert_eq!(run(home.path(), missing).1["months"], before["months"]);

    let (code, imported) = run(home.path(), &[&ir[..], &[csv]].concat());
    assert_eq!(code, 0, "{imported}");
    assert_eq!(imported["schema_version"], 1);
    assert_eq!(imported["recorded"].as_array().unwrap().len(), 10);
    assert_eq!(imported["not_reported"], json!(["2025-04", "2025-06"]));

    let (_, after) = run(home.path(), missing);
    let months: Vec<&str> = after["months"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["month"].as_str().unwrap())
        .collect();
    assert_eq!(months, ["2025-04", "2025-06", "2026-01", "2026-02"]);
    assert_eq!(after["filed_outside"][0]["month"], "2025-01");
    assert_eq!(
        after["filed_outside"][0]["evidence"]["source"],
        "incomes_register_transcript_630"
    );

    let (code, again) = run(home.path(), &[&ir[..], &[csv]].concat());
    assert_eq!(code, 0);
    assert_eq!(again["already_imported"], true);

    let (code, replace) = run(
        home.path(),
        &[
            "file-wage",
            "employer-report",
            "preview",
            "--month",
            "2025-01",
            "--payments",
            "none",
            "--timestamp",
            "2026-03-10T08:00:00Z",
            "--replace",
        ],
    );
    assert_eq!(code, 1);
    assert_eq!(replace["error_code"], "EMPLOYER_REPORT_NO_REFERENCE");

    let strange = home.path().join("other.csv");
    std::fs::write(&strange, "Päivämäärä;Summa\n1.1.2025;10,00\n").unwrap();
    let (code, refused) = run(
        home.path(),
        &[&ir[..], &[strange.to_str().unwrap()]].concat(),
    );
    assert_eq!(code, 1);
    assert_eq!(refused["error_code"], "TRANSCRIPT_UNKNOWN_HEADER");
    assert_eq!(refused["error_kind"], "user");

    let (code, gone) = run(home.path(), &[&ir[..], &["no-such-file.csv"]].concat());
    assert_eq!(code, 1);
    assert_eq!(gone["error_code"], "TRANSCRIPT_UNREADABLE");
}

#[test]
fn registration_help_names_the_source_and_the_network_failure_code() {
    let o = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(["file-wage", "employer-report", "registration", "--help"])
        .output()
        .unwrap();
    assert!(o.status.success());
    let help = String::from_utf8(o.stdout).unwrap();
    assert!(help.contains("YTJ"));
    assert!(help.contains("YTJ_UNAVAILABLE"));
    assert!(help.contains("Example:"));
}
