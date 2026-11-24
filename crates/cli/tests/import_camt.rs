//! End-to-end camt.053 / camt.052 tosite-loading through the built `aggountant` binary:
//! `init` → `import camt parse` → `import camt seal` (record→seal via the
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
    let (code, stdout, stderr) = run(home, &["import", "camt", "seal", "--file", &file], input);
    assert_eq!(code, 0, "seal failed: {stderr}");
    let sealed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(sealed["sealed_count"], 2);
    assert_eq!(sealed["created_count"], 2);
}

#[test]
fn camt_parse_then_seal_round_trip() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);

    // 1. parse — deterministic, no accounts assigned.
    let (code, stdout, _) = run(home, &["import", "camt", "parse", "--file", &file], "");
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
        &["import", "camt", "seal", "--file", &file],
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
        &["import", "camt", "seal", "--file", &file],
        ASSIGNMENTS,
    );
    assert_eq!(code, 0, "{stderr}");

    // Re-import the same statement range: each entry's natural_key replays, so no
    // new vouchers are minted — overlapping ranges collapse for free.
    let (code, stdout, stderr) = run(
        home,
        &["import", "camt", "seal", "--file", &file],
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
    let (code, stdout, stderr) = run(home, &["import", "camt", "seal", "--file", &file], partial);
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
        &["import", "camt", "parse", "--file", "/no/such/file.xml"],
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
        &["import", "camt", "seal", "--file", file.to_str().unwrap()],
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
    let (code, _, stderr) = run(home, &["import", "camt", "seal", "--file", &file], input);
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
    let (code, _, stderr) = run(home, &["import", "camt", "seal", "--file", &file], input);
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
    let (code, _, stderr) = run(home, &["import", "camt", "seal", "--file", &file], input);
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
        &["import", "camt", "seal", "--file", &file, "--dry-run"][..],
        &["import", "camt", "seal", "--file", &file][..],
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
    let (code, stdout, stderr) = run(home, &["import", "camt", "seal", "--file", &file], input);
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
    let (code, stdout, stderr) = run(home, &["import", "camt", "parse", "--file", &file], "");
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
        &["import", "camt", "seal", "--file", &file, "--dry-run"],
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
        &["import", "camt", "seal", "--file", &file, "--dry-run"],
        ASSIGNMENTS,
    );
    let again: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(again["preparation_hash"], hash.as_str());

    let (code, stdout, stderr) = run(
        home,
        &[
            "import",
            "camt",
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
        &["import", "camt", "seal", "--file", &file, "--dry-run"],
        ASSIGNMENTS,
    );
    assert_eq!(code, 0, "{stderr}");
    let replay: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(replay["vouchers"][0]["would_replay"], true);
    assert_eq!(
        replay["vouchers"][0]["outcome"],
        "duplicate_source_observation"
    );
    assert_eq!(replay["vouchers"][0]["voucher_number"], 1);
    assert_eq!(replay["new_voucher_count"], 0);
    assert_eq!(replay["duplicate_observation_count"], 2);
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
        &["import", "camt", "seal", "--file", &file, "--dry-run"],
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
                "camt",
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
        &["import", "camt", "seal", "--file", &file, "--dry-run"],
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
            "camt",
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
        &["import", "camt", "seal", "--file", &file],
        ASSIGNMENTS,
    );
    assert_eq!(code, 0, "{stderr}");

    // The second entry is already sealed to 3000; a different account is the
    // same bank transaction booked differently, refused before any write.
    let changed = ASSIGNMENTS.replace("\"3000\"", "\"3010\"");
    for extra in [&["--dry-run"][..], &[][..]] {
        let mut args = vec!["import", "camt", "seal", "--file", &file];
        args.extend_from_slice(extra);
        let (code, stdout, stderr) = run(home, &args, &changed);
        assert_eq!(code, 1, "{stdout}");
        let error: Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(error["error_code"], "POSTING_PROJECTION_CHANGED");
        assert_eq!(error["details"]["conflict_count"], 1);
        assert_eq!(error["details"]["duplicate_observation_count"], 1);
        assert_eq!(error["details"]["conflicts"][0]["voucher_number"], 2);
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
        &["import", "camt", "seal", "--dry-run", "--file", &file][..],
        &["import", "camt", "seal", "--file", &file][..],
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
    let (code, _, stderr) = run(home, &["import", "camt", "seal", "--file", &file], &input);
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

// ---------------------------------------------------------------------------
// Event identity across overlapping exports (issue `camt-event-identity`).
// ---------------------------------------------------------------------------

const IBAN: &str = "FI2112345600000785";

/// One synthetic statement entry: bank reference, booking date, amount,
/// `DBIT`/`CRDT` and status.
#[derive(Clone)]
struct Entry {
    bank_ref: &'static str,
    date: &'static str,
    amount: &'static str,
    direction: &'static str,
    status: &'static str,
}

const fn entry(bank_ref: &'static str, date: &'static str, amount: &'static str) -> Entry {
    Entry {
        bank_ref,
        date,
        amount,
        direction: "DBIT",
        status: "BOOK",
    }
}

/// A camt.053 export of `entries` with a statement id, so two exports of the
/// same transactions are different files.
fn export(id: &str, entries: &[Entry]) -> String {
    let ntry: String = entries
        .iter()
        .map(|e| {
            format!(
                r#"<Ntry><Amt Ccy="EUR">{}</Amt><CdtDbtInd>{}</CdtDbtInd><Sts>{}</Sts>
<BookgDt><Dt>{}</Dt></BookgDt><AcctSvcrRef>{}</AcctSvcrRef>
<NtryDtls><TxDtls><RltdPties><Cdtr><Nm>Toimittaja Oy</Nm></Cdtr></RltdPties>
<RmtInf><Ustrd>Lasku {}</Ustrd></RmtInf></TxDtls></NtryDtls></Ntry>
"#,
                e.amount, e.direction, e.status, e.date, e.bank_ref, e.bank_ref
            )
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.02"><BkToCstmrStmt>
<GrpHdr><MsgId>{id}</MsgId><CreDtTm>2027-01-02T06:00:00</CreDtTm></GrpHdr>
<Stmt><Id>{id}</Id><Acct><Id><IBAN>{IBAN}</IBAN></Id><Ccy>EUR</Ccy></Acct>
{ntry}</Stmt></BkToCstmrStmt></Document>
"#
    )
}

/// Assignments for the booked `entries`: money out to 7690, the given account
/// for one bank reference when `override_ref` names it.
fn assignments(entries: &[Entry], override_ref: Option<(&str, &str)>) -> String {
    let rows: Vec<String> = entries
        .iter()
        .filter(|e| e.status == "BOOK")
        .map(|e| {
            let account = match override_ref {
                Some((bank_ref, account)) if bank_ref == e.bank_ref => account,
                _ => "7690",
            };
            format!(
                r#"{{"bank_ref":"{}","bank_account":"1910","contra_postings":[{{"account":"{account}","amount":{{"amount":"{}","currency":"EUR"}}}}]}}"#,
                e.bank_ref, e.amount
            )
        })
        .collect();
    format!(
        r#"{{"actor":"importer","assignments":[{}]}}"#,
        rows.join(",")
    )
}

fn write_export(home: &std::path::Path, name: &str, xml: &str) -> String {
    let path = home.join(name);
    std::fs::write(&path, xml).unwrap();
    path.to_str().unwrap().to_string()
}

fn seal(home: &std::path::Path, file: &str, input: &str) -> (i32, Value, Value) {
    let (code, stdout, stderr) = run(home, &["import", "camt", "seal", "--file", file], input);
    let parse = |s: &str| serde_json::from_str(s).unwrap_or(Value::Null);
    (code, parse(&stdout), parse(&stderr))
}

fn voucher_count(home: &std::path::Path) -> u64 {
    let (code, stdout, stderr) = run(home, &["voucher", "verify"], "");
    assert_eq!(code, 0, "verify failed: {stderr}");
    let verify: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(verify["ok"], true);
    verify["voucher_count"].as_u64().unwrap()
}

/// Six transactions of year A; the last four are also in year B, which adds three.
fn yearly_exports() -> (Vec<Entry>, Vec<Entry>) {
    let shared = [
        entry("REF-1229-A", "2026-12-29", "10.00"),
        entry("REF-1230-B", "2026-12-30", "20.00"),
        entry("REF-1231-C", "2026-12-31", "30.00"),
        entry("REF-1231-D", "2026-12-31", "40.00"),
    ];
    let mut year_a = vec![
        entry("REF-1201-X", "2026-12-01", "1.00"),
        entry("REF-1215-Y", "2026-12-15", "2.00"),
    ];
    year_a.extend(shared.iter().cloned());
    let mut year_b = shared.to_vec();
    year_b.extend([
        entry("REF-0102-E", "2027-01-02", "5.00"),
        entry("REF-0103-F", "2027-01-03", "6.00"),
        entry("REF-0104-G", "2027-01-04", "7.00"),
    ]);
    (year_a, year_b)
}

#[test]
fn overlapping_yearly_exports_book_each_transaction_once() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let (year_a, year_b) = yearly_exports();
    let file_a = write_export(home, "a.xml", &export("YEAR-2026", &year_a));
    let file_b = write_export(home, "b.xml", &export("YEAR-2027", &year_b));

    let (code, a, err) = seal(home, &file_a, &assignments(&year_a, None));
    assert_eq!(code, 0, "{err}");
    assert_eq!(a["new_voucher_count"], 6);

    // The preview of year B already tells new from duplicate.
    let (code, stdout, stderr) = run(
        home,
        &["import", "camt", "seal", "--file", &file_b, "--dry-run"],
        &assignments(&year_b, None),
    );
    assert_eq!(code, 0, "{stderr}");
    let preview: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(preview["new_voucher_count"], 3);
    assert_eq!(preview["duplicate_observation_count"], 4);
    assert_eq!(preview["conflict_count"], 0);
    assert_eq!(
        preview["vouchers"][0]["outcome"],
        "duplicate_source_observation"
    );
    assert_eq!(preview["vouchers"][4]["outcome"], "new_voucher");
    assert!(preview["vouchers"][4].get("voucher_number").is_none());

    let (code, b, err) = seal(home, &file_b, &assignments(&year_b, None));
    assert_eq!(code, 0, "{err}");
    assert_eq!(b["new_voucher_count"], 3);
    assert_eq!(b["duplicate_observation_count"], 4);
    assert_eq!(b["conflict_count"], 0);
    assert_eq!(b["created_count"], 3);
    assert_eq!(b["replayed_count"], 4);
    let outcomes: Vec<&str> = b["sealed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["outcome"].as_str().unwrap())
        .collect();
    assert_eq!(
        outcomes,
        [
            "duplicate_source_observation",
            "duplicate_source_observation",
            "duplicate_source_observation",
            "duplicate_source_observation",
            "new_voucher",
            "new_voucher",
            "new_voucher"
        ]
    );
    // A duplicate names the voucher year A sealed for it.
    assert_eq!(b["sealed"][0]["number"], 3);
    assert_eq!(b["sealed"][0]["bank_ref"], "REF-1229-A");
    assert_eq!(voucher_count(home), 9);

    // Each file lists the transactions seen in it; a shared one is in both.
    let observed = |filter: Option<&str>| -> Vec<Value> {
        let mut args = vec!["import", "camt", "observations"];
        if let Some(hash) = filter {
            args.extend(["--attachment-ref", hash]);
        }
        let (code, stdout, stderr) = run(home, &args, "");
        assert_eq!(code, 0, "{stderr}");
        let out: Value = serde_json::from_str(&stdout).unwrap();
        out["observations"].as_array().unwrap().clone()
    };
    let in_a = observed(a["attachment_ref"].as_str());
    let in_b = observed(b["attachment_ref"].as_str());
    assert_eq!(in_a.len(), 6);
    assert_eq!(in_b.len(), 7);
    assert_eq!(observed(None).len(), 13);
    let shared_key = format!("bank-import:holvi:{IBAN}:REF-1229-A");
    assert!(in_a.iter().any(|o| o["event_key"] == shared_key.as_str()));
    assert!(in_b
        .iter()
        .any(|o| o["event_key"] == shared_key.as_str() && o["voucher_number"] == 3));

    // Importing year B again records nothing new and books nothing.
    let (code, again, err) = seal(home, &file_b, &assignments(&year_b, None));
    assert_eq!(code, 0, "{err}");
    assert_eq!(again["duplicate_observation_count"], 7);
    assert_eq!(observed(None).len(), 13);
    assert_eq!(voucher_count(home), 9);
}

/// Seal year A, then try year B with `year_b` changed; the batch must stop with
/// `code` and leave the books and the artifact store as they were.
fn assert_changed_year_b_stops(year_b: &[Entry], input: &str, code: &str, conflict_ref: &str) {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let (year_a, _) = yearly_exports();
    let file_a = write_export(home, "a.xml", &export("YEAR-2026", &year_a));
    let (status, _, err) = seal(home, &file_a, &assignments(&year_a, None));
    assert_eq!(status, 0, "{err}");
    let blobs = artifact_blobs(home).len();
    let vouchers_before = voucher_count(home);

    let file_b = write_export(home, "b.xml", &export("YEAR-2027", year_b));
    for extra in [&["--dry-run"][..], &[][..]] {
        let mut args = vec!["import", "camt", "seal", "--file", &file_b];
        args.extend_from_slice(extra);
        let (status, stdout, stderr) = run(home, &args, input);
        assert_eq!(status, 1, "{stdout}");
        let error: Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(error["error_code"], code, "{stderr}");
        assert_eq!(error["details"]["conflict_count"], 1);
        assert_eq!(error["details"]["new_voucher_count"], 3);
        let conflict = &error["details"]["conflicts"][0];
        assert_eq!(
            conflict["event_key"],
            format!("bank-import:holvi:{IBAN}:{conflict_ref}").as_str()
        );
        assert_eq!(conflict["error_code"], code);
    }
    assert_eq!(voucher_count(home), vouchers_before);
    assert_eq!(artifact_blobs(home).len(), blobs, "statement B was stored");
    let (_, stdout, _) = run(home, &["import", "camt", "observations"], "");
    let out: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out["observation_count"], 6);
}

#[test]
fn a_changed_amount_stops_the_batch_before_any_write() {
    let (_, mut year_b) = yearly_exports();
    year_b[1].amount = "21.00";
    let input = assignments(&year_b, None);
    assert_changed_year_b_stops(&year_b, &input, "SOURCE_FACTS_CHANGED", "REF-1230-B");
}

#[test]
fn a_changed_date_stops_the_batch_before_any_write() {
    let (_, mut year_b) = yearly_exports();
    year_b[2].date = "2027-01-01";
    let input = assignments(&year_b, None);
    assert_changed_year_b_stops(&year_b, &input, "SOURCE_FACTS_CHANGED", "REF-1231-C");
}

#[test]
fn a_changed_bank_status_stops_the_batch_before_any_write() {
    let (_, mut year_b) = yearly_exports();
    year_b[3].status = "PDNG";
    let input = assignments(&year_b, None);
    assert_changed_year_b_stops(&year_b, &input, "SOURCE_FACTS_CHANGED", "REF-1231-D");
}

#[test]
fn a_changed_assignment_of_a_booked_transaction_stops_the_batch_before_any_write() {
    let (_, year_b) = yearly_exports();
    let input = assignments(&year_b, Some(("REF-1229-A", "7600")));
    assert_changed_year_b_stops(&year_b, &input, "POSTING_PROJECTION_CHANGED", "REF-1229-A");
}

/// A home whose camt vouchers were sealed by the import before event identity:
/// the same voucher, under the same natural key, with no source fingerprint and
/// no recorded event. `voucher seal` with the preview's draft produces exactly
/// that voucher.
fn seal_like_the_previous_import(home: &std::path::Path, file: &str, input: &str) {
    let (code, stdout, stderr) = run(
        home,
        &["import", "camt", "seal", "--file", file, "--dry-run"],
        input,
    );
    assert_eq!(code, 0, "{stderr}");
    let preview: Value = serde_json::from_str(&stdout).unwrap();
    let (code, _, stderr) = run(home, &["artifact", "attach", "--file", file], "");
    assert_eq!(code, 0, "{stderr}");
    for voucher in preview["vouchers"].as_array().unwrap() {
        let request = json_string(&serde_json::json!({
            "draft": voucher["draft"],
            "natural_key": voucher["natural_key"],
            "actor": "importer",
        }));
        let (code, stdout, stderr) = run(home, &["voucher", "seal"], &request);
        assert_eq!(code, 0, "{stderr}");
        let sealed: Value = serde_json::from_str(&stdout).unwrap();
        assert!(sealed.get("source_fingerprint").is_none(), "{stdout}");
    }
}

fn json_string(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}

#[test]
fn vouchers_sealed_before_event_identity_are_recognised_from_their_statement() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let (year_a, year_b) = yearly_exports();
    let file_a = write_export(home, "a.xml", &export("YEAR-2026", &year_a));
    seal_like_the_previous_import(home, &file_a, &assignments(&year_a, None));
    assert_eq!(voucher_count(home), 6);

    // A changed booking of a legacy voucher is still refused.
    let file_b = write_export(home, "b.xml", &export("YEAR-2027", &year_b));
    let (code, _, err) = seal(
        home,
        &file_b,
        &assignments(&year_b, Some(("REF-1231-C", "7600"))),
    );
    assert_eq!(code, 1);
    assert_eq!(err["error_code"], "POSTING_PROJECTION_CHANGED");
    assert_eq!(err["details"]["backfilled_count"], 6);
    assert_eq!(voucher_count(home), 6);

    // The unchanged overlap is four duplicates; the legacy identities are
    // backfilled with the statement they were sealed from.
    let (code, b, err) = seal(home, &file_b, &assignments(&year_b, None));
    assert_eq!(code, 0, "{err}");
    assert_eq!(b["new_voucher_count"], 3);
    assert_eq!(b["duplicate_observation_count"], 4);
    assert_eq!(b["backfilled_count"], 6);
    assert_eq!(voucher_count(home), 9);
    let (_, stdout, _) = run(home, &["import", "camt", "observations"], "");
    let out: Value = serde_json::from_str(&stdout).unwrap();
    let rows = out["observations"].as_array().unwrap();
    assert_eq!(rows.len(), 13);
    assert_eq!(
        rows.iter().filter(|o| o["backfilled"] == true).count(),
        6,
        "{out}"
    );

    // Re-importing the legacy statement itself replays as before.
    let (code, a, err) = seal(home, &file_a, &assignments(&year_a, None));
    assert_eq!(code, 0, "{err}");
    assert_eq!(a["duplicate_observation_count"], 6);
    assert_eq!(a["backfilled_count"], 0);
}

