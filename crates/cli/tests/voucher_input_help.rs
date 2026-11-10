use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

fn command(config_home: &std::path::Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aggountant"));
    command
        .env("HOME", config_home)
        .env("APPDATA", config_home)
        .env("USERPROFILE", config_home)
        .env_remove("AGGOUNTANT_HOME")
        .env_remove("XDG_CONFIG_HOME");
    command
}

fn run(config_home: &std::path::Path, args: &[&str], stdin: &str) -> Output {
    let mut child = command(config_home)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn aggountant");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn success_stdout(output: Output) -> String {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn help(config_home: &std::path::Path, action: &str, help_flag: &str) -> String {
    success_stdout(run(config_home, &["voucher", action, help_flag], ""))
}

fn printed_json(help: &str) -> String {
    let after_open = help
        .split_once("```json\n")
        .expect("help has a JSON example")
        .1;
    after_open
        .split_once("\n```")
        .expect("help closes the JSON example")
        .0
        .to_string()
}

fn json_stdout(output: Output) -> Value {
    let stdout = success_stdout(output);
    serde_json::from_str(&stdout).expect("stdout is JSON")
}

fn assert_invalid_input(output: Output) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).expect("JSON error on stderr");
    assert_eq!(error["error_code"], "INVALID_INPUT");
}

#[test]
fn printed_help_examples_execute_and_seal_replays_idempotently() {
    let config_home = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let home = data_home.path().to_str().unwrap();

    let draft_help = help(config_home.path(), "draft", "--help");
    let seal_help = help(config_home.path(), "seal", "--help");
    let draft_input = printed_json(&draft_help);
    let seal_input = printed_json(&seal_help);

    assert!(draft_help.contains("aggountant --home ./accounting --json voucher draft < draft.json"));
    assert!(seal_help.contains("aggountant --home ./accounting --json voucher seal < seal.json"));
    assert!(draft_help.contains("artifact attach --file ./receipt.pdf"));
    assert!(seal_help.contains("artifact attach --file ./receipt.pdf"));

    let draft = json_stdout(run(
        config_home.path(),
        &["--home", home, "--json", "voucher", "draft"],
        &draft_input,
    ));
    assert_eq!(draft["postings"].as_array().unwrap().len(), 2);

    json_stdout(run(
        config_home.path(),
        &["--home", home, "--json", "init"],
        "",
    ));
    let validation = json_stdout(run(
        config_home.path(),
        &["--home", home, "--json", "voucher", "validate"],
        &draft.to_string(),
    ));
    assert_eq!(validation["ok"], true);
    assert_eq!(validation["attachment_present"], false);

    let first = json_stdout(run(
        config_home.path(),
        &["--home", home, "--json", "voucher", "seal"],
        &seal_input,
    ));
    assert_eq!(first["number"], 1);
    assert_eq!(first["replayed"], false);

    let replay = json_stdout(run(
        config_home.path(),
        &["--home", home, "--json", "voucher", "seal"],
        &seal_input,
    ));
    assert_eq!(replay["number"], 1);
    assert_eq!(replay["hash"], first["hash"]);
    assert_eq!(replay["replayed"], true);

    let verification = json_stdout(run(
        config_home.path(),
        &["--home", home, "--json", "voucher", "verify"],
        "",
    ));
    assert_eq!(verification["ok"], true);
    assert_eq!(verification["voucher_count"], 1);
    assert_eq!(verification["head_number"], 1);
}

#[test]
fn help_is_read_only_and_short_and_long_text_forms_include_the_example() {
    let config_home = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let home = data_home.path().to_str().unwrap();

    for action in ["draft", "seal"] {
        for flag in ["-h", "--help"] {
            let output = success_stdout(run(
                config_home.path(),
                &["--home", home, "voucher", action, flag],
                "",
            ));
            assert!(output.contains("Example JSON (input-shape demonstration only"));
            serde_json::from_str::<Value>(&printed_json(&output)).unwrap();
        }
    }

    assert_eq!(std::fs::read_dir(data_home.path()).unwrap().count(), 0);
}

#[test]
fn printed_examples_do_not_default_required_evidence_fields() {
    let config_home = tempfile::tempdir().unwrap();
    let draft_help = help(config_home.path(), "draft", "--help");
    let seal_help = help(config_home.path(), "seal", "--help");
    let draft: Value = serde_json::from_str(&printed_json(&draft_help)).unwrap();
    let seal: Value = serde_json::from_str(&printed_json(&seal_help)).unwrap();

    let mut missing_document_date = draft.clone();
    missing_document_date
        .as_object_mut()
        .unwrap()
        .remove("document_date");
    assert_invalid_input(run(
        config_home.path(),
        &["--json", "voucher", "draft"],
        &missing_document_date.to_string(),
    ));

    let mut missing_money_amount = draft.clone();
    missing_money_amount["postings"][0]["amount"]
        .as_object_mut()
        .unwrap()
        .remove("amount");
    assert_invalid_input(run(
        config_home.path(),
        &["--json", "voucher", "draft"],
        &missing_money_amount.to_string(),
    ));

    let mut missing_money_currency = draft;
    missing_money_currency["postings"][0]["amount"]
        .as_object_mut()
        .unwrap()
        .remove("currency");
    assert_invalid_input(run(
        config_home.path(),
        &["--json", "voucher", "draft"],
        &missing_money_currency.to_string(),
    ));

    for required in ["draft", "natural_key"] {
        let mut missing = seal.clone();
        missing.as_object_mut().unwrap().remove(required);
        assert_invalid_input(run(
            config_home.path(),
            &["--json", "voucher", "seal"],
            &missing.to_string(),
        ));
    }
}
