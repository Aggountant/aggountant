//! The JSON example in a command's INPUT section runs: the commands that work
//! offline in a fresh home are fed their own printed example. That the other
//! stdin commands parse theirs is checked by the unit tests in `input_help.rs`.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

fn run(config_home: &Path, args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .env("HOME", config_home)
        .env("APPDATA", config_home)
        .env("USERPROFILE", config_home)
        .env_remove("AGGOUNTANT_HOME")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("AGGOUNTANT_BROKER_SCRIPT")
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

fn ok_json(output: Output) -> Value {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

/// The one JSON example printed by `<command> --help`.
fn example(config_home: &Path, command: &str) -> String {
    let mut args: Vec<&str> = command.split(' ').collect();
    args.push("--help");
    let help = String::from_utf8(run(config_home, &args, "").stdout).unwrap();
    assert!(help.contains("INPUT (JSON on stdin"), "{command}: {help}");
    help.split_once("```json\n")
        .and_then(|(_, rest)| rest.split_once("\n```"))
        .unwrap_or_else(|| panic!("{command} prints a JSON example"))
        .0
        .to_string()
}

#[test]
fn offline_stdin_commands_accept_their_printed_example() {
    let config_home = tempfile::tempdir().unwrap();
    let data_home = tempfile::tempdir().unwrap();
    let config = config_home.path();
    let home = data_home.path().to_str().unwrap();
    let base = ["--home", home, "--json"];
    let call = |command: &str, extra: &[&str]| {
        let mut args: Vec<&str> = base.to_vec();
        args.extend(command.split(' '));
        args.extend_from_slice(extra);
        ok_json(run(config, &args, &example(config, command)))
    };

    ok_json(run(config, &["--home", home, "--json", "init"], ""));

    let preview = call("capture-receipt record", &["--dry-run"]);
    assert_eq!(preview["projection_valid"], true);
    assert_eq!(preview["dry_run"], true);

    call("knowledge company add", &[]);
    call("knowledge company update", &["bank-fees-practice"]);
    let retired = call("knowledge company retire", &["bank-fees-practice"]);
    assert_eq!(retired["document"]["revisions"][2]["status"], "retired");

    let configured = call("vero-fi configure", &["--dry-run"]);
    assert_eq!(configured["written"], false);

    assert_eq!(call("employee add", &[])["stored"], true);
    let report = call(
        "file-wage preview",
        &["--timestamp", "2026-10-25T12:00:00Z"],
    );
    assert_eq!(report["deadline"], "2026-10-30");

    let payroll = call("run-payroll start", &[]);
    assert!(payroll["run_id"].is_string());
}
