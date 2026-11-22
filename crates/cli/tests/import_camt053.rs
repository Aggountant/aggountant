//! End-to-end camt.053 tosite-loading through the built `aggountant` binary:
//! `init` → `import camt053 parse` → `import camt053 seal` (record→seal via the
//! existing ledger path) → `voucher verify`, all with `--json`. Also covers
//! dedup (idempotent re-seal of an overlapping statement) and the no-auto-assign
//! refusal.

mod common;

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
    common::feed_stdin(&mut child, stdin.as_bytes());
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
fn deductibility_misplaced_on_a_full_treatment_is_refused_before_sealing() {
    // An agent meaning a 50 % deduction but writing `domestic-input` instead of
    // `domestic-input-partial` must get a refusal, not a sealed full deduction.
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);
    let input = r#"{"assignments":[
      {"bank_ref":"HLV-2026-0314-001","bank_account":"1910","document_date":"2026-03-14",
       "contra_postings":[{"account":"7690","amount":{"amount":"124.00","currency":"EUR"}}],
       "vat_treatment":{"type":"domestic-input","rate_class":"domestic-standard",
        "deductibility":{"numerator":1,"denominator":2,
         "provenance":{"source":"documented-use-allocation","reference":"vehicle-logbook-2026"}}}},
      {"bank_ref":"HLV-2026-0320-002","bank_account":"1910","contra_postings":[
        {"account":"3000","amount":{"amount":"2480.00","currency":"EUR"}}]}
    ]}"#;
    let (code, _, stderr) = run(home, &["import", "camt053", "seal", "--file", &file], input);
    assert_eq!(code, 1, "{stderr}");
    let error: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(error["error_code"], "INVALID_INPUT");
    assert!(error["message"].as_str().unwrap().contains("deductibility"));
    assert_no_fiscal_vouchers(home);
}

#[test]
fn no_vat_with_a_rate_class_or_scope_is_refused_before_sealing() {
    // `no-vat` carrying a rate class or scope contradicts itself and must be refused,
    // not sealed with no VAT leg.
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);
    let input = r#"{"assignments":[
      {"bank_ref":"HLV-2026-0314-001","bank_account":"1910","document_date":"2026-03-14",
       "contra_postings":[{"account":"7690","amount":{"amount":"124.00","currency":"EUR"}}],
       "vat_treatment":{"type":"no-vat","rate_class":"domestic-standard","scope":"eu-goods"}},
      {"bank_ref":"HLV-2026-0320-002","bank_account":"1910","contra_postings":[
        {"account":"3000","amount":{"amount":"2480.00","currency":"EUR"}}]}
    ]}"#;
    for args in [
        &["import", "camt053", "seal", "--file", &file, "--dry-run"][..],
        &["import", "camt053", "seal", "--file", &file][..],
    ] {
        let (code, _, stderr) = run(home, args, input);
        assert_eq!(code, 1, "{stderr}");
        let error: Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(error["error_code"], "INVALID_INPUT");
        assert!(error["message"].as_str().unwrap().contains("rate_class"));
    }
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

/// Every `*.blob` under the home's artifact store.
fn artifact_blobs(home: &std::path::Path) -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "blob") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&home.join("artifacts"), &mut out);
    out
}

#[test]
fn parse_reports_the_statement_period_and_balances() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let file = write_sample(home);
    let (code, stdout, stderr) = run(home, &["import", "camt053", "parse", "--file", &file], "");
    assert_eq!(code, 0, "{stderr}");
    let parsed: Value = serde_json::from_str(&stdout).unwrap();
    let statement = &parsed["statements"][0];
    assert_eq!(statement["period"]["from"], "2026-03-01");
    assert_eq!(statement["period"]["to"], "2026-03-31");
    assert_eq!(statement["opening_balance"]["amount"]["amount"], "1000.00");
    assert_eq!(statement["closing_balance"]["type"], "CLBD");
    assert_eq!(statement["closing_balance"]["amount"]["amount"], "3356.00");
    assert_eq!(statement["entries_net"]["amount"], "2356.00");
    assert_eq!(statement["balances_reconcile"], true);
}

