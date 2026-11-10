//! End-to-end camt.053 tosite-loading through the built `aggountant` binary:
//! `init` → `import camt053 parse` → `import camt053 seal` (record→seal via the
//! existing ledger path) → `voucher verify`, all with `--json`. Also covers
//! dedup (idempotent re-seal of an overlapping statement) and the no-auto-assign
//! refusal.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

/// The same fixture the import crate's golden tests use (single source of truth).
const SAMPLE: &str = include_str!("../../import/tests/fixtures/holvi-camt053-sample.xml");

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
}

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

/// Write the fixture into the home and return its path (as a String arg).
fn write_sample(home: &std::path::Path) -> String {
    let path = home.join("holvi-2026-03.xml");
    std::fs::write(&path, SAMPLE).unwrap();
    path.to_str().unwrap().to_string()
}

/// The skill's account assignments for the two booked entries in the fixture.
const ASSIGNMENTS: &str = r#"{
  "actor": "importer",
  "assignments": [
    {"bank_ref":"HLV-2026-0314-001","bank_account":"1910","document_date":"2026-03-14","contra_postings":[
      {"account":"7690","amount":{"amount":"124.00","currency":"EUR"}}
    ],"vat_treatment":{"type":"domestic-input","rate_class":"domestic-standard"}},
    {"bank_ref":"HLV-2026-0320-002","bank_account":"1910","contra_postings":[
      {"account":"3000","amount":{"amount":"2480.00","currency":"EUR"}}
    ]}
  ]
}"#;

#[test]
fn default_chart_accepts_reverse_charge_with_chart_owned_shared_roles() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);
    let input = r#"{
      "actor":"importer",
      "assignments":[
        {"bank_ref":"HLV-2026-0314-001","bank_account":"1910","document_date":"2026-03-14","contra_postings":[
          {"account":"7690","amount":{"amount":"124.00","currency":"EUR"}}
        ],"vat_treatment":{"type":"reverse-charge","rate_class":"domestic-standard","scope":"eu-services"}},
        {"bank_ref":"HLV-2026-0320-002","bank_account":"1910","contra_postings":[
          {"account":"3000","amount":{"amount":"2480.00","currency":"EUR"}}
        ]}
      ]
    }"#;
    let (code, stdout, stderr) = run(home, &["import", "camt053", "seal", "--file", &file], input);
    assert_eq!(code, 0, "seal failed: {stderr}");
    let sealed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(sealed["sealed_count"], 2);
    assert_eq!(sealed["created_count"], 2);
}

#[test]
fn camt053_parse_then_seal_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);

    // 1. parse — deterministic, no accounts assigned.
    let (code, stdout, _) = run(home, &["import", "camt053", "parse", "--file", &file], "");
    assert_eq!(code, 0);
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["account_iban"], "FI2112345600000785");
    let txns = parsed["transactions"].as_array().unwrap();
    assert_eq!(txns.len(), 2, "the PDNG entry is not booked");
    // The parse output carries no account fields (judgment is the skill's).
    assert!(txns[0].get("bank_account").is_none());
    assert!(txns[0].get("contra_account").is_none());

    // 2. seal — the skill supplies accounts on stdin; record→seal each entry.
    let (code, stdout, stderr) = run(
        home,
        &["import", "camt053", "seal", "--file", &file],
        ASSIGNMENTS,
    );
    assert_eq!(code, 0, "seal failed: {stderr}");
    let sealed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(sealed["sealed_count"], 2);
    assert_eq!(sealed["created_count"], 2);
    assert_eq!(sealed["replayed_count"], 0);
    let vouchers = sealed["sealed"].as_array().unwrap();
    assert_eq!(vouchers[0]["number"], 1);
    assert_eq!(vouchers[0]["replayed"], false);
    assert_eq!(vouchers[1]["number"], 2);
    // The original XML is retained as the attachment for the batch (KPL 2:5).
    assert_eq!(sealed["attachment_ref"].as_str().unwrap().len(), 64);

    // 3. verify — the chain is intact with the two imported vouchers.
    let (code, stdout, _) = run(home, &["voucher", "verify"], "");
    assert_eq!(code, 0);
    let verify: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(verify["ok"], true);
    assert_eq!(verify["voucher_count"], 2);
}

#[test]
fn re_sealing_an_overlapping_statement_dedups_via_natural_key() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);

    let (code, _, stderr) = run(
        home,
        &["import", "camt053", "seal", "--file", &file],
        ASSIGNMENTS,
    );
    assert_eq!(code, 0, "{stderr}");

    // Re-import the same statement range: each entry's natural_key replays, so no
    // new vouchers are minted — overlapping ranges collapse for free.
    let (code, stdout, stderr) = run(
        home,
        &["import", "camt053", "seal", "--file", &file],
        ASSIGNMENTS,
    );
    assert_eq!(code, 0, "{stderr}");
    let sealed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(sealed["sealed_count"], 2);
    assert_eq!(sealed["created_count"], 0, "no new vouchers on re-import");
    assert_eq!(sealed["replayed_count"], 2);
    for v in sealed["sealed"].as_array().unwrap() {
        assert_eq!(v["replayed"], true, "same natural_key replays");
    }

    let (_, stdout, _) = run(home, &["voucher", "verify"], "");
    let verify: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(verify["voucher_count"], 2, "no double-posting");
}