#[test]
fn a_booked_key_whose_statement_cannot_be_read_back_is_never_booked_again() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let (_, year_b) = yearly_exports();
    // A voucher under the event key of REF-1229-A without a retained statement.
    let request = json_string(&serde_json::json!({
        "draft": {
            "document_date": "2026-12-29",
            "posting_date": "2026-12-29",
            "description": "Lasku REF-1229-A",
            "postings": [
                {"account": "7690", "amount": {"amount": "10.00", "currency": "EUR"}, "side": "debit"},
                {"account": "1910", "amount": {"amount": "10.00", "currency": "EUR"}, "side": "credit"}
            ]
        },
        "natural_key": format!("bank-import:holvi:{IBAN}:REF-1229-A"),
    }));
    let (code, _, stderr) = run(home, &["voucher", "seal"], &request);
    assert_eq!(code, 0, "{stderr}");

    let file_b = write_export(home, "b.xml", &export("YEAR-2027", &year_b));
    let (code, _, err) = seal(home, &file_b, &assignments(&year_b, None));
    assert_eq!(code, 1);
    assert_eq!(err["error_code"], "SOURCE_IDENTITY_UNKNOWN");
    assert_eq!(err["details"]["conflicts"][0]["voucher_number"], 1);
    assert_eq!(voucher_count(home), 1);
}

