//! End-to-end A1 light path through the built `aggountant` binary:
//! `chart list` → `voucher draft` → `voucher validate` → `artifact attach` →
//! `voucher seal` → `voucher verify`, all with `--json`.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
}

/// Run the CLI with `args`, feeding `stdin`, returning (exit_code, stdout, stderr).
fn run(home: &std::path::Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut cmd = bin();
    cmd.arg("--home").arg(home).arg("--json");
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

#[test]
fn a1_light_path_end_to_end() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();

    // 0. init (write the git-native reference masterdata)
    let (code, stdout, _) = run(home, &["init"], "");
    assert_eq!(code, 0);
    let init: Value = serde_json::from_str(&stdout).unwrap();
    assert!(init["chart_version"]
        .as_str()
        .unwrap()
        .starts_with("fi-general-2026@"));
    assert!(init["account_count"].as_u64().unwrap() > 50);

    // 1. chart list
    let (code, stdout, _) = run(home, &["chart", "list"], "");
    assert_eq!(code, 0);
    let chart: Value = serde_json::from_str(&stdout).unwrap();
    assert!(chart["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|a| a["id"] == "7690"));

    // 2. artifact attach (write a fake receipt file first)
    let receipt = home.join("receipt.pdf");
    std::fs::write(&receipt, b"%PDF-1.4 fake").unwrap();
    let (code, stdout, _) = run(
        home,
        &["artifact", "attach", "--file", receipt.to_str().unwrap()],
        "",
    );
    assert_eq!(code, 0);
    let att: Value = serde_json::from_str(&stdout).unwrap();
    let artifact_ref = att["artifact_ref"].as_str().unwrap().to_string();

    // 3. voucher draft (stdin JSON → validated draft JSON), with the attachment
    let draft_input = format!(
        r#"{{"document_date":"2026-03-14","posting_date":"2026-03-14",
             "description":"Hosting invoice",
             "attachment_ref":"{artifact_ref}",
             "postings":[
               {{"account":"7690","amount":{{"amount":"124.00","currency":"EUR"}},"side":"debit"}},
               {{"account":"1910","amount":{{"amount":"124.00","currency":"EUR"}},"side":"credit"}}
             ]}}"#
    );
    let (code, draft_json, _) = run(home, &["voucher", "draft"], &draft_input);
    assert_eq!(code, 0);
    let draft: Value = serde_json::from_str(&draft_json).unwrap();
    assert_eq!(draft["postings"].as_array().unwrap().len(), 2);

    // 4. voucher validate (dry-run)
    let (code, stdout, _) = run(home, &["voucher", "validate"], &draft_json);
    assert_eq!(code, 0);
    let report: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["attachment_present"], true);

    // 5. voucher seal
    let seal_input =
        format!(r#"{{"draft":{draft_json},"natural_key":"acme:receipt-42","actor":"tester"}}"#);
    let (code, stdout, _) = run(home, &["voucher", "seal"], &seal_input);
    assert_eq!(code, 0);
    let sealed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(sealed["number"], 1);
    assert_eq!(sealed["replayed"], false);
    let hash1 = sealed["hash"].as_str().unwrap().to_string();

    // 5b. idempotent replay on the same natural_key
    let (code, stdout, _) = run(home, &["voucher", "seal"], &seal_input);
    assert_eq!(code, 0);
    let replay: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["hash"].as_str().unwrap(), hash1);

    // 6. voucher verify
    let (code, stdout, _) = run(home, &["voucher", "verify"], "");
    assert_eq!(code, 0);
    let verify: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(verify["ok"], true);
    assert_eq!(verify["voucher_count"], 1);
}

