mod common;

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
    common::feed_stdin(&mut child, stdin.as_bytes());
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
    let accounting = common::initialized_home();
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

    #[cfg(target_os = "macos")]
    let config_file = "Library/Application Support/aggountant/config.json";
    #[cfg(not(target_os = "macos"))]
    let config_file = ".config/aggountant/config.json";
    let config = std::fs::read_to_string(platform.path().join(config_file)).unwrap();
    assert!(config.contains(credential.to_string_lossy().as_ref()));
    assert!(!config.contains("secret-software-key"));
}

#[test]
fn production_becomes_ready_without_optional_test_verification() {
    let accounting = common::initialized_home();
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
    let accounting = common::initialized_home();
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
    let accounting = common::initialized_home();
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
    let accounting = common::initialized_home();
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

/// A production profile shaped like one a company keeps after onboarding:
/// every attestation given, the activation bound to the certificate, and both
/// verify commands' evidence recorded. Values are synthetic.
fn onboarded_profile(fingerprint: &str) -> Value {
    json!({
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
            "test_verification": {
                "observed_at": "2026-09-12T08:15:30.123Z",
                "certificate_fingerprint_sha256": fingerprint
            },
            "portal_verification": {
                "observed_at": "2026-09-13T09:00:00Z",
                "certificate_fingerprint_sha256": fingerprint,
                "operations": ["GetVATPeriods/v1", "FileVATReturn/v2", "GetFiledVATReturn/v2"],
                "file_vat_return_receipt": "{\"UniqueIdentifier\":\"verification\",\"AcceptedTimestamp\":\"2026-09-13T09:00:00Z\"}"
            }
        }
    })
}

/// An onboarded home: the credential path configured and the stored profile
/// written as the verify commands and an earlier configure left it.
fn onboarded_home() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    tempfile::TempDir,
    Value,
) {
    let accounting = common::initialized_home();
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
        .unwrap()
        .to_string();
    let profile = onboarded_profile(&fingerprint);
    std::fs::write(
        accounting.path().join("vero-filing.json"),
        serde_json::to_string_pretty(&profile).unwrap(),
    )
    .unwrap();
    (accounting, platform, secrets, profile)
}

fn stored(accounting: &std::path::Path) -> Value {
    serde_json::from_slice(&std::fs::read(accounting.join("vero-filing.json")).unwrap()).unwrap()
}

