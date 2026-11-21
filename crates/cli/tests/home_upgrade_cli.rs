//! `home upgrade` through the built binary: the mode flag is required, the
//! dry-run emits the planning envelope and writes nothing, the apply reports what
//! it created, and a re-run reports everything as existing.

use std::process::Command;

use serde_json::Value;

fn run(home: &std::path::Path, args: &[&str]) -> (i32, Value, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .arg("--home")
        .arg(home)
        .arg("--json")
        .args(args)
        .output()
        .expect("spawn aggountant");
    let stdout = String::from_utf8(out.stdout).unwrap();
    let value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
    (
        out.status.code().unwrap_or(-1),
        value,
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn home_upgrade_plans_applies_and_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("esimerkki");
    assert_eq!(run(&home, &["init"]).0, 0);
    let reference = home.join("masterdata").join("reference");
    // The layout an older `init` wrote.
    std::fs::remove_file(reference.join("vat-account-roles.json")).unwrap();
    std::fs::remove_file(reference.join("settlement-account-roles.json")).unwrap();
    std::fs::remove_dir_all(reference.join("chart-snapshots")).unwrap();

    // Every command that opens masterdata names the remedy, not just the file.
    let (code, _, stderr) = run(&home, &["chart", "list"]);
    assert_eq!(code, 1, "{stderr}");
    let error: Value = serde_json::from_str(stderr.trim()).unwrap();
    assert_eq!(error["error_code"], "MASTERDATA_CORRUPT");
    let message = error["message"].as_str().unwrap();
    assert!(
        message.contains("aggountant home upgrade --dry-run")
            && message.contains("vat-account-roles.json"),
        "{message}"
    );

    let (code, _, stderr) = run(&home, &["home", "upgrade"]);
    assert_eq!(code, 1, "a mode is required: {stderr}");

    let (code, plan, _) = run(&home, &["home", "upgrade", "--dry-run"]);
    assert_eq!(code, 0);
    assert_eq!(plan["schema_version"], 1);
    assert_eq!(plan["dry_run"], true);
    assert_eq!(plan["changed"], false);
    assert_eq!(plan["masterdata_opens"], true);
    let would = plan["would"].as_array().unwrap();
    assert_eq!(would.len(), 3);
    assert_eq!(would[1]["resource"], "vat_account_roles");
    assert_eq!(would[2]["resource"], "settlement_account_roles");
    assert_eq!(would[2]["path"], "settlement-account-roles.json");
    assert!(!reference.join("vat-account-roles.json").exists());
    assert!(!reference.join("settlement-account-roles.json").exists());
    assert!(!reference.join("chart-snapshots").exists());

    let (code, applied, _) = run(&home, &["home", "upgrade", "--apply"]);
    assert_eq!(code, 0);
    assert_eq!(applied["dry_run"], false);
    assert_eq!(applied["changed"], true);
    let created: Vec<_> = applied["created"].as_array().unwrap().to_vec();
    let planned: Vec<_> = would.iter().map(|w| w["path"].clone()).collect();
    assert_eq!(created, planned);

    let (code, again, _) = run(&home, &["home", "upgrade", "--apply"]);
    assert_eq!(code, 0);
    assert_eq!(again["changed"], false);
    assert_eq!(again["created"].as_array().unwrap().len(), 0);
    assert_eq!(again["existed"].as_array().unwrap().len(), 3);

    assert_eq!(run(&home, &["voucher", "verify"]).0, 0);
}

#[test]
fn home_upgrade_of_a_missing_home_is_a_user_error() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("missing");
    let (code, _, stderr) = run(&home, &["home", "upgrade", "--dry-run"]);
    assert_eq!(code, 1);
    let error: Value = serde_json::from_str(stderr.trim()).unwrap();
    assert_eq!(error["error_code"], "MASTERDATA_NOT_INITIALIZED");
    assert!(!home.exists());
}

