//! Regression coverage for pre-seal diagnostics produced from an approved capture draft.
//! All fixtures are fictional and use the local scripted consult transport.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

const CONSULT_SCRIPT: &str = r#"{"consult-llm":[{"status":200,"body":{"type":"final","advice":{"summary":"Use the deterministic eligibility assessment.","citations":["fictional policy fixture"]}}}]}"#;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
}

fn run(home: &Path, script: Option<&Path>, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut command = bin();
    command
        .arg("--home")
        .arg(home)
        .arg("--json")
        .env_remove("AGGOUNTANT_BROKER_SCRIPT");
    if let Some(script) = script {
        command.env("AGGOUNTANT_BROKER_SCRIPT", script);
    }
    command
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn aggountant");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn success(home: &Path, args: &[&str], stdin: &str) -> Value {
    let (code, stdout, stderr) = run(home, None, args, stdin);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    serde_json::from_str(&stdout).unwrap()
}

fn unsupported_capture_input() -> Value {
    json!({
        "period": "2026-Q2",
        "topic": "vat-deductibility",
        "question": "How is this fictional May purchase recorded?",
        "facts": [{"key": "fixture", "value": "fictional May purchase"}],
        "raw_source": "FICTIONAL RECEIPT FIXTURE",
        "nature": "fully-deductible",
        "receipt_vat": {"amount": "25.50", "currency": "EUR"},
        "claimed_business_share": "1",
        "document_date": "2026-05-31",
        "knowledge_slug": "fictional-may-purchase",
        "draft": {
            "document_date": "2026-05-31",
            "posting_date": "2026-05-31",
            "description": "Fictional unsupported VAT-account movement",
            "postings": [
                {"account": "2939", "amount": {"amount": "125.50", "currency": "EUR"}, "side": "debit"},
                {"account": "1910", "amount": {"amount": "125.50", "currency": "EUR"}, "side": "credit"}
            ]
        },
        "tool_results": []
    })
}

fn seal_existing_domestic_voucher(home: &Path) {
    let input = json!({
        "draft": {
            "document_date": "2026-04-30",
            "posting_date": "2026-04-30",
            "description": "Fictional domestic service",
            "postings": [
                {"account": "5500", "amount": {"amount": "100.00", "currency": "EUR"}, "side": "debit"},
                {"account": "1763", "amount": {"amount": "25.50", "currency": "EUR"}, "side": "debit"},
                {"account": "1910", "amount": {"amount": "125.50", "currency": "EUR"}, "side": "credit"}
            ]
        },
        "natural_key": "fictional:existing-domestic-voucher",
        "actor": "fictional-cli-test"
    });
    let sealed = success(home, &["voucher", "seal"], &input.to_string());
    assert_eq!(sealed["number"], 1);
}

fn attempt_capture(home: &Path, script: &Path) -> Value {
    let input = unsupported_capture_input().to_string();
    let (code, stdout, stderr) = run(home, Some(script), &["capture-receipt", "start"], &input);
    assert_eq!(code, 0, "start failed: {stderr}");
    let run_id = serde_json::from_str::<Value>(&stdout).unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (code, stdout, stderr) = run(
        home,
        Some(script),
        &[
            "capture-receipt",
            "answer",
            "--run-id",
            &run_id,
            "--gate",
            "ApproveConsultDisclosure",
            "--answer",
            "approve",
        ],
        "",
    );
    assert_eq!(code, 0, "disclosure approval failed: {stderr}");
    assert!(!stdout.is_empty());

    let (code, stdout, stderr) = run(
        home,
        Some(script),
        &[
            "capture-receipt",
            "answer",
            "--run-id",
            &run_id,
            "--gate",
            "ApproveVoucher",
            "--answer",
            "approve",
        ],
        "",
    );
    assert_eq!(code, 1, "draft projection is caller-actionable");
    assert!(stdout.is_empty(), "errors belong on stderr: {stdout}");
    serde_json::from_str(&stderr).unwrap()
}

#[test]
fn approved_draft_diagnostic_cannot_be_mistaken_for_existing_voucher_one() {
    let mut messages = Vec::new();

    for existing_voucher in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let home = home.path();
        let script = home.join("broker.json");
        std::fs::write(&script, CONSULT_SCRIPT).unwrap();
        success(home, &["init"], "");

        if existing_voucher {
            seal_existing_domestic_voucher(home);
        }
        let verify_before = success(home, &["voucher", "verify"], "");
        let show_before = existing_voucher.then(|| success(home, &["voucher", "show", "1"], ""));

        let error = attempt_capture(home, &script);
        assert_eq!(error["error_code"], "INVALID_ANSWER");
        assert_eq!(error["error_kind"], "user");
        let message = error["message"].as_str().unwrap();
        assert!(
            message.contains("APPROVED DRAFT"),
            "missing draft context: {message}"
        );
        assert!(
            message.contains("temporary projection identifier"),
            "missing projection context: {message}"
        );
        assert!(
            message.contains("NOT an existing ledger voucher"),
            "missing ledger distinction: {message}"
        );
        assert!(
            message.contains("VAT_STRUCTURE"),
            "underlying code lost: {message}"
        );
        assert!(
            message.contains("voucher 1"),
            "underlying diagnostic lost: {message}"
        );
        assert!(
            message.contains("2939"),
            "actionable account lost: {message}"
        );
        messages.push(message.to_string());

        let verify_after = success(home, &["voucher", "verify"], "");
        assert_eq!(verify_after, verify_before, "voucher count/hash changed");
        assert_eq!(
            verify_after["voucher_count"],
            if existing_voucher { 1 } else { 0 },
            "a new voucher was unexpectedly sealed"
        );
        if let Some(show_before) = show_before {
            let show_after = success(home, &["voucher", "show", "1"], "");
            assert_eq!(
                show_after, show_before,
                "existing canonical voucher changed"
            );
        }
    }

    assert_eq!(
        messages[0], messages[1],
        "ledger contents changed draft context"
    );
}