// ---------------------------------------------------------------------------
// Bank entries booked through `voucher seal` (issue
// `camt-import-manual-bank-vouchers`).
// ---------------------------------------------------------------------------

/// Book REF-1230-B (20.00 out on 2026-12-30) the way a company without the camt
/// import does: `voucher seal` under a key of its own, with `description`.
fn seal_bank_payment_by_hand(home: &std::path::Path, natural_key: &str, description: &str) {
    let request = json_string(&serde_json::json!({
        "draft": {
            "document_date": "2026-12-30",
            "posting_date": "2026-12-30",
            "description": description,
            "postings": [
                {"account": "7690", "amount": {"amount": "20.00", "currency": "EUR"}, "side": "debit"},
                {"account": "1910", "amount": {"amount": "20.00", "currency": "EUR"}, "side": "credit"}
            ]
        },
        "natural_key": natural_key,
    }));
    let (code, _, stderr) = run(home, &["voucher", "seal"], &request);
    assert_eq!(code, 0, "{stderr}");
}

/// The assignments for `entries` with REF-1230-B bound to voucher #1 instead.
fn binding_input(entries: &[Entry]) -> Value {
    let mut input: Value = serde_json::from_str(&assignments(entries, None)).unwrap();
    input["assignments"]
        .as_array_mut()
        .unwrap()
        .retain(|a| a["bank_ref"] != "REF-1230-B");
    input["existing_vouchers"] = serde_json::json!([
        {"bank_ref": "REF-1230-B", "bank_account": "1910", "voucher_number": 1}
    ]);
    input
}

