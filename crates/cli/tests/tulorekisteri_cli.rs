//! `tulorekisteri configure|status` through the built binary: the profile is
//! written by a command that merges like `vero-fi configure`, readiness lists
//! every unmet requirement with an owner, the employer_report block written by
//! its own command survives, and `explain` knows TULOREKISTERI_NOT_READY.

mod common;

use std::path::Path;
use std::process::{Command, Stdio};

use aggountant_vero_vat_api_mock::WageTestCredentials;
use serde_json::{json, Value};

fn invoke(home: &Path, platform: &Path, args: &[&str], stdin: &str) -> (i32, Value, Value) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aggountant"));
    command
        .arg("--home")
        .arg(home)
        .arg("--json")
        .args(args)
        .env("HOME", platform)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("AGGOUNTANT_VERO_CREDENTIALS_FILE")
        .env_remove("AGGOUNTANT_BROKER_SCRIPT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    common::feed_stdin(&mut child, stdin.as_bytes());
    let output = child.wait_with_output().unwrap();
    let parse = |bytes: &[u8]| serde_json::from_slice(bytes).unwrap_or(Value::Null);
    (
        output.status.code().unwrap(),
        parse(&output.stdout),
        parse(&output.stderr),
    )
}

fn write_credential(path: &Path) {
    std::fs::write(
        path,
        json!({
            "target": "production",
            "business_id": "1234567-1",
            "software_key": "fictional-software-key",
            "client_certificate_pem": WageTestCredentials::CERT,
            "client_private_key_pem": WageTestCredentials::KEY,
            "ca_certificate_pem": WageTestCredentials::CA
        })
        .to_string(),
    )
    .unwrap();
}

