//! The reproduction of issue `vat-untagged-posting-omitted` through the CLI:
//! Esimerkki Oy seals a March 2026 sale with 503,90 € output VAT on 2939 and no VAT
//! code. `voucher validate` warns, `file-vat preview` reports the discrepancy and
//! `file-vat start` refuses before contacting the Tax Administration.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn run(
    home: &std::path::Path,
    script: Option<&std::path::Path>,
    args: &[&str],
    stdin: &str,
) -> (i32, Value, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aggountant"));
    cmd.arg("--home").arg(home).arg("--json");
    cmd.env_remove("AGGOUNTANT_BROKER_SCRIPT");
    if let Some(script) = script {
        cmd.env("AGGOUNTANT_BROKER_SCRIPT", script);
    }
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
    (
        out.status.code().unwrap_or(-1),
        value,
        String::from_utf8(out.stderr).unwrap(),
    )
}

fn sale() -> Value {
    let line = |account: &str, side: &str, amount: &str| json!({"account": account, "amount": {"amount": amount, "currency": "EUR"}, "side": side});
    json!({
        "document_date": "2026-03-05",
        "posting_date": "2026-03-05",
        "description": "Myyntilasku 2026-0042",
        "postings": [
            line("1700", "debit", "2480.00"),
            line("3000", "credit", "1976.10"),
            line("2939", "credit", "503.90"),
        ]
    })
}

#[test]
fn an_untagged_sale_vat_leg_warns_shows_in_preview_and_blocks_filing() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, None, &["init"], "").0, 0);

    let (code, report, err) = run(home, None, &["voucher", "validate"], &sale().to_string());
    assert_eq!(code, 0, "{err}");
    assert_eq!(report["warnings"][0]["code"], "VAT_POSTING_UNTAGGED");

    let seal = json!({"draft": sale(), "natural_key": "sale-2026-0042"});
    let (code, _, err) = run(home, None, &["voucher", "seal"], &seal.to_string());
    assert_eq!(code, 0, "{err}");
    let (code, started, err) = run(
        home,
        None,
        &["run", "start", "close-period", "--period", "2026-03"],
        "",
    );
    assert_eq!(code, 0, "{err}");
    let run_id = started["run_id"].as_str().unwrap();
    let answer = [
        "run",
        "answer",
        "--run-id",
        run_id,
        "--gate",
        "ApproveClose",
        "--answer",
        "approve",
    ];
    assert_eq!(run(home, None, &answer, "").0, 0);

    let (code, preview, err) = run(
        home,
        None,
        &["file-vat", "preview", "--period", "2026-03"],
        "",
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(preview["sales_vat"], "0.00");
    assert_eq!(preview["vat_reconciliation"]["status"], "unreconciled");
    assert_eq!(
        preview["vat_reconciliation"]["output"]["untagged"]["amount"],
        "503.90"
    );
    assert_eq!(preview["attention"][0]["code"], "VAT_ACCOUNTS_UNRECONCILED");
    assert_eq!(preview["attention"][0]["vouchers"], json!([1]));

    // An empty script: any authority call would fail with a different code.
    let script = home.join("script.json");
    std::fs::write(&script, "{}").unwrap();
    let input = json!({
        "period": "2026-03",
        "sales_vat": {"amount": "0.00", "currency": "EUR"},
        "purchase_vat": {"amount": "0.00", "currency": "EUR"},
        "business_id": "1234567-1",
        "contact_name": "Maija Meikäläinen",
        "contact_phone": "+358401234567",
        "payee_iban": "FI2112345600000785"
    });
    let (code, _, err) = run(
        home,
        Some(&script),
        &["file-vat", "start"],
        &input.to_string(),
    );
    assert_ne!(code, 0);
    let error: Value = serde_json::from_str(&err).unwrap();
    assert_eq!(error["error_code"], "VAT_ACCOUNTS_UNRECONCILED", "{err}");
}