#[test]
fn an_entry_booked_through_voucher_seal_stops_the_import_until_bound() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    seal_bank_payment_by_hand(home, "bank:REF-1230-B", "Toimittaja Oy");
    let (year_a, year_b) = yearly_exports();
    let file_a = write_export(home, "a.xml", &export("YEAR-2026", &year_a));
    let blobs = artifact_blobs(home).len();

    // The import sees the voucher and stops, in the preview and in the seal.
    for extra in [&["--dry-run"][..], &[][..]] {
        let mut args = vec!["import", "camt", "seal", "--file", &file_a];
        args.extend_from_slice(extra);
        let (status, stdout, stderr) = run(home, &args, &assignments(&year_a, None));
        assert_eq!(status, 1, "{stdout}");
        let error: Value = serde_json::from_str(&stderr).unwrap();
        assert_eq!(
            error["error_code"], "BANK_ENTRY_POSSIBLY_BOOKED",
            "{stderr}"
        );
        let conflicts = error["details"]["conflicts"].as_array().unwrap();
        assert_eq!(conflicts.len(), 1, "{stderr}");
        assert_eq!(
            conflicts[0]["event_key"],
            format!("bank-import:holvi:{IBAN}:REF-1230-B").as_str()
        );
        assert_eq!(conflicts[0]["voucher_number"], 1);
        assert_eq!(conflicts[0]["evidence"], "bank_reference");
    }
    assert_eq!(voucher_count(home), 1);
    assert_eq!(
        artifact_blobs(home).len(),
        blobs,
        "the statement was stored"
    );

    // Bound to voucher #1: the preview shows the binding and the approval
    // seals the other five.
    let input = json_string(&binding_input(&year_a));
    let (code, stdout, stderr) = run(
        home,
        &["import", "camt", "seal", "--file", &file_a, "--dry-run"],
        &input,
    );
    assert_eq!(code, 0, "{stderr}");
    let preview: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(preview["bound_count"], 1);
    assert_eq!(preview["new_voucher_count"], 5);
    let bound = &preview["vouchers"][3];
    assert_eq!(bound["bank_ref"], "REF-1230-B");
    assert_eq!(bound["outcome"], "bound_existing_voucher");
    assert_eq!(bound["voucher_number"], 1);
    assert!(bound.get("draft").is_none());
    let hash = preview["preparation_hash"].as_str().unwrap().to_string();
    let (code, stdout, stderr) = run(
        home,
        &[
            "import",
            "camt",
            "seal",
            "--file",
            &file_a,
            "--approved-preparation-hash",
            &hash,
        ],
        &input,
    );
    assert_eq!(code, 0, "{stderr}");
    let sealed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(sealed["bound_count"], 1);
    assert_eq!(sealed["created_count"], 5);
    assert_eq!(sealed["replayed_count"], 0);
    assert_eq!(sealed["sealed"][3]["number"], 1);
    assert_eq!(voucher_count(home), 6);

    // The overlapping export with the same binding books nothing twice.
    let file_b = write_export(home, "b.xml", &export("YEAR-2027", &year_b));
    let (code, b, err) = seal(home, &file_b, &json_string(&binding_input(&year_b)));
    assert_eq!(code, 0, "{err}");
    assert_eq!(b["duplicate_observation_count"], 4);
    assert_eq!(b["new_voucher_count"], 3);
    assert_eq!(voucher_count(home), 9);
}

