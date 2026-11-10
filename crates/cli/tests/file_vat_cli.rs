//! S3 done-criterion: the `file-vat` Run runs **end to end via the CLI**, with the
//! authority transport supplied through `$AGGOUNTANT_BROKER_SCRIPT`. Proves the CLI
//! wiring: primitive-per-command, `--json`, structured
//! stdin, machine-readable error codes — and the `BROKER_NOT_CONFIGURED` guard.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
}

fn help(args: &[&str]) -> String {
    let output = bin().args(args).arg("--help").output().unwrap();
    assert!(output.status.success(), "{:?}", output.status);
    String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn file_snapshot(root: &std::path::Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    fn walk(
        root: &std::path::Path,
        dir: &std::path::Path,
        out: &mut Vec<(std::path::PathBuf, Vec<u8>)>,
    ) {
        if !dir.exists() {
            return;
        }
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.push((
                    path.strip_prefix(root).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                ));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn run(
    home: &std::path::Path,
    script: Option<&std::path::Path>,
    args: &[&str],
    stdin: &str,
) -> (i32, String, String) {
    let mut cmd = bin();
    cmd.arg("--home").arg(home).arg("--json");
    cmd.env_remove("AGGOUNTANT_BROKER_SCRIPT");
    for key in [
        "AGGOUNTANT_VERO_API_BASE_URL",
        "AGGOUNTANT_VERO_CREDENTIALS_FILE",
        "AGGOUNTANT_VERO_ENV",
        "AGGOUNTANT_VERO_PRODUCTION_APPROVED",
    ] {
        cmd.env_remove(key);
    }
    if let Some(s) = script {
        cmd.env("AGGOUNTANT_BROKER_SCRIPT", s);
    }
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

/// A `voucher seal` stdin for a balanced VAT voucher. The `tagged` line carries
/// `base` with `code`, on its accrual side — sales VAT accrues on the credit side,
/// purchase VAT on the debit side — so the derived VAT is positive (a normal
/// invoice, not a credit note). `document_date` 2026-02 selects the 25,5 % rate.
fn vat_seal_json(natural_key: &str, tagged: &str, base: &str, code: &str, counter: &str) -> String {
    let (tagged_side, counter_side) = if code.starts_with("sales-") {
        ("credit", "debit")
    } else {
        ("debit", "credit")
    };
    format!(
        r#"{{
  "draft": {{
    "document_date": "2026-02-15",
    "posting_date": "2026-02-15",
    "description": "vat voucher",
    "postings": [
      {{ "account": "{tagged}", "amount": {{ "amount": "{base}", "currency": "EUR" }}, "side": "{tagged_side}", "vat_code": "{code}" }},
      {{ "account": "{counter}", "amount": {{ "amount": "{base}", "currency": "EUR" }}, "side": "{counter_side}" }}
    ]
  }},
  "natural_key": "{natural_key}"
}}"#
    )
}

/// Seal Q1 (Open→Closing→ApproveClose→Sealed) via the CLI, the file-vat precondition.
/// Also seals a sales (10000 base → 2550,00 output VAT) and a purchase (4000 base →
/// 1020,00 deductible VAT) voucher so the in-Run derived-vs-input cross-check passes.
fn seal_q1(home: &std::path::Path) {
    assert_eq!(run(home, None, &["init"], "").0, 0);
    // Sales output VAT: base on the credit line (revenue) tagged sales-*.
    let (code, _, err) = run(
        home,
        None,
        &["voucher", "seal"],
        &vat_seal_json(
            "sales-v1",
            "3000",
            "10000.00",
            "sales-domestic-standard",
            "1700",
        ),
    );
    assert_eq!(code, 0, "{err}");
    // Purchase input VAT: base on the debit line (an expense) tagged purchase-*.
    let (code, _, err) = run(
        home,
        None,
        &["voucher", "seal"],
        &vat_seal_json(
            "purchase-v1",
            "4000",
            "4000.00",
            "purchase-domestic-standard",
            "1910",
        ),
    );
    assert_eq!(code, 0, "{err}");
    let (code, stdout, err) = run(
        home,
        None,
        &["run", "start", "close-period", "--period", "2026-Q1"],
        "",
    );
    assert_eq!(code, 0, "{err}");
    let started: Value = serde_json::from_str(&stdout).unwrap();
    let run_id = started["run_id"].as_str().unwrap().to_string();
    let (code, _, err) = run(
        home,
        None,
        &[
            "run",
            "answer",
            "--run-id",
            &run_id,
            "--gate",
            "ApproveClose",
            "--answer",
            "approve",
        ],
        "",
    );
    assert_eq!(code, 0, "{err}");
}

const FILE_VAT_INPUT: &str = r#"{
  "period": "2026-Q1",
  "sales_vat": { "amount": "2550.00", "currency": "EUR" },
  "purchase_vat": { "amount": "1020.00", "currency": "EUR" },
  "business_id": "1572860-0",
        "contact_name": "Ada Operator",
        "contact_phone": "+358401234567",
        "payee_iban": "FI2112345600000785"
}"#;

fn happy_script() -> String {
    json!({
      "vero-vat-periods": [{"status": 200, "body": {"FilingPeriod":[{
          "Period":"2026-03-31", "Frequency":"VATQTR",
          "StartDate":"2026-01-01", "EndDate":"2026-03-31",
          "Status":"Missing", "DueDate":"2026-05-12"
      }]}}],
      "vero-file-vat-return": [{"status": 200, "body": {
          "UniqueIdentifier":"VERO-R1", "AcceptedTimestamp":"2026-05-01T10:00:00Z"
      }}],
      "pankki-maksu": [{"status": 200, "body": {"receipt": "PMT-1"}}]
    })
    .to_string()
}

#[test]
fn offline_preview_works_while_vero_is_disabled_without_run_or_credential_access() {
    let home = tempfile::tempdir().unwrap();
    seal_q1(home.path());
    let database = home.path().join("fiscal.sqlite");
    let before = std::fs::read(&database).unwrap();
    let process_before = file_snapshot(&home.path().join("process"));

    let mut cmd = bin();
    cmd.arg("--home")
        .arg(home.path())
        .arg("--json")
        .args(["file-vat", "preview", "--period", "2026-Q1"])
        // Both paths are deliberately unusable. Preview must not inspect either one.
        .env(
            "AGGOUNTANT_BROKER_SCRIPT",
            home.path().join("missing-script.json"),
        )
        .env(
            "AGGOUNTANT_VERO_CREDENTIALS_FILE",
            home.path().join("missing-credentials.json"),
        );
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let preview: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(preview["filing_fields"]["301"]["amount"], "2550.00");
    assert_eq!(preview["filing_fields"]["307"]["amount"], "1020.00");
    assert_eq!(preview["filing_fields"]["308"]["amount"], "1530.00");
    assert_eq!(preview["payment_amount"], "1530.00");
    assert!(preview["payment_instruction"]
        .as_str()
        .unwrap()
        .contains("does not initiate"));
    assert_eq!(std::fs::read(database).unwrap(), before);
    assert_eq!(file_snapshot(&home.path().join("process")), process_before);
}

#[test]
fn manual_completion_accepts_only_supported_channels_and_records_evidence() {
    let home = tempfile::tempdir().unwrap();
    seal_q1(home.path());
    let (code, stdout, err) = run(
        home.path(),
        None,
        &[
            "file-vat",
            "complete-manual",
            "--period",
            "2026-Q1",
            "--channel",
            "omavero",
            "--evidence-ref",
            "archive/vero/receipt.pdf",
        ],
        "",
    );
    assert_eq!(code, 0, "{err}");
    let completed: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(completed["vat_filed"], true);
    assert_eq!(completed["manual_filing"]["channel"], "omavero");
    assert_eq!(
        completed["manual_filing"]["evidence_ref"],
        "archive/vero/receipt.pdf"
    );

    let other = tempfile::tempdir().unwrap();
    seal_q1(other.path());
    let (code, _, stderr) = run(
        other.path(),
        None,
        &[
            "file-vat",
            "complete-manual",
            "--period",
            "2026-Q1",
            "--channel",
            "paper",
            "--evidence-ref",
            "archive/receipt.pdf",
        ],
        "",
    );
    assert_eq!(code, 1);
    assert!(stderr.contains("invalid value 'paper'"), "{stderr}");
}

#[test]
fn cli_manual_completion_requires_resolution_evidence_after_unknown() {
    let home = tempfile::tempdir().unwrap();
    seal_q1(home.path());
    let script = home.path().join("broker_script.json");
    std::fs::write(
        &script,
        json!({
            "vero-vat-periods": [{"status": 200, "body": {"FilingPeriod":[{
                "Period":"2026-03-31", "Frequency":"VATQTR",
                "StartDate":"2026-01-01", "EndDate":"2026-03-31",
                "Status":"Missing", "DueDate":"2026-05-12"
            }]}}],
            "vero-file-vat-return": [{"timeout": true}]
        })
        .to_string(),
    )
    .unwrap();
    let (_, stdout, _) = run(
        home.path(),
        Some(&script),
        &["file-vat", "start"],
        FILE_VAT_INPUT,
    );
    let run_id = serde_json::from_str::<Value>(&stdout).unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (code, _, err) = run(
        home.path(),
        Some(&script),
        &["file-vat", "approve-filing", "--run-id", &run_id],
        "",
    );
    assert_eq!(code, 0, "{err}");

    let base = [
        "file-vat",
        "complete-manual",
        "--period",
        "2026-Q1",
        "--channel",
        "ilmoitin",
        "--evidence-ref",
        "archive/manual-receipt.txt",
    ];
    let (code, _, stderr) = run(home.path(), None, &base, "");
    assert_eq!(code, 1);
    assert!(stderr.contains("OUTCOME_UNKNOWN"), "{stderr}");

    let mut resolved = base.to_vec();
    resolved.extend([
        "--remote-status-evidence-ref",
        "archive/status-resolution.txt",
    ]);
    let (code, stdout, stderr) = run(home.path(), None, &resolved, "");
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["manual_filing"]
            ["remote_status_evidence_ref"],
        "archive/status-resolution.txt"
    );
}

