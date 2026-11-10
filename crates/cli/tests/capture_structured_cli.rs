use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::{json, Value};

fn invoke(home: &Path, args: &[&str], input: &Value) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aggountant"));
    command
        .arg("--home")
        .arg(home)
        .arg("--json")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.to_string().as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn success(home: &Path, args: &[&str], input: &Value) -> Value {
    let output = invoke(home, args, input);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn failure(home: &Path, args: &[&str], input: &Value) -> Value {
    let output = invoke(home, args, input);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    serde_json::from_slice(&output.stderr).unwrap()
}

fn receipt() -> Value {
    json!({
        "natural_key": "receipt:cli-fictional-1",
        "actor": "cli-test-agent",
        "source": {
            "amount": {"amount":"125.50","currency":"EUR"},
            "document_date": "2026-07-15",
            "description": "Fictional domestic service"
        },
        "proposal": {
            "posting_date": "2026-07-16",
            "net_account": "7690",
            "contra_account": "1910",
            "vat": {"type":"domestic-input","rate_class":"domestic-standard"}
        }
    })
}

#[test]
fn dry_run_then_one_bound_approval_seals_and_retry_replays() {
    let home = tempfile::tempdir().unwrap();
    success(home.path(), &["init"], &json!(null));

    let preview = success(
        home.path(),
        &["capture-receipt", "record", "--dry-run"],
        &receipt(),
    );
    assert_eq!(preview["dry_run"], true);
    assert_eq!(preview["projection_valid"], true);
    assert_eq!(preview["ledger_preflight_valid"], true);
    assert!(preview.get("number").is_none());
    assert!(preview.get("would_seal").is_none());
    let hash = preview["preparation_hash"].as_str().unwrap();

    let missing_approval = failure(home.path(), &["capture-receipt", "record"], &receipt());
    assert_eq!(missing_approval["error_code"], "APPROVAL_REQUIRED");

    let recorded = success(
        home.path(),
        &[
            "capture-receipt",
            "record",
            "--approved-preparation-hash",
            hash,
        ],
        &receipt(),
    );
    assert_eq!(recorded["dry_run"], false);
    assert_eq!(recorded["replayed"], false);
    assert!(recorded["source_fingerprint"].is_string());

    let retry = success(
        home.path(),
        &[
            "capture-receipt",
            "record",
            "--approved-preparation-hash",
            hash,
        ],
        &receipt(),
    );
    assert_eq!(retry["number"], recorded["number"]);
    assert_eq!(retry["replayed"], true);
}

#[test]
fn missing_date_unsupported_treatment_and_unknown_field_are_deterministic_refusals() {
    let home = tempfile::tempdir().unwrap();
    success(home.path(), &["init"], &json!(null));

    let mut missing = receipt();
    missing["source"]
        .as_object_mut()
        .unwrap()
        .remove("document_date");
    let error = failure(
        home.path(),
        &["capture-receipt", "record", "--dry-run"],
        &missing,
    );
    assert_eq!(error["error_code"], "CAPTURE_FACT_REQUIRED");

    let mut unsupported = receipt();
    unsupported["proposal"]["vat"] = json!({
        "type": "unsupported",
        "treatment": "margin-scheme",
        "reason": "needs separate source facts"
    });
    let error = failure(
        home.path(),
        &["capture-receipt", "record", "--dry-run"],
        &unsupported,
    );
    assert_eq!(error["error_code"], "CAPTURE_UNSUPPORTED_TREATMENT");

    let mut unknown = receipt();
    unknown["source"]["guessed_vat"] = json!(true);
    let error = failure(
        home.path(),
        &["capture-receipt", "record", "--dry-run"],
        &unknown,
    );
    assert_eq!(error["error_code"], "INVALID_INPUT");
    assert!(error["message"].as_str().unwrap().contains("unknown field"));
}