#[test]
fn a_voucher_with_the_same_amount_and_date_stops_the_import_until_ruled_out() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    seal_bank_payment_by_hand(home, "receipt-17", "Toimittaja Oy");
    let (year_a, _) = yearly_exports();
    let file_a = write_export(home, "a.xml", &export("YEAR-2026", &year_a));

    let (code, _, err) = seal(home, &file_a, &assignments(&year_a, None));
    assert_eq!(code, 1);
    assert_eq!(err["error_code"], "BANK_ENTRY_POSSIBLY_BOOKED");
    assert_eq!(
        err["details"]["conflicts"][0]["evidence"],
        "account_amount_date"
    );
    assert_eq!(voucher_count(home), 1);

    // The user has checked that #1 is another payment.
    let mut input: Value = serde_json::from_str(&assignments(&year_a, None)).unwrap();
    for row in input["assignments"].as_array_mut().unwrap() {
        if row["bank_ref"] == "REF-1230-B" {
            row["not_booked_by"] = serde_json::json!([1]);
        }
    }
    let (code, out, err) = seal(home, &file_a, &json_string(&input));
    assert_eq!(code, 0, "{err}");
    assert_eq!(out["created_count"], 6);
    assert_eq!(voucher_count(home), 7);
}

#[test]
fn a_binding_is_refused_when_its_input_is_inconsistent() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    seal_bank_payment_by_hand(home, "bank:REF-1230-B", "Toimittaja Oy");
    let (year_a, _) = yearly_exports();
    let file_a = write_export(home, "a.xml", &export("YEAR-2026", &year_a));

    // Both an assignment and a binding for the same entry.
    let mut both: Value = serde_json::from_str(&assignments(&year_a, None)).unwrap();
    both["existing_vouchers"] = binding_input(&year_a)["existing_vouchers"].clone();
    // A binding for an entry that is not in the statement.
    let mut unknown = binding_input(&year_a);
    unknown["existing_vouchers"][0]["bank_ref"] = "REF-NOWHERE".into();
    // A binding to a voucher whose bank leg is on another account.
    let mut wrong = binding_input(&year_a);
    wrong["existing_vouchers"][0]["bank_account"] = "1911".into();
    for (input, code) in [
        (both, "INVALID_ASSIGNMENT"),
        (unknown, "INVALID_ASSIGNMENT"),
        (wrong, "BANK_VOUCHER_MISMATCH"),
    ] {
        let (status, _, err) = seal(home, &file_a, &json_string(&input));
        assert_eq!(status, 1);
        assert_eq!(err["error_code"], code, "{err}");
    }
    assert_eq!(voucher_count(home), 1);
}