#[test]
fn home_upgrade_adds_only_the_settlement_roles_to_a_recent_home() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("esimerkki");
    assert_eq!(run(&home, &["init"]).0, 0);
    let reference = home.join("masterdata").join("reference");
    let roles = reference.join("settlement-account-roles.json");
    std::fs::remove_file(&roles).unwrap();
    let vat_roles = std::fs::read(reference.join("vat-account-roles.json")).unwrap();
    let chart = std::fs::read(reference.join("chart-of-accounts.json")).unwrap();

    let (code, plan, _) = run(&home, &["home", "upgrade", "--dry-run"]);
    assert_eq!(code, 0);
    let would = plan["would"].as_array().unwrap();
    assert_eq!(would.len(), 1, "{plan}");
    assert_eq!(would[0]["resource"], "settlement_account_roles");
    assert!(!roles.exists());

    let (code, applied, _) = run(&home, &["home", "upgrade", "--apply"]);
    assert_eq!(code, 0);
    assert_eq!(
        applied["created"],
        serde_json::json!(["settlement-account-roles.json"])
    );
    assert!(roles.exists());
    assert_eq!(
        std::fs::read(reference.join("vat-account-roles.json")).unwrap(),
        vat_roles
    );
    assert_eq!(
        std::fs::read(reference.join("chart-of-accounts.json")).unwrap(),
        chart
    );
    let (_, again, _) = run(&home, &["home", "upgrade", "--apply"]);
    assert_eq!(again["changed"], false);
}

/// A home whose `init` wrote the default chart's VAT roles before they had an
/// `eu-goods` reverse-charge target: the upgrade extends exactly that file, and an
/// EU goods receipt then projects instead of refusing.
#[test]
fn home_upgrade_adds_the_eu_goods_target_to_an_unedited_older_role_file() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("esimerkki");
    assert_eq!(run(&home, &["init"]).0, 0);
    let reference = home.join("masterdata").join("reference");
    let older = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "tests/fixtures/main-home-calendar-2026/masterdata/reference/vat-account-roles.json",
    ))
    .unwrap();
    std::fs::write(reference.join("vat-account-roles.json"), &older).unwrap();

    let receipt = r#"{"natural_key":"receipt:fictional-eu-supplier-2026-07-15","actor":"test",
      "source":{"amount":{"amount":"1000.00","currency":"EUR"},"document_date":"2026-07-15",
        "description":"Fictional EU goods purchase"},
      "proposal":{"posting_date":"2026-07-16","net_account":"4000","contra_account":"2700",
        "vat":{"type":"reverse-charge","rate_class":"domestic-standard","scope":"eu-goods"}}}"#;
    let record = |home: &std::path::Path| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_aggountant"))
            .arg("--home")
            .arg(home)
            .args(["--json", "capture-receipt", "record", "--dry-run"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(receipt.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8(out.stderr).unwrap(),
        )
    };
    let (code, stderr) = record(&home);
    assert_ne!(code, 0);
    assert!(
        stderr.contains("eu-goods") && stderr.contains("home upgrade"),
        "{stderr}"
    );

    let (code, plan, _) = run(&home, &["home", "upgrade", "--dry-run"]);
    assert_eq!(code, 0);
    assert_eq!(plan["masterdata_opens"], true);
    let would = plan["would"].as_array().unwrap();
    assert_eq!(would.len(), 1, "{plan}");
    assert_eq!(would[0]["action"], "update");
    assert_eq!(would[0]["resource"], "vat_account_roles");
    assert_eq!(plan["skipped"].as_array().unwrap().len(), 0);
    assert_eq!(
        std::fs::read(reference.join("vat-account-roles.json")).unwrap(),
        older
    );

    let (code, applied, _) = run(&home, &["home", "upgrade", "--apply"]);
    assert_eq!(code, 0);
    assert_eq!(applied["changed"], true);
    assert_eq!(applied["created"], serde_json::json!([]));
    assert_eq!(
        applied["updated"],
        serde_json::json!(["vat-account-roles.json"])
    );

    let (code, stderr) = record(&home);
    assert_eq!(code, 0, "{stderr}");
    let (_, again, _) = run(&home, &["home", "upgrade", "--apply"]);
    assert_eq!(again["changed"], false);
    assert_eq!(again["updated"], serde_json::json!([]));
}
