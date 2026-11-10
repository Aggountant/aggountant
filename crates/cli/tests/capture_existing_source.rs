//! Regression coverage for public source inspection primitives.
//! All sources and accounting events are fictional. The tests execute a fixed
//! shell fixture extracted from the former procedural skill and assert the public
//! voucher list/show contract; economic-event
//! identity remains an agent/user judgment.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::{json, Value};

const PREFLIGHT: &str = include_str!("fixtures/source-preflight.sh");

fn invoke(home: &Path, args: &[&str], stdin: &str) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aggountant"));
    command
        .arg("--home")
        .arg(home)
        .arg("--json")
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
    child.wait_with_output().unwrap()
}

fn success(home: &Path, args: &[&str], stdin: &str) -> Value {
    let output = invoke(home, args, stdin);
    assert!(
        output.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{args:?} wrote stderr");
    serde_json::from_slice(&output.stdout).expect("successful stdout is JSON")
}

fn preflight_template() -> &'static str {
    PREFLIGHT
}

fn attach(home: &Path, bytes: &[u8]) -> String {
    let source = home.join("fictional-source.pdf");
    std::fs::write(&source, bytes).unwrap();
    success(
        home,
        &["artifact", "attach", "--file", source.to_str().unwrap()],
        "",
    )["artifact_ref"]
        .as_str()
        .unwrap()
        .to_string()
}

fn seal(
    home: &Path,
    key: &str,
    document_date: &str,
    posting_date: &str,
    attachment: &str,
) -> Value {
    let input = json!({
        "draft": {
            "document_date": document_date,
            "posting_date": posting_date,
            "description": format!("Fictional service {key}"),
            "attachment_ref": attachment,
            "postings": [
                {"account":"7690","amount":{"amount":"24.00","currency":"EUR"},"side":"debit"},
                {"account":"1910","amount":{"amount":"24.00","currency":"EUR"},"side":"credit"}
            ]
        },
        "natural_key": key,
        "actor": "fictional-source-preflight-test"
    });
    success(home, &["voucher", "seal"], &input.to_string())
}

fn execute_fixture_preflight(home: &Path, attachment: &str) -> (tempfile::TempDir, Output) {
    let output_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        output_dir.path().join("artifact.json"),
        json!({"artifact_ref": attachment}).to_string(),
    )
    .unwrap();
    let output = Command::new("sh")
        .arg("-c")
        .arg(preflight_template())
        .env("AGGOUNTANT_BIN", env!("CARGO_BIN_EXE_aggountant"))
        .env("HOME_DIR", home)
        .env("D", output_dir.path())
        .output()
        .expect("execute fixture preflight");
    (output_dir, output)
}