/// A statement with balances and no entries, in either container.
fn balance_only(container: &str) -> String {
    let (ns, outer, block) = match container {
        "052" => ("camt.052.001.02", "BkToCstmrAcctRpt", "Rpt"),
        _ => ("camt.053.001.02", "BkToCstmrStmt", "Stmt"),
    };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:{ns}"><{outer}><{block}><Id>Q-2026-04</Id>
<FrToDt><FrDtTm>2026-04-01T00:00:00</FrDtTm><ToDtTm>2026-04-30T23:59:59</ToDtTm></FrToDt>
<Acct><Id><IBAN>FI2112345600000785</IBAN></Id><Ccy>EUR</Ccy></Acct>
<Bal><Tp><CdOrPrtry><Cd>OPBD</Cd></CdOrPrtry></Tp><Amt Ccy="EUR">250.00</Amt><CdtDbtInd>CRDT</CdtDbtInd><Dt><Dt>2026-04-01</Dt></Dt></Bal>
<Bal><Tp><CdOrPrtry><Cd>CLBD</Cd></CdOrPrtry></Tp><Amt Ccy="EUR">250.00</Amt><CdtDbtInd>CRDT</CdtDbtInd><Dt><Dt>2026-04-30</Dt></Dt></Bal>
</{block}></{outer}></Document>"#
    )
}