#[test]
fn eu_sale_buyer_vat_id_survives_seal_and_invalid_input_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);

    let draft = r#"{"document_date":"2026-05-31","posting_date":"2026-05-31",
        "description":"EU goods sale","buyer_vat_id":"se 556677889901",
        "postings":[
          {"account":"1910","amount":{"amount":"100.00","currency":"EUR"},"side":"debit"},
          {"account":"3000","amount":{"amount":"100.00","currency":"EUR"},"side":"credit","vat_code":"sales-eu-goods"}
        ]}"#;
    let seal_input = format!(r#"{{"draft":{draft},"natural_key":"eu-sale:1","actor":"tester"}}"#);
    let (code, stdout, stderr) = run(home, &["voucher", "seal"], &seal_input);
    assert_eq!(code, 0, "{stderr}");
    let sealed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(sealed["number"], 1);

    let (code, stdout, stderr) = run(home, &["voucher", "show", "1"], "");
    assert_eq!(code, 0, "{stderr}");
    let shown: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(shown["body"]["buyer_vat_id"], "SE556677889901");

    let invalid = draft.replace("se 556677889901", "not-a-vat-id");
    let invalid_seal =
        format!(r#"{{"draft":{invalid},"natural_key":"eu-sale:bad","actor":"tester"}}"#);
    let (code, stdout, stderr) = run(home, &["voucher", "seal"], &invalid_seal);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    let error: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(error["error_code"], "VAT_BUYER_ID");

    let (code, stdout, stderr) = run(home, &["voucher", "verify"], "");
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["voucher_count"],
        1
    );
}

#[test]
fn malformed_stdin_is_a_user_error_with_code() {
    let home = tempfile::tempdir().unwrap();
    let (code, stdout, stderr) = run(home.path(), &["voucher", "draft"], "{ not json");
    assert_eq!(code, 1, "user error exit code");
    assert!(stdout.is_empty(), "no data on stdout for an error");
    let err: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err["error_code"], "INVALID_INPUT");
    assert_eq!(err["error_kind"], "user");
}

#[test]
fn unbalanced_draft_reports_model_error_code() {
    let home = tempfile::tempdir().unwrap();
    let input = r#"{"document_date":"2026-03-14","posting_date":"2026-03-14",
        "description":"bad",
        "postings":[
          {"account":"7690","amount":{"amount":"124.00","currency":"EUR"},"side":"debit"},
          {"account":"1910","amount":{"amount":"100.00","currency":"EUR"},"side":"credit"}
        ]}"#;
    let (code, _stdout, stderr) = run(home.path(), &["voucher", "draft"], input);
    assert_eq!(code, 1);
    let err: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err["error_code"], "UNBALANCED_VOUCHER");
}

#[test]
fn init_writes_a_git_diffable_masterdata_tree() {
    let home = tempfile::tempdir().unwrap();
    let (code, _stdout, _) = run(home.path(), &["init"], "");
    assert_eq!(code, 0);

    // The reference masterdata lands as plaintext under the company home.
    let reference = home.path().join("masterdata").join("reference");
    let chart = std::fs::read_to_string(reference.join("chart-of-accounts.json")).unwrap();
    assert!(
        chart.contains("\"1910\""),
        "real chart includes the bank account"
    );
    assert!(chart.contains("Atk-laite- ja ohjelmistokulut"));
    // chart_version is a plaintext file, and the README documents the tree.
    let version = std::fs::read_to_string(reference.join("chart_version")).unwrap();
    assert!(version.trim().starts_with("fi-general-2026@"));
    assert!(reference.join("README.md").exists());
    // No PII pattern (IBAN/HETU) in the plaintext (ADR-1 canary).
    assert!(
        !chart.contains(" FI"),
        "no IBAN-like token in the chart bytes"
    );
}

#[test]
fn re_init_refuses_to_clobber() {
    let home = tempfile::tempdir().unwrap();
    let (code, _, _) = run(home.path(), &["init"], "");
    assert_eq!(code, 0);

    let (code, stdout, stderr) = run(home.path(), &["init"], "");
    assert_eq!(code, 1, "re-init is a user error, not a clobber");
    assert!(stdout.is_empty());
    let err: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err["error_code"], "ALREADY_INITIALIZED");
}

#[test]
fn commands_before_init_fail_clearly() {
    let home = tempfile::tempdir().unwrap();
    let (code, _stdout, stderr) = run(home.path(), &["chart", "list"], "");
    assert_eq!(code, 1);
    let err: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err["error_code"], "MASTERDATA_NOT_INITIALIZED");
}
