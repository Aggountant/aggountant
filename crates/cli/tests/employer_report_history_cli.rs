//! `file-wage employer-report import-transcript` and `missing` end to end. The
//! YTJ `registration` command needs the network, so only its help is checked
//! here; its logic is tested in the process crate behind a recorded seam.
mod common;

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

    // With the reference recorded, a replacement still needs the original's
    // pension insurances, which only the e-service shows.
    let (code, recorded) = run(
        home.path(),
        &[
            "file-wage",
            "employer-report",
            "complete-manual",
            "--month",
            "2025-01",
            "--ir-report-id",
            "22222222222233334444555555555555",
        ],
    );
    assert_eq!(code, 0, "{recorded}");
    let preview = |extra: &[&str]| {
        run(
            home.path(),
            &[
                &[
                    "file-wage",
                    "employer-report",
                    "preview",
                    "--month",
                    "2025-01",
                    "--payments",
                    "tax-free-reimbursements-only",
                    "--timestamp",
                    "2026-03-10T08:00:00Z",
                    "--replace",
                ][..],
                extra,
            ]
            .concat(),
        )
    };
    let (code, unknown) = preview(&[]);
    assert_eq!(code, 1);
    assert_eq!(unknown["error_code"], "EMPLOYER_REPORT_PREVIOUS_UNKNOWN");
    let (code, stated) = preview(&["--previous-pension-insurance", "46:12-3456789"]);
    assert_eq!(code, 0, "{stated}");
    assert_eq!(
        stated["report"]["pension_insurances"],
        json!([{"provider_code":46,"policy_number":"12-3456789"}])
    );
    let (code, none) = preview(&["--previous-no-pension-insurance"]);
    assert_eq!(code, 0, "{none}");
    assert_eq!(none["report"]["pension_insurances"], json!([]));
    let (code, both) = preview(&[
        "--previous-no-pension-insurance",
        "--previous-pension-insurance",
        "46:12-3456789",
    ]);
    assert_ne!(code, 0, "{both}");

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