fn assert_empty_report(value: &Value) {
    let empty = &value["empty_report"];
    assert_eq!(empty["account_iban"], "FI2112345600000785", "{value}");
    assert_eq!(empty["periods"][0]["from"], "2026-04-01");
    assert_eq!(empty["periods"][0]["to"], "2026-04-30");
    let message = empty["message"].as_str().unwrap();
    assert!(
        message.contains("FI2112345600000785") && message.contains("2026-04-01..2026-04-30"),
        "{message}"
    );
}

/// A quiet month imports without a workaround, and every step of the flow
/// (parse, preview, seal) says the report had no entries, for which account
/// and period.
#[test]
fn a_balance_only_statement_imports_as_an_explicit_empty_report() {
    for container in ["052", "053"] {
        let home = tempfile::tempdir().unwrap();
        let home = home.path();
        assert_eq!(run(home, &["init"], "").0, 0);
        let file = write_export(home, "quiet.xml", &balance_only(container));

        let (code, stdout, stderr) = run(home, &["import", "camt", "parse", "--file", &file], "");
        assert_eq!(code, 0, "{container}: {stderr}");
        let parsed: Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(parsed["transactions"].as_array().unwrap().len(), 0);
        assert_empty_report(&parsed);

        let input = r#"{"assignments":[]}"#;
        let (code, stdout, stderr) = run(
            home,
            &["import", "camt", "seal", "--file", &file, "--dry-run"],
            input,
        );
        assert_eq!(code, 0, "{container}: {stderr}");
        let preview: Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(preview["voucher_count"], 0);
        assert_empty_report(&preview);
        let hash = preview["preparation_hash"].as_str().unwrap().to_string();

        let (code, stdout, stderr) = run(
            home,
            &[
                "import",
                "camt",
                "seal",
                "--file",
                &file,
                "--approved-preparation-hash",
                &hash,
            ],
            input,
        );
        assert_eq!(code, 0, "{container}: {stderr}");
        let sealed: Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(sealed["sealed_count"], 0);
        assert_empty_report(&sealed);
        assert_eq!(voucher_count(home), 0);
    }
}