fn run_fixture_preflight(home: &Path, attachment: &str) -> tempfile::TempDir {
    let (output_dir, output) = execute_fixture_preflight(home, attachment);
    assert!(
        output.status.success(),
        "fixture preflight failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output_dir
}

fn read_json(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn fixture_preflight_reports_zero_candidates_without_creating_a_voucher() {
    let home = tempfile::tempdir().unwrap();
    success(home.path(), &["init"], "");
    let attachment = attach(home.path(), b"fictional unused source bytes");

    let output = run_fixture_preflight(home.path(), &attachment);
    let candidates = read_json(output.path().join("source-candidates.json"));
    assert_eq!(candidates["count"], 0);
    assert_eq!(candidates["truncated"], false);
    assert_eq!(candidates["vouchers"], json!([]));
    assert_eq!(
        success(home.path(), &["voucher", "verify"], "")["voucher_count"],
        0
    );
}

#[test]
fn one_exact_source_returns_the_real_number_and_body_without_changing_the_chain() {
    let home = tempfile::tempdir().unwrap();
    let initialized = success(home.path(), &["init"], "");
    let chart_version = initialized["chart_version"]
        .as_str()
        .expect("init returns a chart version");
    assert!(!chart_version.is_empty());
    let attachment = attach(home.path(), b"fictional exact source bytes");
    let sealed = seal(
        home.path(),
        "fictional:exact",
        "2026-01-11",
        "2026-01-12",
        &attachment,
    );
    let before = success(home.path(), &["voucher", "verify"], "");

    let output = run_fixture_preflight(home.path(), &attachment);
    let candidates = read_json(output.path().join("source-candidates.json"));
    assert_eq!(candidates["count"], 1);
    assert_eq!(candidates["vouchers"][0]["number"], sealed["number"]);
    assert_eq!(
        candidates["vouchers"][0]["match_status"],
        "possible_source_match"
    );
    assert!(candidates.get("duplicate").is_none());

    let shown = read_json(output.path().join("source-candidate-1.json"));
    assert_eq!(shown["number"], sealed["number"]);
    assert_eq!(shown["voucher_hash"], sealed["hash"]);
    assert_eq!(
        shown["body"],
        json!({
            "wire_version": 1,
            "schema_version": 1,
            "record_type": "voucher",
            "number": 1,
            "document_date": "2026-01-11",
            "posting_date": "2026-01-12",
            "description": "Fictional service fictional:exact",
            "attachment_ref": attachment,
            "postings": [
                {"account":"7690","amount":{"amount":"24.00","currency":"EUR"},"side":"debit"},
                {"account":"1910","amount":{"amount":"24.00","currency":"EUR"},"side":"credit"}
            ],
            "chart_version": chart_version
        })
    );

    let after = success(home.path(), &["voucher", "verify"], "");
    assert_eq!(before["ok"], true);
    assert_eq!(before["voucher_count"], 1);
    assert_eq!(after["ok"], true);
    assert_eq!(after["voucher_count"], 1);
    assert_eq!(after["voucher_count"], before["voucher_count"]);
    assert_eq!(after["head_number"], before["head_number"]);
    assert_eq!(after["head_hash"], before["head_hash"]);
}

#[test]
fn source_lookup_does_not_hide_an_old_invoice_behind_the_current_posting_date() {
    let home = tempfile::tempdir().unwrap();
    success(home.path(), &["init"], "");
    let attachment = attach(home.path(), b"fictional recurring invoice source bytes");
    seal(
        home.path(),
        "fictional:old-invoice",
        "2025-12-20",
        "2026-01-05",
        &attachment,
    );

    // A current-event date filter would hide the old same-source voucher.
    let filtered = success(
        home.path(),
        &[
            "voucher",
            "list",
            "--attachment-ref",
            &attachment,
            "--posting-from",
            "2026-05-01",
            "--posting-to",
            "2026-05-31",
        ],
        "",
    );
    assert_eq!(filtered["count"], 0);

    // The fixture preflight intentionally starts with attachment_ref alone.
    let output = run_fixture_preflight(home.path(), &attachment);
    let candidates = read_json(output.path().join("source-candidates.json"));
    assert_eq!(
        candidates.get("filters"),
        Some(&json!({
            "attachment_ref": attachment,
            "posting_from": Value::Null,
            "posting_to": Value::Null
        }))
    );
    assert_eq!(candidates["count"], 1);
    assert_eq!(candidates["vouchers"][0]["posting_date"], "2026-01-05");
    let shown = read_json(output.path().join("source-candidate-1.json"));
    assert_eq!(shown["body"]["posting_date"], "2026-01-05");
}

#[test]
fn fixture_preflight_stops_when_the_bounded_candidate_list_is_truncated() {
    let home = tempfile::tempdir().unwrap();
    success(home.path(), &["init"], "");
    let attachment = attach(home.path(), b"fictional heavily reused source bytes");
    for index in 1..=101 {
        seal(
            home.path(),
            &format!("fictional:truncated-{index}"),
            "2026-01-01",
            "2026-01-02",
            &attachment,
        );
    }

    let (output_dir, output) = execute_fixture_preflight(home.path(), &attachment);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "source candidate list is truncated; stop and ask before capture\n"
    );
    let candidates = read_json(output_dir.path().join("source-candidates.json"));
    assert_eq!(candidates["count"], 100);
    assert_eq!(candidates["truncated"], true);
    assert!(!output_dir.path().join("source-candidate-numbers").exists());
    assert_eq!(
        success(home.path(), &["voucher", "verify"], "")["voucher_count"],
        101
    );
}

#[test]
fn two_same_source_candidates_remain_distinct_truthful_canonical_views() {
    let home = tempfile::tempdir().unwrap();
    success(home.path(), &["init"], "");
    let attachment = attach(home.path(), b"fictional source used by two events");
    let first = seal(
        home.path(),
        "fictional:first-event",
        "2026-02-01",
        "2026-02-02",
        &attachment,
    );
    let second = seal(
        home.path(),
        "fictional:second-event",
        "2026-03-01",
        "2026-03-02",
        &attachment,
    );

    let output = run_fixture_preflight(home.path(), &attachment);
    let candidates = read_json(output.path().join("source-candidates.json"));
    assert_eq!(candidates["count"], 2);
    assert_eq!(candidates["truncated"], false);
    assert_eq!(candidates["vouchers"][0]["number"], first["number"]);
    assert_eq!(candidates["vouchers"][1]["number"], second["number"]);
    assert!(candidates.get("duplicate").is_none());

    let first_body = read_json(output.path().join("source-candidate-1.json"));
    let second_body = read_json(output.path().join("source-candidate-2.json"));
    assert_eq!(first_body["body"]["posting_date"], "2026-02-02");
    assert_eq!(second_body["body"]["posting_date"], "2026-03-02");
    assert_ne!(first_body["voucher_hash"], second_body["voucher_hash"]);
    assert_eq!(
        first_body["body"]["attachment_ref"],
        second_body["body"]["attachment_ref"]
    );
}
