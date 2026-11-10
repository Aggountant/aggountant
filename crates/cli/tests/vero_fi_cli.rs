use std::io::Write;
use std::process::{Command, Stdio};

use aggountant_vero_vat_api_mock::MtlsTestCredentials;
use serde_json::{json, Value};

fn invoke(
    accounting_home: &std::path::Path,
    platform_home: &std::path::Path,
    args: &[&str],
    stdin: &str,
) -> (i32, String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aggountant"));
    command
        .arg("--home")
        .arg(accounting_home)
        .arg("--json")
        .args(args)
        .env("HOME", platform_home)
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("AGGOUNTANT_VERO_CREDENTIALS_FILE")
        .env_remove("AGGOUNTANT_BROKER_SCRIPT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn write_credential(path: &std::path::Path, target: &str) {
    std::fs::write(
        path,
        json!({
            "target": target,
            "business_id": "1572860-0",
            "software_key": "secret-software-key",
            "client_certificate_pem": String::from_utf8_lossy(MtlsTestCredentials::CLIENT_CERTIFICATE_PEM),
            "client_private_key_pem": String::from_utf8_lossy(MtlsTestCredentials::CLIENT_PRIVATE_KEY_PEM),
            "ca_certificate_pem": String::from_utf8_lossy(MtlsTestCredentials::CA_CERTIFICATE_PEM)
        })
        .to_string(),
    )
    .unwrap();
}

fn disabled_input(credential: &std::path::Path) -> Value {
    json!({
        "schema_version": 1,
        "mode": "disabled",
        "business_id": "1572860-0",
        "contact_name": "Ada Operator",
        "contact_phone": "+358401234567",
        "attestations": {},
        "credential_path": credential,
    })
}

#[test]
fn configure_splits_non_secret_profile_from_platform_credential_path() {
    let accounting = tempfile::tempdir().unwrap();
    let platform = tempfile::tempdir().unwrap();
    let secrets = tempfile::tempdir().unwrap();
    let credential = secrets.path().join("vero.json");
    write_credential(&credential, "production");

    let input = disabled_input(&credential).to_string();
    let (code, stdout, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure"],
        &input,
    );
    assert_eq!(code, 0, "{stderr}");
    let output: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(output["ready"], false);
    assert_eq!(output["written"], true);
    assert!(!stdout.contains("secret-software-key"));
    assert!(!stdout.contains("PRIVATE KEY"));

    let profile = std::fs::read_to_string(accounting.path().join("vero-filing.json")).unwrap();
    assert!(profile.contains("\"mode\": \"disabled\""));
    assert!(!profile.contains(credential.to_string_lossy().as_ref()));
    assert!(!profile.contains("secret-software-key"));
    assert!(!profile.contains("PRIVATE KEY"));

    let config =
        std::fs::read_to_string(platform.path().join(".config/aggountant/config.json")).unwrap();
    assert!(config.contains(credential.to_string_lossy().as_ref()));
    assert!(!config.contains("secret-software-key"));
}

#[test]
fn production_becomes_ready_without_optional_test_verification() {
    let accounting = tempfile::tempdir().unwrap();
    let platform = tempfile::tempdir().unwrap();
    let secrets = tempfile::tempdir().unwrap();
    let credential = secrets.path().join("vero.json");
    write_credential(&credential, "production");

    let (code, _, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure"],
        &disabled_input(&credential).to_string(),
    );
    assert_eq!(code, 0, "{stderr}");
    let (_, stdout, _) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "status"],
        "",
    );
    let status: Value = serde_json::from_str(&stdout).unwrap();
    let fingerprint = status["certificate"]["fingerprint_sha256"]
        .as_str()
        .unwrap();

    let mut incomplete = disabled_input(&credential);
    incomplete["mode"] = json!("production");
    let (code, _, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure"],
        &incomplete.to_string(),
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("VERO_NOT_READY"), "{stderr}");

    let ready = json!({
        "schema_version": 1,
        "mode": "production",
        "business_id": "1572860-0",
        "contact_name": "Ada Operator",
        "contact_phone": "+358401234567",
        "attestations": {
            "software_registered": true,
            "api_agreement_active": true,
            "production_verification_completed": true,
            "production_permission_active": true,
            "activation_certificate_fingerprint_sha256": fingerprint,
            "portal_verification": {
                "observed_at": "2026-09-13T00:00:00Z",
                "certificate_fingerprint_sha256": fingerprint,
                "operations": ["GetVATPeriods/v1", "FileVATReturn/v2", "GetFiledVATReturn/v2"],
                "file_vat_return_receipt": "{\"UniqueIdentifier\":\"verification\",\"AcceptedTimestamp\":\"2026-09-13T00:00:00Z\"}"
            }
        }
    });
    let (code, stdout, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure"],
        &ready.to_string(),
    );
    assert_eq!(code, 0, "{stderr}");
    let output: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(output["ready"], true);

    let (_, stdout, _) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "status"],
        "",
    );
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["ready"],
        true
    );
}

#[test]
fn verify_accepts_only_the_explicit_test_target() {
    let accounting = tempfile::tempdir().unwrap();
    let platform = tempfile::tempdir().unwrap();
    let (code, _, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "verify", "--target", "production"],
        "",
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("invalid value 'production'"), "{stderr}");
}

#[test]
fn portal_verification_help_has_no_target_or_url_and_missing_credentials_stop_early() {
    let accounting = tempfile::tempdir().unwrap();
    let platform = tempfile::tempdir().unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(["vero-fi", "verify-portal", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("static Verification endpoints"), "{help}");
    assert!(!help.contains("--target"), "{help}");
    assert!(!help.contains("--url"), "{help}");

    let profile = json!({
        "schema_version": 1,
        "mode": "disabled",
        "business_id": "1572860-0",
        "contact_name": "Ada Operator",
        "contact_phone": "+358401234567",
        "attestations": {}
    });
    let (code, _, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure"],
        &profile.to_string(),
    );
    assert_eq!(code, 0, "{stderr}");
    let (code, _, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "verify-portal"],
        "",
    );
    assert_eq!(code, 1);
    assert!(
        stderr.contains("VERO_PORTAL_VERIFICATION_NOT_READY"),
        "{stderr}"
    );
    assert!(!accounting.path().join("integration/outbox").exists());
}

#[test]
fn configure_is_strict_and_dry_run_writes_nothing() {
    let accounting = tempfile::tempdir().unwrap();
    let platform = tempfile::tempdir().unwrap();
    let input = json!({
        "schema_version": 1,
        "mode": "disabled",
        "attestations": {},
        "unexpected": true
    });
    let (code, _, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure", "--dry-run"],
        &input.to_string(),
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("INVALID_INPUT"));
    assert!(!accounting.path().join("vero-filing.json").exists());
}