#[test]
fn dry_run_previews_every_voucher_and_writes_nothing() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);

    let (code, stdout, stderr) = run(
        home,
        &["import", "camt053", "seal", "--file", &file, "--dry-run"],
        ASSIGNMENTS,
    );
    assert_eq!(code, 0, "dry-run failed: {stderr}");
    let preview: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(preview["dry_run"], true);
    assert_eq!(preview["voucher_count"], 2);
    assert_eq!(preview["actor"], "importer");
    assert_eq!(
        preview["statements"][0]["closing_balance"]["amount"]["amount"],
        "3356.00"
    );
    let hash = preview["preparation_hash"].as_str().unwrap().to_string();
    assert!(hash.starts_with("sha256:v1:"), "{hash}");
    let vouchers = preview["vouchers"].as_array().unwrap();
    assert_eq!(vouchers[0]["bank_ref"], "HLV-2026-0314-001");
    assert_eq!(vouchers[0]["would_replay"], false);
    // The domestic-input treatment is expanded in the preview: bank, net and VAT legs.
    assert_eq!(
        vouchers[0]["draft"]["postings"].as_array().unwrap().len(),
        3
    );
    assert_eq!(
        vouchers[0]["draft"]["attachment_ref"].as_str().unwrap(),
        preview["attachment_ref"].as_str().unwrap()
    );
    assert_no_fiscal_vouchers(home);
    assert!(
        artifact_blobs(home).is_empty(),
        "dry-run attached the statement"
    );

    // The same input previews to the same hash, and seals with it.
    let (_, stdout, _) = run(
        home,
        &["import", "camt053", "seal", "--file", &file, "--dry-run"],
        ASSIGNMENTS,
    );
    let again: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(again["preparation_hash"], hash.as_str());

    let (code, stdout, stderr) = run(
        home,
        &[
            "import",
            "camt053",
            "seal",
            "--file",
            &file,
            "--approved-preparation-hash",
            &hash,
        ],
        ASSIGNMENTS,
    );
    assert_eq!(code, 0, "approved seal failed: {stderr}");
    let sealed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(sealed["dry_run"], false);
    assert_eq!(sealed["created_count"], 2);
    assert_eq!(sealed["preparation_hash"], hash.as_str());
    assert_eq!(sealed["attachment_ref"], preview["attachment_ref"]);

    // After the seal, a preview of the same statement shows the replay.
    let (code, stdout, stderr) = run(
        home,
        &["import", "camt053", "seal", "--file", &file, "--dry-run"],
        ASSIGNMENTS,
    );
    assert_eq!(code, 0, "{stderr}");
    let replay: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(replay["vouchers"][0]["would_replay"], true);
    assert_eq!(
        replay["vouchers"][0]["warnings"][0]["code"],
        "NATURAL_KEY_ALREADY_SEALED"
    );
    assert_eq!(replay["preparation_hash"], hash.as_str());
}

#[test]
fn a_changed_assignment_refuses_the_approved_seal_before_any_write() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);
    let (_, stdout, _) = run(
        home,
        &["import", "camt053", "seal", "--file", &file, "--dry-run"],
        ASSIGNMENTS,
    );
    let preview: Value = serde_json::from_str(&stdout).unwrap();
    let hash = preview["preparation_hash"].as_str().unwrap();

    let changed = ASSIGNMENTS.replace("\"3000\"", "\"3010\"");
    for input in [
        changed.as_str(),
        &ASSIGNMENTS.replace("importer", "someone-else"),
    ] {
        let (code, stdout, stderr) = run(
            home,
            &[
                "import",
                "camt053",
                "seal",
                "--file",
                &file,
                "--approved-preparation-hash",
                hash,
            ],
            input,
        );
        assert_eq!(code, 1, "{stdout}");
        let error: Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(error["error_code"], "IMPORT_PREPARATION_CHANGED");
        assert_eq!(error["error_kind"], "user");
    }
    assert_no_fiscal_vouchers(home);
    assert!(artifact_blobs(home).is_empty());
}

#[test]
fn dry_run_refuses_an_unknown_account_as_the_seal_would() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);
    let input = ASSIGNMENTS.replace("\"3000\"", "\"9999\"");
    let (code, stdout, stderr) = run(
        home,
        &["import", "camt053", "seal", "--file", &file, "--dry-run"],
        &input,
    );
    assert_eq!(code, 1, "{stdout}");
    let error: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(error["error_code"], "ACCOUNT_NOT_FOUND");
    assert!(artifact_blobs(home).is_empty());
}

#[test]
fn dry_run_and_an_approval_hash_together_are_a_usage_error() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let file = write_sample(home);
    let (code, _, stderr) = run(
        home,
        &[
            "import",
            "camt053",
            "seal",
            "--file",
            &file,
            "--dry-run",
            "--approved-preparation-hash",
            "sha256:v1:00",
        ],
        ASSIGNMENTS,
    );
    assert_eq!(code, 1, "a usage error is caller-actionable");
    assert!(stderr.contains("approved-preparation-hash"), "{stderr}");
}

#[test]
fn dry_run_refuses_a_changed_reimport_as_the_seal_would() {
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

    // The second entry is already sealed to 3000; a different account is a
    // reused natural key with different content, refused before any write.
    let changed = ASSIGNMENTS.replace("\"3000\"", "\"3010\"");
    for extra in [&["--dry-run"][..], &[][..]] {
        let mut args = vec!["import", "camt053", "seal", "--file", &file];
        args.extend_from_slice(extra);
        let (code, stdout, stderr) = run(home, &args, &changed);
        assert_eq!(code, 1, "{stdout}");
        let error: Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(error["error_code"], "IDEMPOTENCY_KEY_REUSE");
    }
    let (_, stdout, _) = run(home, &["voucher", "verify"], "");
    let verify: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(verify["voucher_count"], 2);
}