fn codes(report: &Value) -> Vec<&str> {
    report["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["code"].as_str().unwrap())
        .collect()
}

#[test]
fn configure_merges_status_itemises_and_employer_report_survives() {
    let home = tempfile::tempdir().unwrap();
    let platform = tempfile::tempdir().unwrap();
    let (h, p) = (home.path(), platform.path());
    let profile_path = h.join(".aggountant/tulorekisteri.json");

    let (code, status, err) = invoke(h, p, &["tulorekisteri", "status"], "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(status["schema_version"], 1);
    assert_eq!(status["ready"], false);
    assert_eq!(codes(&status), ["TULOREKISTERI_PROFILE_MISSING"]);

    // Disabled first: written without a credential, blockers itemised.
    let disabled = json!({"schema_version":1,"mode":"disabled","business_id":"1234567-1","reporting_permission_active":false}).to_string();
    let (code, dry, err) = invoke(
        h,
        p,
        &["tulorekisteri", "configure", "--dry-run"],
        &disabled,
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(dry["written"], false);
    assert_eq!(dry["would"][0]["action"], "create");
    assert!(!profile_path.exists());
    let (code, written, err) = invoke(h, p, &["tulorekisteri", "configure"], &disabled);
    assert_eq!(code, 0, "{err}");
    assert_eq!(written["written"], true);
    assert_eq!(
        codes(&written),
        [
            "TULOREKISTERI_DISABLED",
            "TULOREKISTERI_REPORTING_PERMISSION_UNATTESTED",
            "TULOREKISTERI_CREDENTIAL_PATH_MISSING"
        ]
    );
    let owners: Vec<&str> = written["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["owner"].as_str().unwrap())
        .collect();
    assert_eq!(owners, ["human", "human", "agent"]);

    // The employer_report block, written by its own command, is kept.
    let (code, _, err) = invoke(
        h,
        p,
        &[
            "file-wage",
            "employer-report",
            "configure",
            "--offline",
            "--registered-from",
            "2026-08",
            "--contact-name",
            "Fictional Contact",
            "--contact-phone",
            "+358 40 1234567",
            "--no-pension-insurance",
        ],
        "",
    );
    assert_eq!(code, 0, "{err}");

    // Production is refused, naming each blocker, while the channel is not ready.
    let production = json!({"mode":"production","reporting_permission_active":true}).to_string();
    let (code, _, err) = invoke(h, p, &["tulorekisteri", "configure"], &production);
    assert_eq!(code, 1);
    assert_eq!(err["error_code"], "TULOREKISTERI_NOT_READY");
    let message = err["message"].as_str().unwrap();
    assert!(
        message.contains("TULOREKISTERI_CREDENTIAL_PATH_MISSING") && !message.contains("DISABLED"),
        "{message}"
    );
    assert!(err["hint"].is_string(), "explain has an entry: {err}");

    // With a credential outside the home the merge goes through.
    let credential = p.join("tulorekisteri-credentials.json");
    write_credential(&credential);
    let production = json!({"mode":"production","reporting_permission_active":true,"credentials_file":credential}).to_string();
    let (code, ready, err) = invoke(h, p, &["tulorekisteri", "configure"], &production);
    assert_eq!(code, 0, "{err}");
    assert_eq!(ready["ready"], true, "{ready}");
    assert_eq!(ready["employer_report_configured"], true);
    let changed: Vec<&str> = ready["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["field"].as_str().unwrap())
        .collect();
    assert_eq!(
        changed,
        ["credentials_file", "mode", "reporting_permission_active"]
    );
    let stored: Value = serde_json::from_slice(&std::fs::read(&profile_path).unwrap()).unwrap();
    assert_eq!(stored["employer_report"]["registered_from"], "2026-08");
    assert_eq!(stored["business_id"], "1234567-1");

    let (code, status, _) = invoke(h, p, &["tulorekisteri", "status"], "");
    assert_eq!(code, 0);
    assert_eq!(status["ready"], true, "{status}");
    assert!(status["certificate"]["not_after"].is_string());

    // --replace rewrites the channel settings and keeps the employer_report block.
    let (code, replaced, err) = invoke(
        h,
        p,
        &["tulorekisteri", "configure", "--replace"],
        &json!({"schema_version":1,"mode":"disabled","business_id":"1234567-1","reporting_permission_active":true}).to_string(),
    );
    assert_eq!(code, 0, "{err}");
    let changed: Vec<&str> = replaced["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["field"].as_str().unwrap())
        .collect();
    assert_eq!(changed, ["credentials_file", "mode"]);
    let stored: Value = serde_json::from_slice(&std::fs::read(&profile_path).unwrap()).unwrap();
    assert_eq!(stored["employer_report"]["registered_from"], "2026-08");
    let (code, _, err) = invoke(h, p, &["tulorekisteri", "configure"], &production);
    assert_eq!(code, 0, "{err}");

    // A credential for another company is a human blocker, not a vague refusal.
    let (code, status, _) = invoke(
        h,
        p,
        &["tulorekisteri", "configure", "--dry-run"],
        &json!({"mode":"disabled","business_id":"7654321-2"}).to_string(),
    );
    assert_eq!(code, 0);
    assert_eq!(
        codes(&status),
        [
            "TULOREKISTERI_DISABLED",
            "TULOREKISTERI_CREDENTIAL_BUSINESS_ID_MISMATCH"
        ]
    );
}

#[test]
fn configure_refuses_what_it_does_not_own_or_cannot_read() {
    let home = tempfile::tempdir().unwrap();
    let platform = tempfile::tempdir().unwrap();
    let (h, p) = (home.path(), platform.path());
    let fails = |args: &[&str], input: &str| {
        let (code, _, err) = invoke(h, p, args, input);
        assert_eq!(code, 1, "{err}");
        err["error_code"].as_str().unwrap().to_string()
    };
    let configure = ["tulorekisteri", "configure"];
    assert_eq!(
        fails(&configure, r#"{"employer_report":{}}"#),
        "INVALID_INPUT"
    );
    assert_eq!(
        fails(&configure, r#"{"modee":"disabled"}"#),
        "INVALID_INPUT"
    );
    // Nothing stored to merge into: the required fields must be given.
    assert_eq!(fails(&configure, r#"{"mode":"disabled"}"#), "INVALID_INPUT");
    let base = json!({"schema_version":1,"mode":"disabled","business_id":"1234567-1","reporting_permission_active":false});
    let mut test_mode = base.clone();
    test_mode["mode"] = json!("test");
    assert_eq!(fails(&configure, &test_mode.to_string()), "INVALID_INPUT");
    let mut relative = base.clone();
    relative["credentials_file"] = json!("credentials.json");
    assert_eq!(
        fails(&configure, &relative.to_string()),
        "INVALID_CREDENTIAL_PATH"
    );

    // A broken stored file is reported by status and overwritten only with --replace.
    std::fs::create_dir_all(h.join(".aggountant")).unwrap();
    std::fs::write(h.join(".aggountant/tulorekisteri.json"), "[]").unwrap();
    let (_, status, _) = invoke(h, p, &["tulorekisteri", "status"], "");
    assert_eq!(codes(&status), ["TULOREKISTERI_PROFILE_INVALID"]);
    assert_eq!(
        fails(&configure, r#"{"mode":"disabled"}"#),
        "TULOREKISTERI_PROFILE_INVALID"
    );
    let (code, _, err) = invoke(
        h,
        p,
        &["tulorekisteri", "configure", "--replace"],
        &base.to_string(),
    );
    assert_eq!(code, 0, "{err}");
}
