//! S5 done-criterion: the `capture-receipt` (RF1) and `hunt-receipts` (RF2) Runs run
//! **end to end via the CLI**, including the F13 obligation blocking `close-period`. The
//! authority transport is supplied through `$AGGOUNTANT_BROKER_SCRIPT` (v1 ships no live
//! adapter).

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
}

fn run(
    home: &std::path::Path,
    script: Option<&std::path::Path>,
    args: &[&str],
    stdin: &str,
) -> (i32, String, String) {
    let mut cmd = bin();
    cmd.arg("--home").arg(home).arg("--json");
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

const CAPTURE_INPUT: &str = r#"{
  "period": "2026-Q1",
  "topic": "vat-deductibility",
  "question": "Is the VAT on this passenger-car lease deductible at 70%?",
  "facts": [{"key": "expense_nature", "value": "passenger-car leasing"}],
  "raw_source": "FICTIONAL RECEIPT FIXTURE",
  "nature": "passenger-car-mixed-use",
  "receipt_vat": {"amount": "124.00", "currency": "EUR"},
  "claimed_business_share": "0.70",
  "document_date": "2026-03-14",
  "obligation": {"kind": "ajopaivakirja-missing", "description": "ajopäiväkirja puuttuu", "deadline": "2026-04-30"},
  "knowledge_slug": "passenger-car-lease-vat",
  "draft": {
    "document_date": "2026-03-14", "posting_date": "2026-03-14", "description": "Auton leasing",
    "postings": [
      {"account": "7690", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "debit"},
      {"account": "1910", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "credit"}
    ]
  },
  "tool_results": []
}"#;

const CONSULT_SCRIPT: &str = r#"{"consult-llm":[{"status":200,"body":{"type":"final","advice":{"summary":"Use the deterministic eligibility assessment.","citations":["fictional policy fixture"]}}}]}"#;

const NO_OBLIGATION_CAPTURE_INPUT: &str = r#"{
  "period": "2026-Q1",
  "topic": "vat-deductibility",
  "question": "Is VAT fully deductible for this fictional business service?",
  "facts": [{"key": "fixture", "value": "fictional occupational service"}],
  "raw_source": "FICTIONAL RECEIPT FIXTURE",
  "nature": "fully-deductible",
  "receipt_vat": {"amount": "0.12", "currency": "EUR"},
  "claimed_business_share": "1",
  "document_date": "2026-02-28",
  "knowledge_slug": "fictional-service-vat",
  "draft": {
    "document_date": "2026-02-28", "posting_date": "2026-03-10", "description": "Fictional occupational service",
    "postings": [
      {"account": "5500", "amount": {"amount": "23.34", "currency": "EUR"}, "side": "debit"},
      {"account": "1763", "amount": {"amount": "0.12", "currency": "EUR"}, "side": "debit"},
      {"account": "1910", "amount": {"amount": "23.46", "currency": "EUR"}, "side": "credit"}
    ]
  },
  "tool_results": []
}"#;

const CONTRADICTORY_CAPTURE_INPUT: &str = r#"{
  "period": "2026-Q1",
  "topic": "vat-deductibility",
  "question": "Is VAT deductible for this synthetic classification?",
  "facts": [{"key": "fixture", "value": "synthetic passenger-car classification"}],
  "raw_source": "FICTIONAL RECEIPT FIXTURE",
  "nature": "passenger-car-mixed-use",
  "receipt_vat": {"amount": "65.33", "currency": "EUR"},
  "claimed_business_share": "0.70",
  "document_date": "2026-03-14",
  "obligation": {"kind": "fixture-follow-up", "description": "fictional follow-up", "deadline": "2026-04-30"},
  "knowledge_slug": "fictional-capture-vat",
  "draft": {
    "document_date": "2026-03-14", "posting_date": "2026-03-14", "description": "Fictional classified purchase",
    "postings": [
      {"account": "7690", "amount": {"amount": "416.88", "currency": "EUR"}, "side": "debit"},
      {"account": "1763", "amount": {"amount": "65.33", "currency": "EUR"}, "side": "debit"},
      {"account": "1910", "amount": {"amount": "482.21", "currency": "EUR"}, "side": "credit"}
    ]
  },
  "tool_results": []
}"#;