#[test]
fn file_vat_help_describes_direct_and_scripted_vero_transport_truthfully() {
    for args in [
        vec!["file-vat"],
        vec!["file-vat", "start"],
        vec!["file-vat", "approve-filing"],
        vec!["file-vat", "reconcile"],
    ] {
        let help = help(&args);
        assert!(
            help.contains("Broker reliability uses an outbox and OutcomeUnknown reconciliation"),
            "{help}"
        );
        assert!(!help.contains("409-recovery"), "{help}");
        assert!(help.contains("fixed Vero.fi target"), "{help}");
        assert!(!help.contains("AGGOUNTANT_VERO_API_BASE_URL"), "{help}");
        assert!(help.contains("AGGOUNTANT_VERO_CREDENTIALS_FILE"), "{help}");
        assert!(help.contains("vero-fi status"), "{help}");
        assert!(
            help.contains("AGGOUNTANT_BROKER_SCRIPT selects the scripted test seam only"),
            "{help}"
        );
        assert!(!help.contains("ships no live network adapter"), "{help}");
    }
}

#[test]
fn file_vat_end_to_end_via_cli() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    seal_q1(home);

    let script = home.join("broker_script.json");
    std::fs::write(&script, happy_script()).unwrap();
    // The explicit script seam must neither read nor validate the live profile.
    // This also catches accidental second seam decisions inside input projection.
    std::fs::write(home.join("vero-filing.json"), "not-json").unwrap();

    // Start → parked on ApproveFiling, with the computed figures echoed.
    let (code, stdout, err) = run(home, Some(&script), &["file-vat", "start"], FILE_VAT_INPUT);
    assert_eq!(code, 0, "{err}");
    let started: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(started["status"], "awaiting_gate");
    assert_eq!(started["net_vat"], "1530.00");
    assert_eq!(started["bank_payment_required"], "1530.00");
    let run_id = started["run_id"].as_str().unwrap().to_string();

    // Approve filing → payment gate opens.
    let (code, stdout, err) = run(
        home,
        Some(&script),
        &["file-vat", "approve-filing", "--run-id", &run_id],
        "",
    );
    assert_eq!(code, 0, "{err}");
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["status"], "awaiting_gate");

    // period show → filed.
    let (_, stdout, _) = run(home, None, &["period", "show", "--period", "2026-Q1"], "");
    let pv: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(pv["vat_filed"], true);

    // Approve payment → Completed.
    let (code, stdout, err) = run(
        home,
        Some(&script),
        &["file-vat", "approve-payment", "--run-id", &run_id],
        "",
    );
    assert_eq!(code, 0, "{err}");
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["status"], "completed");

    // The whole chain (including the audited filing self-edge) still verifies.
    let (code, _, _) = run(home, None, &["voucher", "verify"], "");
    assert_eq!(code, 0);
}

