//! `file-vat status` through the CLI: `--json` shape, exit codes, and that a
//! failed authority read is `VAT_STATUS_UNKNOWN`, never "not filed".

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn run(home: &Path, script: Option<&Path>, args: &[&str], stdin: &str) -> (i32, Value, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aggountant"));
    cmd.arg("--home").arg(home).arg("--json");
    for key in [
        "AGGOUNTANT_BROKER_SCRIPT",
        "AGGOUNTANT_VERO_API_BASE_URL",
        "AGGOUNTANT_VERO_CREDENTIALS_FILE",
        "AGGOUNTANT_VERO_ENV",
        "AGGOUNTANT_VERO_PRODUCTION_APPROVED",
    ] {
        cmd.env_remove(key);
    }
    if let Some(script) = script {
        cmd.env("AGGOUNTANT_BROKER_SCRIPT", script);
    }
    let mut child = cmd
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    (
        out.status.code().unwrap_or(-1),
        serde_json::from_str(&stdout).unwrap_or(Value::Null),
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn seal(home: &Path, key: &str, tagged: &str, counter: &str, base: &str, code: &str) {
    let (tagged_side, counter_side) = if code.starts_with("sales-") {
        ("credit", "debit")
    } else {
        ("debit", "credit")
    };
    let line = |account: &str, side: &str, vat: Option<&str>| {
        let mut line = json!({
            "account": account,
            "amount": {"amount": base, "currency": "EUR"},
            "side": side,
        });
        if let Some(vat) = vat {
            line["vat_code"] = json!(vat);
        }
        line
    };
    let input = json!({
        "draft": {
            "document_date": "2026-02-15",
            "posting_date": "2026-02-15",
            "description": key,
            "postings": [line(tagged, tagged_side, Some(code)), line(counter, counter_side, None)],
        },
        "natural_key": key,
    });
    let (code, _, err) = run(home, None, &["voucher", "seal"], &input.to_string());
    assert_eq!(code, 0, "{err}");
}

/// Esimerkki Oy's Q1: 2550,00 sales VAT and 1020,00 deductible VAT, sealed.
fn sealed_q1(home: &Path) {
    assert_eq!(run(home, None, &["init"], "").0, 0);
    seal(
        home,
        "sales",
        "3000",
        "1700",
        "10000.00",
        "sales-domestic-standard",
    );
    seal(
        home,
        "purchase",
        "4000",
        "1910",
        "4000.00",
        "purchase-domestic-standard",
    );
    let (code, started, err) = run(
        home,
        None,
        &["run", "start", "close-period", "--period", "2026-Q1"],
        "",
    );
    assert_eq!(code, 0, "{err}");
    let run_id = started["run_id"].as_str().unwrap();
    let args = [
        "run",
        "answer",
        "--run-id",
        run_id,
        "--gate",
        "ApproveClose",
        "--answer",
        "approve",
    ];
    assert_eq!(run(home, None, &args, "").0, 0);
}

const INPUT: &str = r#"{
  "period": "2026-Q1",
  "sales_vat": { "amount": "2550.00", "currency": "EUR" },
  "purchase_vat": { "amount": "1020.00", "currency": "EUR" },
  "business_id": "1234567-1",
  "contact_name": "Esimerkki Oy",
  "contact_phone": "+358401234567",
  "payee_iban": "FI2112345600000785"
}"#;

/// File Q1 through the scripted seam; returns the approved request JSON.
fn filed_q1(home: &Path) -> Value {
    sealed_q1(home);
    let script = home.join("file-script.json");
    std::fs::write(
        &script,
        json!({
            "vero-vat-periods": [{"status": 200, "body": {"FilingPeriod": [{
                "Period": "2026-03-31", "Frequency": "VATQTR",
                "StartDate": "2026-01-01", "EndDate": "2026-03-31",
                "Status": "Missing", "DueDate": "2026-05-12"
            }]}}],
            "vero-file-vat-return": [{"status": 200, "body": {
                "UniqueIdentifier": "VERO-R1", "AcceptedTimestamp": "2026-05-01T10:00:00Z"
            }}]
        })
        .to_string(),
    )
    .unwrap();
    let (code, started, err) = run(home, Some(&script), &["file-vat", "start"], INPUT);
    assert_eq!(code, 0, "{err}");
    let run_id = started["run_id"].as_str().unwrap();
    let (code, _, err) = run(
        home,
        Some(&script),
        &["file-vat", "approve-filing", "--run-id", run_id],
        "",
    );
    assert_eq!(code, 0, "{err}");
    serde_json::from_str(started["filing_request"].as_str().unwrap()).unwrap()
}

/// A script that offers only the GetFiledVATReturn read. A FileVATReturn request
/// would find no scripted response and fail the command.
fn status_script(home: &Path, step: Value) -> std::path::PathBuf {
    let script = home.join("status-script.json");
    std::fs::write(
        &script,
        json!({ "vero-filed-vat-return": [step] }).to_string(),
    )
    .unwrap();
    script
}

const STATUS: [&str; 4] = ["file-vat", "status", "--period", "2026-Q1"];

#[test]
fn status_reports_a_processed_return_from_the_receipt() {
    let home = tempfile::tempdir().unwrap();
    let request = filed_q1(home.path());
    let details = &request["VATDetails"];
    let script = status_script(
        home.path(),
        json!({"status": 200, "body": {
            "Status": "Processed", "SubmittedDate": "2026-05-02",
            "VATOnDomesticSales": details["VATOnDomesticSalesByTaxRate"],
            "VATOnPurchasesAndImports": details["VATOnPurchasesAndImports"],
            "DeductibleTax": {"DeductibleVAT": details["DeductibleVAT"]},
            "SalesPurchasesImports": details["SalesPurchasesImports"],
        }}),
    );
    let (code, status, err) = run(home.path(), Some(&script), &STATUS, "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(status["status"], "processed");
    assert_eq!(status["authority_checked"], true);
    assert_eq!(status["receipt"]["unique_identifier"], "VERO-R1");
    assert_eq!(status["next_command"], Value::Null);
}

#[test]
fn status_reports_received_while_the_return_is_not_visible() {
    let home = tempfile::tempdir().unwrap();
    filed_q1(home.path());
    let script = status_script(
        home.path(),
        json!({"status": 200, "body": {"Status": "Missing"}}),
    );
    let (code, status, err) = run(home.path(), Some(&script), &STATUS, "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(status["status"], "received");
}

#[test]
fn a_failed_read_is_status_unknown_with_a_system_exit() {
    let home = tempfile::tempdir().unwrap();
    filed_q1(home.path());
    let script = status_script(home.path(), json!({"timeout": true}));
    let (code, _, stderr) = run(home.path(), Some(&script), &STATUS, "");
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("VAT_STATUS_UNKNOWN"), "{stderr}");
    assert!(stderr.contains("VERO-R1"), "{stderr}");
    assert!(!stderr.contains("not_filed"), "{stderr}");
}

#[test]
fn missing_vero_readiness_is_status_unknown_not_not_filed() {
    let home = tempfile::tempdir().unwrap();
    filed_q1(home.path());
    let (code, _, stderr) = run(home.path(), None, &STATUS, "");
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("VAT_STATUS_UNKNOWN"), "{stderr}");
    assert!(stderr.contains("VERO_NOT_READY"), "{stderr}");
}

#[test]
fn an_unfiled_period_needs_no_credentials() {
    let home = tempfile::tempdir().unwrap();
    sealed_q1(home.path());
    // No script and no readiness: answered from local records alone.
    let (code, status, err) = run(home.path(), None, &STATUS, "");
    assert_eq!(code, 0, "{err}");
    assert_eq!(status["status"], "not_filed");
    assert_eq!(status["authority_checked"], false);
}
