//! `aggountant explain` and the `hint` field, read from the bundled user
//! guide's error-code table.

use std::process::{Command, Stdio};

use serde_json::Value;

fn run(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(args)
        .env_remove("AGGOUNTANT_HOME")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

/// The "Mitä tehdään" cell of `code`'s row in the Finnish guide's error-code
/// table, read at test time so rewording the guide does not break the test.
fn guide_what_to_do(code: &str) -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/user/fi/reference/error-codes.md"
    );
    let guide = std::fs::read_to_string(path).unwrap();
    let prefix = format!("| `{code}` |");
    let row = guide
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no row for {code} in {path}"));
    let cells: Vec<&str> = row.trim_matches('|').split(" | ").collect();
    assert_eq!(cells.len(), 3, "{row}");
    cells[2].trim().to_string()
}

#[test]
fn error_json_carries_the_guides_hint() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let home = home.to_str().unwrap();
    assert_eq!(run(&["--home", home, "init"]).0, 0);
    let (code, _, stderr) = run(&["--home", home, "init"]);
    assert_eq!(code, 1);
    let error: Value = serde_json::from_str(stderr.trim()).unwrap();
    assert_eq!(error["error_code"], "ALREADY_INITIALIZED");
    assert_eq!(error["hint"], guide_what_to_do("ALREADY_INITIALIZED"));
    let (_, stdout, _) = run(&["explain", "ALREADY_INITIALIZED", "--json"]);
    let entry: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(entry["what_to_do"], error["hint"]);
}

#[test]
fn error_without_a_guide_entry_has_no_hint() {
    let (code, _, stderr) = run(&["explain", "lower_case"]);
    assert_eq!(code, 1);
    let error: Value = serde_json::from_str(stderr.trim()).unwrap();
    assert_eq!(error["error_code"], "INVALID_INPUT");
    assert_eq!(error["hint"], "agentti korjaa syötteen");

    let (code, _, stderr) = run(&["explain", "CONFIG_WRITE_ERROR"]);
    assert_eq!(code, 1);
    let error: Value = serde_json::from_str(stderr.trim()).unwrap();
    assert_eq!(error["error_code"], "ERROR_CODE_UNKNOWN");
    assert!(error.get("hint").is_none());
}

#[test]
fn explain_prints_the_entry_in_either_language() {
    let (code, stdout, _) = run(&["explain", "PERIOD_SEALED", "--lang", "en", "--json"]);
    assert_eq!(code, 0);
    let entry: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(entry["code"], "PERIOD_SEALED");
    assert_eq!(entry["lang"], "en");
    assert_eq!(entry["what_to_do"], "record it in the next open period");
    assert_eq!(
        entry["guide_url"],
        "https://aggountant.dev/en/reference/error-codes/"
    );

    let (code, stdout, _) = run(&["explain", "PERIOD_SEALED"]);
    assert_eq!(code, 0);
    assert!(
        stdout.starts_with("PERIOD_SEALED\nMitä tapahtui: "),
        "{stdout}"
    );
    assert!(stdout.contains("Mitä tehdään: kirjaa seuraavalle avoimelle kaudelle"));
}

#[test]
fn explain_without_a_code_lists_the_codes() {
    let (code, stdout, _) = run(&["explain", "--json"]);
    assert_eq!(code, 0);
    let list: Value = serde_json::from_str(&stdout).unwrap();
    let codes = list["codes"].as_array().unwrap();
    assert!(codes.len() >= 30);
    assert!(codes.iter().any(|c| c["code"] == "VAT_FILING_UNRESOLVED"));
}