#[test]
fn capture_receipt_end_to_end_and_obligation_blocks_close_via_cli() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let script = home.join("broker.json");
    std::fs::write(&script, CONSULT_SCRIPT).unwrap();

    assert_eq!(run(home, None, &["init"], "").0, 0);

    // Start → parks on ApproveConsultDisclosure.
    let (code, out, err) = run(
        home,
        Some(&script),
        &["capture-receipt", "start"],
        CAPTURE_INPUT,
    );
    assert_eq!(code, 0, "start failed: {err}");
    let run_id = serde_json::from_str::<Value>(&out).unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Approve disclosure → drives the consult, runs eligibility, opens ApproveVoucher.
    let (code, _, err) = run(
        home,
        Some(&script),
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
    assert_eq!(code, 0, "disclosure approve failed: {err}");

    // Approve voucher → seals + opens the obligation.
    let (code, _, err) = run(
        home,
        Some(&script),
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
    assert_eq!(code, 0, "voucher approve failed: {err}");

    // The obligation now blocks close-period (exit 1, OPEN_OBLIGATION).
    let (code, _, err) = run(
        home,
        None,
        &["run", "start", "close-period", "--period", "2026-Q1"],
        "",
    );
    assert_eq!(code, 1, "close-period should be blocked");
    assert!(
        err.contains("OPEN_OBLIGATION"),
        "expected OPEN_OBLIGATION: {err}"
    );

    // Resolve through the public CLI, then the same period can begin closing.
    let (code, out, err) = run(home, None, &["obligation", "list"], "");
    assert_eq!(code, 0, "obligation list failed: {err}");
    let obligations: Value = serde_json::from_str(&out).unwrap();
    let obligation_id = obligations["obligations"][0]["id"].as_str().unwrap();
    let (code, _, err) = run(
        home,
        None,
        &["obligation", "resolve", "--id", obligation_id],
        "",
    );
    assert_eq!(code, 0, "obligation resolve failed: {err}");
    let (code, _, err) = run(
        home,
        None,
        &["run", "start", "close-period", "--period", "2026-Q1"],
        "",
    );
    assert_eq!(code, 0, "resolved obligation still blocked close: {err}");

    // Approve the knowledge gate → the run completes and the knowledge doc is written.
    let (code, _, err) = run(
        home,
        Some(&script),
        &[
            "capture-receipt",
            "answer",
            "--run-id",
            &run_id,
            "--gate",
            "ApproveKnowledgePR",
            "--answer",
            "approve",
        ],
        "",
    );
    assert_eq!(code, 0, "knowledge approve failed: {err}");
    let (_, out, _) = run(home, None, &["knowledge", "list"], "");
    assert!(
        out.contains("passenger-car-lease-vat"),
        "knowledge not written: {out}"
    );
}

#[test]
fn capture_receipt_without_obligation_seals_and_allows_close_via_cli() {
    let null_input = NO_OBLIGATION_CAPTURE_INPUT.replacen(
        "  \"knowledge_slug\"",
        "  \"obligation\": null,\n  \"knowledge_slug\"",
        1,
    );
    for (shape, input) in [
        ("missing", NO_OBLIGATION_CAPTURE_INPUT),
        ("null", null_input.as_str()),
    ] {
        let home = tempfile::tempdir().unwrap();
        let home = home.path();
        let script = home.join("broker.json");
        std::fs::write(&script, CONSULT_SCRIPT).unwrap();
        assert_eq!(run(home, None, &["init"], "").0, 0);

        let (code, out, err) = run(home, Some(&script), &["capture-receipt", "start"], input);
        assert_eq!(code, 0, "start with {shape} obligation failed: {err}");
        let run_id = serde_json::from_str::<Value>(&out).unwrap()["run_id"]
            .as_str()
            .unwrap()
            .to_string();
        for gate in ["ApproveConsultDisclosure", "ApproveVoucher"] {
            let (code, _, err) = run(
                home,
                Some(&script),
                &[
                    "capture-receipt",
                    "answer",
                    "--run-id",
                    &run_id,
                    "--gate",
                    gate,
                    "--answer",
                    "approve",
                ],
                "",
            );
            assert_eq!(code, 0, "{shape} obligation: {gate} failed: {err}");
        }

        let (code, out, err) = run(home, None, &["voucher", "verify"], "");
        assert_eq!(code, 0, "voucher verify failed: {err}");
        assert_eq!(
            serde_json::from_str::<Value>(&out).unwrap()["voucher_count"],
            1
        );
        let (code, out, err) = run(home, None, &["obligation", "list"], "");
        assert_eq!(code, 0, "obligation list failed: {err}");
        assert!(serde_json::from_str::<Value>(&out).unwrap()["obligations"]
            .as_array()
            .unwrap()
            .is_empty());
        let (code, _, err) = run(
            home,
            None,
            &["run", "start", "close-period", "--period", "2026-Q1"],
            "",
        );
        assert_eq!(code, 0, "{shape} obligation should allow close: {err}");
    }
}

#[test]
fn capture_receipt_cli_rejects_malformed_obligation_object() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    assert_eq!(run(home, None, &["init"], "").0, 0);
    let malformed = NO_OBLIGATION_CAPTURE_INPUT.replacen(
        "  \"knowledge_slug\"",
        "  \"obligation\": {\"kind\": \"fictional-follow-up\"},\n  \"knowledge_slug\"",
        1,
    );
    let (code, out, err) = run(home, None, &["capture-receipt", "start"], &malformed);
    assert_eq!(code, 1);
    assert!(out.is_empty());
    let error: Value = serde_json::from_str(&err).unwrap();
    assert_eq!(error["error_code"], "INVALID_INPUT");
}