#[test]
fn domestic_input_with_an_eu_class_is_refused_before_sealing() {
    // A bank-line assignment feeds the same expansion as capture-receipt: an EU
    // purchase written as domestic-input + eu-goods would seal with no VAT legs.
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);
    let input = r#"{"assignments":[
      {"bank_ref":"HLV-2026-0314-001","bank_account":"1910","document_date":"2026-03-14",
       "contra_postings":[{"account":"7690","amount":{"amount":"124.00","currency":"EUR"}}],
       "vat_treatment":{"type":"domestic-input","rate_class":"eu-goods"}},
      {"bank_ref":"HLV-2026-0320-002","bank_account":"1910","contra_postings":[
        {"account":"3000","amount":{"amount":"2480.00","currency":"EUR"}}]}
    ]}"#;
    for args in [
        &["import", "camt053", "seal", "--dry-run", "--file", &file][..],
        &["import", "camt053", "seal", "--file", &file][..],
    ] {
        let (code, _, stderr) = run(home, args, input);
        assert_eq!(code, 1, "{stderr}");
        let error: Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(error["error_code"], "VAT_TREATMENT_CLASS_MISMATCH");
        assert!(error["message"]
            .as_str()
            .unwrap()
            .contains(r#""scope":"eu-goods""#));
        assert!(error["hint"].is_string(), "{error}");
    }
    assert_no_fiscal_vouchers(home);
}

#[test]
fn exempt_eu_goods_acquisition_reaches_field_312_with_zero_tax_from_bank_and_receipt() {
    // An intra-Community acquisition of goods that is exempt in Finland is reported
    // only in "Tavaraostot muista EU-maista" (312), with no tax and no deduction.
    // Both intake routes record it, and the return preview moves only 312 (305 is
    // reported as zero; 307 and 308 stay zero).
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);
    let exempt = r#"{"type":"reverse-charge","rate_class":"zero-rated","scope":"eu-goods"}"#;
    let input = format!(
        r#"{{"actor":"importer","assignments":[
      {{"bank_ref":"HLV-2026-0314-001","bank_account":"1910","document_date":"2026-03-14",
       "contra_postings":[{{"account":"4000","amount":{{"amount":"124.00","currency":"EUR"}}}}],
       "vat_treatment":{exempt}}},
      {{"bank_ref":"HLV-2026-0320-002","bank_account":"1910","contra_postings":[
        {{"account":"3000","amount":{{"amount":"2480.00","currency":"EUR"}}}}]}}
    ]}}"#
    );
    let (code, _, stderr) = run(
        home,
        &["import", "camt053", "seal", "--file", &file],
        &input,
    );
    assert_eq!(code, 0, "{stderr}");

    let receipt = format!(
        r#"{{"natural_key":"receipt:fictional-eu-goods-1","actor":"cli-test-agent",
      "source":{{"amount":{{"amount":"356.00","currency":"EUR"}},"document_date":"2026-03-20",
        "description":"Fictional exempt goods from another EU state"}},
      "proposal":{{"posting_date":"2026-03-20","net_account":"4000","contra_account":"1910",
        "vat":{exempt}}}}}"#
    );
    let (code, stdout, stderr) = run(home, &["capture-receipt", "record", "--dry-run"], &receipt);
    assert_eq!(code, 0, "{stderr}");
    let preview: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(preview["projection_valid"], true, "{preview}");
    let hash = preview["preparation_hash"].as_str().unwrap();
    let (code, _, stderr) = run(
        home,
        &[
            "capture-receipt",
            "record",
            "--approved-preparation-hash",
            hash,
        ],
        &receipt,
    );
    assert_eq!(code, 0, "{stderr}");

    let (code, stdout, stderr) = run(home, &["voucher", "verify"], "");
    assert_eq!(code, 0, "{stderr}");
    let verify: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(verify["voucher_count"], 3, "{verify}");

    let (code, stdout, stderr) = run(home, &["file-vat", "preview", "--period", "2026-03"], "");
    assert_eq!(code, 0, "{stderr}");
    let preview: Value = serde_json::from_str(&stdout).unwrap();
    let fields = preview["filing_fields"].as_object().unwrap();
    let amounts: Vec<(&str, &str)> = fields
        .iter()
        .map(|(field, money)| (field.as_str(), money["amount"].as_str().unwrap()))
        .collect();
    assert_eq!(
        amounts,
        vec![
            ("305", "0.00"),
            ("307", "0.00"),
            ("308", "0.00"),
            ("312", "480.00")
        ],
        "{preview}"
    );
    assert_eq!(preview["sales_vat"], "0.00");
    assert_eq!(preview["purchase_vat"], "0.00");
    assert_eq!(preview["vat_reconciliation"]["status"], "reconciled");
}