fn run_stdin(home: &Path, args: &[&str], stdin: &str) -> (i32, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .arg("--home")
        .arg(home)
        .arg("--json")
        .args(args)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    common::feed_stdin(&mut child, stdin.as_bytes());
    let o = child.wait_with_output().unwrap();
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

/// Only `--offline` runs here: the YTJ comparison is tested in the process
/// crate behind the recorded seam.
#[test]
fn configure_writes_only_the_employer_report_block_from_flags_or_stdin() {
    let home = common::initialized_home();
    let h = home.path();
    let profile = h.join(".aggountant/tulorekisteri.json");
    let configure = ["file-wage", "employer-report", "configure", "--offline"];
    let flags = [
        "--registered-from",
        "2020-02",
        "--contact-name",
        "Fictional Contact",
        "--contact-phone",
        "+358 40 1234567",
    ];

    let (code, err) = run(
        h,
        &[&configure[..], &flags[..], &["--no-pension-insurance"]].concat(),
    );
    assert_eq!(code, 1);
    assert_eq!(err["error_code"], "TULOREKISTERI_NOT_READY");

    std::fs::create_dir_all(h.join(".aggountant")).unwrap();
    let original = "{\n    \"schema_version\": 1,\n    \"mode\": \"production\",\n    \"business_id\": \"1234567-1\",\n    \"reporting_permission_active\": true,\n    \"credentials_file\": null\n}\n";
    std::fs::write(&profile, original).unwrap();

    // Neither pension flag: refused, an empty list is never a default.
    let (code, err) = run(h, &[&configure[..], &flags[..]].concat());
    assert_eq!(code, 1);
    assert_eq!(err["error_code"], "INVALID_INPUT");
    assert!(err["message"]
        .as_str()
        .unwrap()
        .contains("--no-pension-insurance"));
    // Both pension flags: clap refuses the conflict.
    let (code, _) = run(
        h,
        &[
            &configure[..],
            &flags[..],
            &["--no-pension-insurance", "--pension-insurance", "46:1"],
        ]
        .concat(),
    );
    assert_ne!(code, 0);
    let (code, err) = run(
        h,
        &[
            &configure[..],
            &flags[..],
            &["--pension-insurance", "46:far too long policy"],
        ]
        .concat(),
    );
    assert_ne!(code, 0, "{err}");
    let (code, err) = run(
        h,
        &[
            "file-wage",
            "employer-report",
            "configure",
            "--offline",
            "--registered-from",
            "2020-02",
            "--contact-name",
            "Fictional Contact",
            "--contact-phone",
            "call me",
            "--no-pension-insurance",
        ],
    );
    assert_eq!(code, 1);
    assert_eq!(err["error_code"], "INVALID_INPUT");
    assert_eq!(std::fs::read_to_string(&profile).unwrap(), original);

    let pension = [
        "--pension-insurance",
        "46:12-3456789",
        "--pension-insurance",
        "54:A1",
    ];
    let (code, dry) = run(
        h,
        &[&configure[..], &flags[..], &pension[..], &["--dry-run"]].concat(),
    );
    assert_eq!(code, 0, "{dry}");
    assert_eq!(dry["schema_version"], 1);
    assert_eq!(dry["written"], false);
    assert_eq!(dry["would"][0]["field"], "employer_report");
    assert_eq!(dry["registration_verdict"], "not_checked");
    assert_eq!(std::fs::read_to_string(&profile).unwrap(), original);
    // Without --json the before/after and the dry run are still visible.
    let human = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .arg("--home")
        .arg(h)
        .args([&configure[..], &flags[..], &pension[..], &["--dry-run"]].concat())
        .env("XDG_CONFIG_HOME", h.join("config"))
        .output()
        .unwrap();
    let human = String::from_utf8(human.stdout).unwrap();
    assert!(human.contains("dry run, nothing written"), "{human}");
    assert!(human.contains("before: null"), "{human}");
    assert!(human.contains("12-3456789"), "{human}");

    let (code, done) = run(h, &[&configure[..], &flags[..], &pension[..]].concat());
    assert_eq!(code, 0, "{done}");
    assert_eq!(done["written"], true);
    assert_eq!(done["before"], Value::Null);
    assert_eq!(
        done["after"]["pension_insurances"][1],
        json!({"provider_code":54,"policy_number":"A1"})
    );
    let written = std::fs::read_to_string(&profile).unwrap();
    assert!(written.starts_with(original.trim_end().trim_end_matches('}').trim_end()));
    let (code, missing) = run(
        h,
        &[
            "file-wage",
            "employer-report",
            "missing",
            "--as-of",
            "2020-04-10",
        ],
    );
    assert_eq!(code, 0, "{missing}");
    assert_eq!(missing["registered_from"], "2020-02");

    // The same block as stdin JSON replaces it; a partial flag set is refused.
    let block = json!({"registered_from":"2021-05","contact_name":"Other Contact","contact_phone":"040 1234567","pension_insurances":[]});
    let (code, done) = run_stdin(h, &configure, &block.to_string());
    assert_eq!(code, 0, "{done}");
    assert_eq!(done["before"]["registered_from"], "2020-02");
    assert_eq!(done["after"], block);
    let (code, err) = run_stdin(
        h,
        &[&configure[..], &["--registered-from", "2021-05"]].concat(),
        &block.to_string(),
    );
    assert_eq!(code, 1);
    assert!(err["message"]
        .as_str()
        .unwrap()
        .contains("missing --contact-name"));
    let (code, err) = run_stdin(h, &configure, r#"{"registered_from":"2021-05"}"#);
    assert_eq!(code, 1);
    assert_eq!(err["error_code"], "INVALID_INPUT");
}

#[test]
fn configure_help_names_the_register_check_and_its_escape_hatches() {
    let o = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(["file-wage", "employer-report", "configure", "--help"])
        .output()
        .unwrap();
    assert!(o.status.success());
    let help = String::from_utf8(o.stdout).unwrap();
    for needle in [
        "YTJ",
        "EMPLOYER_REGISTER_MISMATCH",
        "--accept-register-mismatch",
        "--offline",
        "--no-pension-insurance",
        "PROVIDER_CODE:POLICY_NUMBER",
        "Examples:",
    ] {
        assert!(help.contains(needle), "{needle}");
    }
}