#[test]
fn the_human_output_of_an_empty_import_says_it_was_empty() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_export(home, "quiet.xml", &balance_only("052"));
    let out = bin()
        .arg("--home")
        .arg(home)
        .args(["import", "camt", "parse", "--file", &file])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains("empty report: ") && stdout.contains("2026-04-01..2026-04-30"),
        "{stdout}"
    );
}

/// `import camt053` stays a working alias for callers written before the
/// rename, and `import camt052` keeps working too.
#[test]
fn the_old_verbs_remain_aliases_of_import_camt() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, &["init"], "").0, 0);
    let file = write_sample(home);
    let (_, primary, _) = run(home, &["import", "camt", "parse", "--file", &file], "");
    for alias in ["camt053", "camt052"] {
        let (code, stdout, stderr) = run(home, &["import", alias, "parse", "--file", &file], "");
        assert_eq!(code, 0, "{alias}: {stderr}");
        assert_eq!(stdout, primary, "{alias}");
    }
    let (code, stdout, stderr) = run(
        home,
        &["import", "camt053", "seal", "--file", &file],
        ASSIGNMENTS,
    );
    assert_eq!(code, 0, "{stderr}");
    let sealed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(sealed["created_count"], 2);
}

#[test]
fn a_malformed_camt052_reports_invalid_camt() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let broken = balance_only("052").replace("<IBAN>FI2112345600000785</IBAN>", "");
    let file = write_export(home, "broken.xml", &broken);
    let (code, stdout, stderr) = run(home, &["import", "camt", "parse", "--file", &file], "");
    assert_ne!(code, 0, "{stdout}");
    let error: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(error["error_code"], "INVALID_CAMT", "{stderr}");
    assert!(
        error["message"].as_str().unwrap().contains("IBAN"),
        "{stderr}"
    );
    assert!(!stderr.contains("053"), "{stderr}");
}