#[test]
fn capture_receipt_cli_returns_actionable_mismatch_and_seals_nothing() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let script = home.join("broker.json");
    std::fs::write(&script, CONSULT_SCRIPT).unwrap();
    assert_eq!(run(home, None, &["init"], "").0, 0);

    let (code, out, err) = run(
        home,
        Some(&script),
        &["capture-receipt", "start"],
        CONTRADICTORY_CAPTURE_INPUT,
    );
    assert_eq!(code, 0, "start failed: {err}");
    let run_id = serde_json::from_str::<Value>(&out).unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        run(
            home,
            Some(&script),
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
        )
        .0,
        0
    );

    let (code, out, err) = run(
        home,
        Some(&script),
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
    assert_eq!(code, 1, "mismatch is caller-actionable");
    assert!(out.is_empty(), "errors belong on stderr: {out}");
    let error: Value = serde_json::from_str(&err).unwrap();
    assert_eq!(error["error_code"], "CAPTURE_VAT_MISMATCH");
    assert_eq!(error["error_kind"], "user");
    assert!(error["message"].as_str().unwrap().contains("start a new"));

    let (code, out, err) = run(home, None, &["voucher", "verify"], "");
    assert_eq!(code, 0, "verify failed: {err}");
    assert_eq!(
        serde_json::from_str::<Value>(&out).unwrap()["voucher_count"],
        0
    );
}

const HUNT_INPUT: &str = r#"{
  "transactions": [{
    "bank_account": "FICTIONAL-BANK-ACCOUNT", "txn_id": "t1", "counterparty": "Example Vendor",
    "amount": {"amount": "124.00", "currency": "EUR"}, "date": "2026-03-14", "evidence_status": "missing",
    "proposed_draft": {
      "document_date": "2026-03-14", "posting_date": "2026-03-14", "description": "Verkkokauppa osto",
      "postings": [
        {"account": "7690", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "debit"},
        {"account": "1910", "amount": {"amount": "124.00", "currency": "EUR"}, "side": "credit"}
      ]
    }
  }]
}"#;

const PULL_SCRIPT: &str =
    r#"{"email-pull":[{"status":200,"body":{"document":{"receipt":"WK-1","amount":"124.00"}}}]}"#;

#[test]
fn hunt_receipts_matches_and_seals_per_item_via_cli() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path();
    let script = home.join("broker.json");
    std::fs::write(&script, PULL_SCRIPT).unwrap();

    assert_eq!(run(home, None, &["init"], "").0, 0);

    let (code, out, err) = run(home, Some(&script), &["hunt-receipts", "start"], HUNT_INPUT);
    assert_eq!(code, 0, "hunt start failed: {err}");
    let run_id = serde_json::from_str::<Value>(&out).unwrap()["run_id"]
        .as_str()
        .unwrap()
        .to_string();

    // A typed intake candidate exists.
    let (_, out, _) = run(home, None, &["intake", "list"], "");
    assert!(out.contains("\"t1\""), "no intake candidate: {out}");

    // Approve the match → seals the voucher (per-item).
    let (code, _, err) = run(
        home,
        Some(&script),
        &[
            "hunt-receipts",
            "answer",
            "--run-id",
            &run_id,
            "--gate",
            "ApproveMatch:FICTIONAL-BANK-ACCOUNT:t1",
            "--answer",
            "approve",
        ],
        "",
    );
    assert_eq!(code, 0, "match approve failed: {err}");

    // The chain verifies (one match voucher sealed).
    let (code, out, _) = run(home, None, &["voucher", "verify"], "");
    assert_eq!(code, 0);
    assert_eq!(
        serde_json::from_str::<Value>(&out).unwrap()["voucher_count"],
        1
    );
}