#[test]
fn file_vat_without_a_broker_script_refuses() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    seal_q1(home);
    // No configured production readiness: refuse rather than fabricate a filing.
    // This is caller-actionable through `vero-fi status`/`configure` (exit 1).
    let (code, _, stderr) = run(home, None, &["file-vat", "start"], FILE_VAT_INPUT);
    assert_eq!(code, 1);
    assert!(stderr.contains("VERO_NOT_READY"), "{stderr}");
    // Missing identity is projected once by readiness, not re-checked with a
    // second command-specific error after Broker construction.
    for blocker in [
        "VERO_BUSINESS_ID_MISSING",
        "VERO_CONTACT_NAME_MISSING",
        "VERO_CONTACT_PHONE_MISSING",
    ] {
        assert_eq!(stderr.matches(blocker).count(), 1, "{stderr}");
    }
}

#[test]
fn obsolete_environment_activation_cannot_enable_production() {
    let home = tempfile::tempdir().unwrap();
    seal_q1(home.path());
    let mut cmd = bin();
    cmd.arg("--home")
        .arg(home.path())
        .arg("--json")
        .args(["file-vat", "start"])
        .env_remove("AGGOUNTANT_BROKER_SCRIPT")
        .env(
            "AGGOUNTANT_VERO_API_BASE_URL",
            "https://production.example/",
        )
        .env("AGGOUNTANT_VERO_ENV", "production")
        .env_remove("AGGOUNTANT_VERO_PRODUCTION_APPROVED")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(FILE_VAT_INPUT.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("VERO_NOT_READY"));
}

