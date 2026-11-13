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
    std::fs::remove_dir_all(reference.join("chart-snapshots")).unwrap();

    let (code, _, stderr) = run(&home, &["home", "upgrade"]);
    assert_eq!(code, 1, "a mode is required: {stderr}");

    let (code, plan, _) = run(&home, &["home", "upgrade", "--dry-run"]);
    assert_eq!(code, 0);
    assert_eq!(plan["schema_version"], 1);
    assert_eq!(plan["dry_run"], true);
    assert_eq!(plan["changed"], false);
    assert_eq!(plan["masterdata_opens"], true);
    let would = plan["would"].as_array().unwrap();
    assert_eq!(would.len(), 2);
    assert_eq!(would[1]["resource"], "vat_account_roles");
    assert!(!reference.join("vat-account-roles.json").exists());
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
    assert_eq!(again["existed"].as_array().unwrap().len(), 2);

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
