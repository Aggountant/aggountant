//! Regression coverage for the capture Run's single-primary-source contract.
//! All receipt content is fictional. The artifact byte read is deliberately a read-only
//! check of the current IdentityCipher/layout, not a public retrieval API guarantee.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::{json, Value};

const CAPTURE_INPUT: &str = include_str!("fixtures/capture-input.json");
const SOURCE_BYTES: &[u8] = b"%PDF-1.4\nFICTIONAL DOMESTIC SERVICE RECEIPT\nNET EUR 100.00\nVAT EUR 25.50\nTOTAL EUR 125.50\n";
const CONSULT_SCRIPT: &str = r#"{"consult-llm":[{"status":200,"body":{"type":"final","advice":{"summary":"Apply the deterministic eligibility assessment to the fictional domestic fixture.","citations":["fictional policy fixture"]}}}]}"#;

fn invoke(
    home: &std::path::Path,
    script: Option<&std::path::Path>,
    args: &[&str],
    stdin: &str,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aggountant"));
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
    child.wait_with_output().unwrap()
}

fn success_json(output: Output, step: &str) -> Value {
    assert!(
        output.status.success(),
        "{step} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{step} wrote stderr");
    serde_json::from_slice(&output.stdout).expect("successful stdout is JSON")
}

fn capture_example() -> Value {
    serde_json::from_str(CAPTURE_INPUT).expect("valid capture input fixture")
}

fn count_files_excluding(path: &std::path::Path, needle: &[u8]) -> usize {
    assert!(
        !needle.is_empty(),
        "persistence probe needle must be nonempty"
    );
    let mut count = 0;
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            count += count_files_excluding(&path, needle);
        } else {
            let bytes = std::fs::read(&path).unwrap();
            assert!(
                !bytes.windows(needle.len()).any(|window| window == needle),
                "source bytes persisted in Run file {}",
                path.display()
            );
            count += 1;
        }
    }
    count
}

fn assert_run_excludes(home: &std::path::Path, run_id: &str, needles: &[&[u8]]) {
    let run_dir = home.join("process").join("runs").join(run_id);
    assert!(
        run_dir.is_dir(),
        "expected Run directory {}",
        run_dir.display()
    );
    for needle in needles {
        let scanned = count_files_excluding(&run_dir, needle);
        assert!(scanned > 0, "persistence probe scanned no Run files");
    }
}

fn verify_empty(home: &std::path::Path) {
    let verification = success_json(
        invoke(home, None, &["voucher", "verify"], ""),
        "negative voucher verify",
    );
    assert_eq!(verification["voucher_count"], 0);
}

fn assert_validator_rejects_reference(reference: &str, label: &str) {
    let home = tempfile::tempdir().unwrap();
    success_json(invoke(home.path(), None, &["init"], ""), "negative init");

    let mut input = capture_example();
    input["draft"]["attachment_ref"] = json!(reference);
    let output = invoke(
        home.path(),
        None,
        &["voucher", "validate"],
        &input["draft"].to_string(),
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "{label} unexpectedly validated"
    );
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).expect("JSON validator error");
    assert_eq!(
        error["error_code"], "ARTIFACT_NOT_FOUND",
        "{label}: {error}"
    );
    verify_empty(home.path());
}

#[test]
fn capture_example_archives_and_binds_one_fictional_original_end_to_end() {
    let home = tempfile::tempdir().unwrap();
    let source = home.path().join("fictional-original.pdf");
    let broker = home.path().join("broker.json");
    std::fs::write(&source, SOURCE_BYTES).unwrap();
    std::fs::write(&broker, CONSULT_SCRIPT).unwrap();

    success_json(invoke(home.path(), None, &["init"], ""), "init");
    let attached = success_json(
        invoke(
            home.path(),
            None,
            &["artifact", "attach", "--file", source.to_str().unwrap()],
            "",
        ),
        "artifact attach",
    );
    let artifact_ref = attached["artifact_ref"].as_str().unwrap().to_string();
    assert_eq!(attached["size"], SOURCE_BYTES.len());

    // Use the fixed capture input fixture with its runtime attachment reference.
    let mut input = capture_example();
    assert_eq!(input["claimed_business_share"], "1");
    assert!(input.get("obligation").is_none());
    assert_eq!(
        input["draft"]["attachment_ref"],
        "<ARTIFACT_REF_FROM_ATTACH>"
    );
    input["draft"]["attachment_ref"] = json!(artifact_ref);

    let validation = success_json(
        invoke(
            home.path(),
            None,
            &["voucher", "validate"],
            &input["draft"].to_string(),
        ),
        "voucher validate",
    );
    assert_eq!(validation["ok"], true);
    assert_eq!(validation["attachment_present"], true);

    let started = success_json(
        invoke(
            home.path(),
            Some(&broker),
            &["capture-receipt", "start"],
            &input.to_string(),
        ),
        "capture start",
    );
    let run_id = started["run_id"].as_str().unwrap().to_string();
    let raw_source = input["raw_source"].as_str().unwrap().as_bytes();
    assert_run_excludes(home.path(), &run_id, &[raw_source, SOURCE_BYTES]);
    for gate in [
        "ApproveConsultDisclosure",
        "ApproveVoucher",
        "ApproveKnowledgePR",
    ] {
        success_json(
            invoke(
                home.path(),
                Some(&broker),
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
            ),
            gate,
        );
    }

    let verification = success_json(
        invoke(home.path(), None, &["voucher", "verify"], ""),
        "voucher verify",
    );
    assert_eq!(verification["voucher_count"], 1);

    let obligations = success_json(
        invoke(home.path(), None, &["obligation", "list"], ""),
        "obligation list",
    );
    assert!(obligations["obligations"].as_array().unwrap().is_empty());

    // The public read-only CLI proves that the sealed canonical body retained the exact ref.
    let sealed = success_json(
        invoke(home.path(), None, &["voucher", "show", "1"], ""),
        "voucher show",
    );
    assert_eq!(sealed["body"]["number"], 1);
    assert_eq!(sealed["body"]["attachment_ref"], artifact_ref);
    assert_run_excludes(home.path(), &run_id, &[raw_source, SOURCE_BYTES]);

    // There is currently no public artifact getter. Read before any possible re-attach so
    // this cannot recreate and mask a missing blob. This byte comparison intentionally
    // records only the current IdentityCipher and sharded-layout behavior for our fixture.
    let (shard, rest) = artifact_ref.split_at(2);
    let stored = std::fs::read(
        home.path()
            .join("artifacts")
            .join(shard)
            .join(format!("{rest}.blob")),
    )
    .unwrap();
    assert_eq!(stored, SOURCE_BYTES);
}

