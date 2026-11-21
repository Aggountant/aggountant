//! `close-year answer` never assumes an answer: approving `ApproveStatements`
//! seals the fiscal year irreversibly, so the approval cannot be given by
//! omitting `--answer`.

use std::process::{Command, Output, Stdio};

fn invoke(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(args)
        .env_remove("AGGOUNTANT_HOME")
        .stdin(Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn answer_without_an_explicit_answer_is_a_usage_error() {
    let home = tempfile::tempdir().unwrap();
    let home = home.path().to_str().unwrap();
    assert_eq!(invoke(&["--home", home, "init"]).status.code(), Some(0));

    let output = invoke(&[
        "--home",
        home,
        "close-year",
        "answer",
        "--run-id",
        "cy-does-not-exist",
        "--gate",
        "ApproveStatements",
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--answer"), "stderr: {stderr}");
    // Refused by the parser, before any run is looked up.
    assert!(!stderr.contains("RUN_NOT_FOUND"), "stderr: {stderr}");
}

#[test]
fn answer_help_names_the_gates_and_the_permanent_approvals() {
    let output = invoke(&["close-year", "answer", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let help = String::from_utf8_lossy(&output.stdout);
    for gate in [
        "ApproveAdjustments",
        "ApproveTaseErittelyt",
        "ApproveLiitetiedot",
        "ApproveStatements",
    ] {
        assert!(help.contains(gate), "help does not name {gate}:\n{help}");
    }
    assert!(help.contains("seals the fiscal"), "{help}");
    assert!(help.contains("no default"), "{help}");
    assert!(!help.contains("[default: approve]"), "{help}");
    for flag in ["--run-id <RUN_ID>", "--gate <GATE>", "--answer <ANSWER>"] {
        let line = help
            .lines()
            .find(|line| line.trim_start().starts_with(flag))
            .unwrap_or_else(|| panic!("no {flag} in help:\n{help}"));
        assert!(
            line.trim() != flag,
            "{flag} has no help text on its line:\n{help}"
        );
    }
}