#[test]
fn seal_refuses_an_entry_with_no_assignment() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);

    // Only assign one of the two booked entries — the CLI must not guess the other.
    let partial = r#"{"assignments":[
      {"bank_ref":"HLV-2026-0314-001","bank_account":"1910","contra_postings":[
        {"account":"7690","amount":{"amount":"124.00","currency":"EUR"}}
      ]}
    ]}"#;
    let (code, stdout, stderr) = run(
        home,
        &["import", "camt053", "seal", "--file", &file],
        partial,
    );
    assert_eq!(code, 1, "user error");
    assert!(stdout.is_empty());
    let err: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err["error_code"], "MISSING_ASSIGNMENT");
}

#[test]
fn parse_of_a_missing_caller_file_is_a_user_error() {
    let home = tempfile::tempdir().unwrap();
    let (code, _stdout, stderr) = run(
        home.path(),
        &["import", "camt053", "parse", "--file", "/no/such/file.xml"],
        "",
    );
    assert_eq!(code, 1, "caller-actionable not-found exit code");
    let err: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(err["error_code"], "FILE_NOT_FOUND");
    assert_eq!(err["error_kind"], "user");
}

fn assert_no_fiscal_vouchers(home: &std::path::Path) {
    let (code, stdout, stderr) = run(home, &["voucher", "verify"], "");
    assert_eq!(code, 0, "verify failed: {stderr}");
    let verify: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(verify["voucher_count"], 0);
}

#[test]
fn taxable_legacy_gross_129_is_refused_before_any_fiscal_write() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let xml = SAMPLE.replacen(">124.00</Amt>", ">129.00</Amt>", 1);
    let file = home.join("fictional-legacy-129.xml");
    std::fs::write(&file, xml).unwrap();
    let input = r#"{"assignments":[
      {"bank_ref":"HLV-2026-0314-001","bank_account":"1910","document_date":"2026-03-14","contra_postings":[
        {"account":"7690","amount":{"amount":"129.00","currency":"EUR"},"vat_code":"purchase-domestic-standard"}]},
      {"bank_ref":"HLV-2026-0320-002","bank_account":"1910","contra_postings":[
        {"account":"3000","amount":{"amount":"2480.00","currency":"EUR"}}]}
    ]}"#;
    let (code, stdout, stderr) = run(
        home,
        &[
            "import",
            "camt053",
            "seal",
            "--file",
            file.to_str().unwrap(),
        ],
        input,
    );
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    let error: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(error["error_code"], "UNSAFE_LEGACY_VAT_CODE");
    assert_eq!(error["error_kind"], "user");
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("contra_postings[0].vat_code"));
    assert_no_fiscal_vouchers(home);
}

#[test]
fn taxable_sales_tag_in_last_entry_rejects_the_whole_mixed_batch() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);
    let input = r#"{"assignments":[
      {"bank_ref":"HLV-2026-0314-001","bank_account":"1910","contra_postings":[
        {"account":"7690","amount":{"amount":"124.00","currency":"EUR"}}]},
      {"bank_ref":"HLV-2026-0320-002","bank_account":"1910","document_date":"2026-03-20","contra_postings":[
        {"account":"3000","amount":{"amount":"2480.00","currency":"EUR"},"vat_code":"sales-domestic-standard"}]}
    ]}"#;
    let (code, _, stderr) = run(home, &["import", "camt053", "seal", "--file", &file], input);
    assert_eq!(code, 1);
    let error: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(error["error_code"], "UNSAFE_LEGACY_VAT_CODE");
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("sales VAT expansion is unsupported"));
    assert_no_fiscal_vouchers(home);
}

#[test]
fn vat_treatment_without_document_date_is_a_stable_batch_refusal() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);
    let input = r#"{"assignments":[
      {"bank_ref":"HLV-2026-0314-001","bank_account":"1910","contra_postings":[
        {"account":"7690","amount":{"amount":"124.00","currency":"EUR"}}],
       "vat_treatment":{"type":"domestic-input","rate_class":"domestic-standard"}},
      {"bank_ref":"HLV-2026-0320-002","bank_account":"1910","contra_postings":[
        {"account":"3000","amount":{"amount":"2480.00","currency":"EUR"}}]}
    ]}"#;
    let (code, _, stderr) = run(home, &["import", "camt053", "seal", "--file", &file], input);
    assert_eq!(code, 1);
    let error: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(error["error_code"], "MISSING_DOCUMENT_DATE");
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("field document_date"));
    assert_no_fiscal_vouchers(home);
}

#[test]
fn zero_rate_legacy_tag_remains_a_supported_transport_path() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);
    let input = r#"{"assignments":[
      {"bank_ref":"HLV-2026-0314-001","bank_account":"1910","contra_postings":[
        {"account":"7690","amount":{"amount":"124.00","currency":"EUR"},"vat_code":"purchase-zero-rated"}]},
      {"bank_ref":"HLV-2026-0320-002","bank_account":"1910","contra_postings":[
        {"account":"3000","amount":{"amount":"2480.00","currency":"EUR"}}]}
    ]}"#;
    let (code, stdout, stderr) = run(home, &["import", "camt053", "seal", "--file", &file], input);
    assert_eq!(code, 0, "{stderr}");
    let output: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(output["created_count"], 2);
}