#[test]
fn public_paths_prevent_invalid_references_from_producing_a_voucher() {
    // The public validator rejects a well-formed ref whose blob is missing.
    let missing_ref = "0".repeat(64);
    assert_validator_rejects_reference(&missing_ref, "missing artifact");

    // The general backend still allows a well-formed missing ref through start. Exercise the
    // actual path: sealing refuses it, and no voucher is produced. This test does not claim a
    // new universal start-time law; its pre-start validator is the bounded protection.
    let missing_home = tempfile::tempdir().unwrap();
    let missing_broker = missing_home.path().join("broker.json");
    std::fs::write(&missing_broker, CONSULT_SCRIPT).unwrap();
    success_json(
        invoke(missing_home.path(), None, &["init"], ""),
        "missing-ref init",
    );
    let mut missing_input = capture_example();
    missing_input["draft"]["attachment_ref"] = json!(missing_ref);
    let started = success_json(
        invoke(
            missing_home.path(),
            Some(&missing_broker),
            &["capture-receipt", "start"],
            &missing_input.to_string(),
        ),
        "missing-ref capture start",
    );
    let run_id = started["run_id"].as_str().unwrap();
    success_json(
        invoke(
            missing_home.path(),
            Some(&missing_broker),
            &[
                "capture-receipt",
                "answer",
                "--run-id",
                run_id,
                "--gate",
                "ApproveConsultDisclosure",
                "--answer",
                "approve",
            ],
            "",
        ),
        "missing-ref disclosure approval",
    );
    let rejected = invoke(
        missing_home.path(),
        Some(&missing_broker),
        &[
            "capture-receipt",
            "answer",
            "--run-id",
            run_id,
            "--gate",
            "ApproveVoucher",
            "--answer",
            "approve",
        ],
        "",
    );
    assert_eq!(rejected.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&rejected.stderr).unwrap();
    assert_eq!(error["error_code"], "ARTIFACT_NOT_FOUND");
    verify_empty(missing_home.path());

    // A one-nibble mutation of a real returned reference is also absent and rejected.
    let home = tempfile::tempdir().unwrap();
    success_json(invoke(home.path(), None, &["init"], ""), "mutation init");
    let source = home.path().join("fictional-original.pdf");
    std::fs::write(&source, SOURCE_BYTES).unwrap();
    let attached = success_json(
        invoke(
            home.path(),
            None,
            &["artifact", "attach", "--file", source.to_str().unwrap()],
            "",
        ),
        "mutation attach",
    );
    let mut mutated = attached["artifact_ref"].as_str().unwrap().to_string();
    let replacement = if mutated.ends_with('0') { '1' } else { '0' };
    mutated.pop();
    mutated.push(replacement);
    let mut input = capture_example();
    input["draft"]["attachment_ref"] = json!(mutated);
    let rejected = invoke(
        home.path(),
        None,
        &["voucher", "validate"],
        &input["draft"].to_string(),
    );
    assert_eq!(rejected.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&rejected.stderr).unwrap();
    assert_eq!(error["error_code"], "ARTIFACT_NOT_FOUND");
    verify_empty(home.path());

    // A wrong wire value is rejected by the actual capture start deserializer.
    let malformed_home = tempfile::tempdir().unwrap();
    success_json(
        invoke(malformed_home.path(), None, &["init"], ""),
        "malformed init",
    );
    let mut malformed = capture_example();
    malformed["draft"]["attachment_ref"] = json!("not-an-artifact-hash");
    let rejected = invoke(
        malformed_home.path(),
        None,
        &["capture-receipt", "start"],
        &malformed.to_string(),
    );
    assert_eq!(rejected.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&rejected.stderr).unwrap();
    assert_eq!(error["error_code"], "INVALID_INPUT");
    verify_empty(malformed_home.path());
}