#[test]
fn partial_configure_merges_and_keeps_stored_evidence() {
    let (accounting, platform, _secrets, profile) = onboarded_home();
    let before = std::fs::read(accounting.path().join("vero-filing.json")).unwrap();

    let patch = json!({"contact_phone": "+358 50 7654321"}).to_string();
    let (code, stdout, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure", "--dry-run"],
        &patch,
    );
    assert_eq!(code, 0, "{stderr}");
    let output: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(output["written"], false);
    assert_eq!(output["merged"], true);
    assert_eq!(output["ready"], true);
    assert_eq!(
        output["changes"],
        json!([{"field": "contact_phone", "before": "+358401234567", "after": "+358 50 7654321"}])
    );
    assert_eq!(
        std::fs::read(accounting.path().join("vero-filing.json")).unwrap(),
        before
    );

    let (code, stdout, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure"],
        &patch,
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["written"],
        true
    );
    let mut expected = profile.clone();
    expected["contact_phone"] = json!("+358 50 7654321");
    assert_eq!(stored(accounting.path()), expected);

    // The skill's earlier pattern, an empty attestations object, keeps every
    // attestation instead of resetting them.
    let (code, stdout, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure"],
        &json!({"schema_version": 1, "mode": "production", "attestations": {}}).to_string(),
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["changes"],
        json!([])
    );
    assert_eq!(stored(accounting.path()), expected);

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
fn a_whole_profile_input_with_its_evidence_changes_nothing() {
    let (accounting, platform, _secrets, profile) = onboarded_home();
    let (code, stdout, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure"],
        &profile.to_string(),
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["changes"],
        json!([])
    );
    assert_eq!(stored(accounting.path()), profile);
}

#[test]
fn clearing_or_changing_recorded_evidence_is_refused_without_replace() {
    let (accounting, platform, _secrets, profile) = onboarded_home();
    let before = std::fs::read(accounting.path().join("vero-filing.json")).unwrap();
    let inputs = [
        json!({"attestations": {"portal_verification": null}}),
        json!({"attestations": {"test_verification": {"observed_at": "2026-10-01T00:00:00Z"}}}),
        json!({"attestations": null}),
    ];
    for input in inputs {
        let (code, _, stderr) = invoke(
            accounting.path(),
            platform.path(),
            &["vero-fi", "configure"],
            &input.to_string(),
        );
        assert_eq!(code, 1, "{input}");
        assert!(
            stderr.contains("VERO_EVIDENCE_WOULD_BE_DROPPED"),
            "{stderr}"
        );
        assert_eq!(
            std::fs::read(accounting.path().join("vero-filing.json")).unwrap(),
            before
        );
    }

    // An explicit null clears an ordinary field; production readiness then
    // refuses it because the activation is no longer bound.
    let (code, _, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure"],
        &json!({"attestations": {"activation_certificate_fingerprint_sha256": null}}).to_string(),
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("VERO_NOT_READY"), "{stderr}");

    // --replace writes the input as the whole profile, dropping what it omits.
    let mut whole = profile.clone();
    whole["mode"] = json!("disabled");
    whole["attestations"]
        .as_object_mut()
        .unwrap()
        .remove("portal_verification");
    let (code, stdout, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure", "--replace"],
        &whole.to_string(),
    );
    assert_eq!(code, 0, "{stderr}");
    let output: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(output["merged"], false);
    assert!(output["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(
            |change| change["field"] == "attestations.portal_verification.observed_at"
                && change["after"].is_null()
        ));
    assert_eq!(stored(accounting.path()), whole);
}

#[test]
fn merge_input_stays_strict_and_an_invalid_stored_profile_needs_replace() {
    let (accounting, platform, _secrets, profile) = onboarded_home();
    let before = std::fs::read(accounting.path().join("vero-filing.json")).unwrap();
    for input in [
        json!({"attestations": {"unexpected": true}}),
        json!({"contact_phone": "not a phone"}),
        json!(["not", "an", "object"]),
    ] {
        let (code, _, stderr) = invoke(
            accounting.path(),
            platform.path(),
            &["vero-fi", "configure"],
            &input.to_string(),
        );
        assert_eq!(code, 1, "{input}");
        assert!(stderr.contains("INVALID_INPUT"), "{stderr}");
    }
    assert_eq!(
        std::fs::read(accounting.path().join("vero-filing.json")).unwrap(),
        before
    );

    std::fs::write(accounting.path().join("vero-filing.json"), "{\"mode\":").unwrap();
    let (code, _, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure"],
        &json!({"contact_phone": "+358401234567"}).to_string(),
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("VERO_PROFILE_INVALID"), "{stderr}");
    let (code, _, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure", "--replace"],
        &profile.to_string(),
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stored(accounting.path()), profile);
}

#[test]
fn a_first_configure_without_mode_is_refused() {
    let accounting = common::initialized_home();
    let platform = tempfile::tempdir().unwrap();
    let (code, _, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure"],
        &json!({"contact_phone": "+358401234567"}).to_string(),
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("INVALID_INPUT"), "{stderr}");
    assert!(stderr.contains("mode"), "{stderr}");
    assert!(!accounting.path().join("vero-filing.json").exists());
}

#[test]
fn a_misspelt_field_cleared_with_null_is_refused() {
    let (accounting, platform, _secrets, _) = onboarded_home();
    let before = std::fs::read(accounting.path().join("vero-filing.json")).unwrap();
    for input in [
        json!({"unexpected": null}),
        json!({"attestations": {"production_permission_activ": null}}),
        json!({"attestations": {"portal_verification": {"observed": null}}}),
    ] {
        let (code, _, stderr) = invoke(
            accounting.path(),
            platform.path(),
            &["vero-fi", "configure"],
            &input.to_string(),
        );
        assert_eq!(code, 1, "{input}");
        assert!(stderr.contains("INVALID_INPUT"), "{stderr}");
        assert!(stderr.contains("unknown field"), "{stderr}");
    }
    assert_eq!(
        std::fs::read(accounting.path().join("vero-filing.json")).unwrap(),
        before
    );
}

#[test]
fn a_new_credential_path_is_listed_among_the_changes() {
    let (accounting, platform, secrets, _) = onboarded_home();
    let old = secrets.path().join("vero.json");
    let new = secrets.path().join("vero-renewed.json");
    write_credential(&new, "production");
    let input = json!({"credential_path": new}).to_string();
    for args in [
        &["vero-fi", "configure", "--dry-run"][..],
        &["vero-fi", "configure"],
    ] {
        let (code, stdout, stderr) = invoke(accounting.path(), platform.path(), args, &input);
        assert_eq!(code, 0, "{stderr}");
        assert_eq!(
            serde_json::from_str::<Value>(&stdout).unwrap()["changes"],
            json!([{"field": "credential_path", "before": old, "after": new}]),
            "{args:?}"
        );
    }
    let (code, stdout, stderr) = invoke(
        accounting.path(),
        platform.path(),
        &["vero-fi", "configure"],
        &input,
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["changes"],
        json!([])
    );
}