#[test]
fn file_vat_timeout_reconciles_without_resubmitting() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    seal_q1(home);

    // Each CLI invocation is a fresh process with a fresh transport that reads the
    // script from the top — so we rewrite the script between steps to model the
    // world changing between attempts (the first submit times out; reconciliation
    // later observes the accepted return without resubmitting).
    let script = home.join("broker_script.json");
    let periods = json!({"status":200,"body":{"FilingPeriod":[{
        "Period":"2026-03-31", "Frequency":"VATQTR",
        "StartDate":"2026-01-01", "EndDate":"2026-03-31",
        "Status":"Missing", "DueDate":"2026-05-12"
    }]}});
    std::fs::write(
        &script,
        json!({
            "vero-vat-periods":[periods],
            "vero-file-vat-return":[{"timeout":true}]
        })
        .to_string(),
    )
    .unwrap();

    let (_, stdout, _) = run(home, Some(&script), &["file-vat", "start"], FILE_VAT_INPUT);
    let started: Value = serde_json::from_str(&stdout).unwrap();
    let run_id = started["run_id"].as_str().unwrap().to_string();
    let filed_request: Value =
        serde_json::from_str(started["filing_request"].as_str().unwrap()).unwrap();
    let details = &filed_request["VATDetails"];
    let filed = json!({
        "Status":"Processed", "SubmittedDate":"2099-01-01",
        "VATOnDomesticSales": details["VATOnDomesticSalesByTaxRate"],
        "VATOnPurchasesAndImports": details["VATOnPurchasesAndImports"],
        "DeductibleTax":{"DeductibleVAT":details["DeductibleVAT"]},
        "SalesPurchasesImports":details["SalesPurchasesImports"]
    });

    // Approve → Unknown → NeedsAttention.
    let (code, stdout, err) = run(
        home,
        Some(&script),
        &["file-vat", "approve-filing", "--run-id", &run_id],
        "",
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["status"],
        "needs_attention"
    );

    // Visibility has caught up: reconcile the matching latest return. No filing
    // submit operation is configured in this second process, proving no resend.
    std::fs::write(
        &script,
        json!({
            "vero-filed-vat-return":[{"status":200,"body":filed}]
        })
        .to_string(),
    )
    .unwrap();

    let (code, stdout, err) = run(
        home,
        Some(&script),
        &["file-vat", "reconcile", "--run-id", &run_id],
        "",
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["status"],
        "awaiting_gate"
    );
}
