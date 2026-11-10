use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

fn run(home: &std::path::Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aggountant"));
    command.arg("--home").arg(home).arg("--json").args(args);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
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

fn add_input() -> Value {
    json!({
        "id": "hosting-vat-treatment",
        "statement": "Hosting invoices are reverse-charge purchases.",
        "scope": "Example Hosting Ltd managed hosting invoices.",
        "source": {
            "reference": "user decision in conversation 2026-09-12",
            "date": "2026-09-12"
        }
    })
}

#[test]
fn company_knowledge_cli_covers_add_show_search_update_history_and_retire() {
    let home = tempfile::tempdir().unwrap();
    let (code, stdout, stderr) = run(
        home.path(),
        &["knowledge", "company", "add"],
        &add_input().to_string(),
    );
    assert_eq!(code, 0, "{stderr}");
    let added: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(added["document"]["revisions"][0]["status"], "active");

    let (code, stdout, stderr) = run(
        home.path(),
        &[
            "knowledge",
            "company",
            "show",
            "--id",
            "hosting-vat-treatment",
        ],
        "",
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["revision"]["source"]["date"],
        "2026-09-12"
    );

    let (_, stdout, _) = run(
        home.path(),
        &[
            "knowledge",
            "company",
            "search",
            "--query",
            "example hosting",
        ],
        "",
    );
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let update = json!({
        "statement": "Hosting invoices are Finnish domestic purchases.",
        "scope": "Invoices dated on or after 2026-10-01.",
        "source": {"reference": "supplier establishment notice", "date": "2026-10-01"},
        "reason": "The supplier began invoicing from Finland."
    });
    let (code, _, stderr) = run(
        home.path(),
        &[
            "knowledge",
            "company",
            "update",
            "--id",
            "hosting-vat-treatment",
        ],
        &update.to_string(),
    );
    assert_eq!(code, 0, "{stderr}");

    let (_, stdout, _) = run(
        home.path(),
        &[
            "knowledge",
            "company",
            "history",
            "--id",
            "hosting-vat-treatment",
        ],
        "",
    );
    let history: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        history["document"]["revisions"].as_array().unwrap().len(),
        2
    );
    assert_eq!(history["document"]["revisions"][0]["revision"], 1);

    let retire = json!({
        "source": {"reference": "contract termination", "date": "2026-12-31"},
        "reason": "The contract ended."
    });
    let (code, _, stderr) = run(
        home.path(),
        &[
            "knowledge",
            "company",
            "retire",
            "--id",
            "hosting-vat-treatment",
        ],
        &retire.to_string(),
    );
    assert_eq!(code, 0, "{stderr}");
    let (_, stdout, _) = run(home.path(), &["knowledge", "company", "list"], "");
    assert!(serde_json::from_str::<Value>(&stdout).unwrap()["items"]
        .as_array()
        .unwrap()
        .is_empty());
    let (_, stdout, _) = run(
        home.path(),
        &["knowledge", "company", "list", "--include-retired"],
        "",
    );
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["items"][0]["status"],
        "retired"
    );
}

#[test]
fn invalid_input_is_stderr_only_and_does_not_replace_existing_file() {
    let home = tempfile::tempdir().unwrap();
    assert_eq!(
        run(
            home.path(),
            &["knowledge", "company", "add"],
            &add_input().to_string()
        )
        .0,
        0
    );
    let path = home
        .path()
        .join("knowledge/company/hosting-vat-treatment.json");
    let before = std::fs::read(&path).unwrap();
    let malformed = json!({
        "statement": " padded ",
        "scope": "scope",
        "source": {"reference": "user", "date": "2026-09-13"},
        "reason": "change"
    });
    let (code, stdout, stderr) = run(
        home.path(),
        &[
            "knowledge",
            "company",
            "update",
            "--id",
            "hosting-vat-treatment",
        ],
        &malformed.to_string(),
    );
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    let error: Value = serde_json::from_str(&stderr).unwrap();
    assert_eq!(error["error_code"], "INVALID_KNOWLEDGE");
    assert_eq!(error["error_kind"], "user");
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn old_user_workflow_and_company_binding_commands_are_absent() {
    let home = tempfile::tempdir().unwrap();
    for command in ["user", "propose", "match"] {
        let (code, _, stderr) = run(home.path(), &["knowledge", command], "");
        assert_ne!(code, 0);
        assert!(stderr.contains("unrecognized subcommand"));
    }
    assert!(!home
        .path()
        .join("process/user-knowledge-company.json")
        .exists());
}

#[test]
fn consult_knowledge_list_and_show_remain_available() {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join("knowledge")).unwrap();
    std::fs::write(home.path().join("knowledge/advice.md"), "consult advice").unwrap();
    assert_eq!(
        run(
            home.path(),
            &["knowledge", "company", "add"],
            &add_input().to_string(),
        )
        .0,
        0
    );
    let (code, stdout, stderr) = run(home.path(), &["knowledge", "list"], "");
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["knowledge"],
        json!(["advice"])
    );
    let (code, stdout, stderr) = run(home.path(), &["knowledge", "show", "--slug", "advice"], "");
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["document"],
        "consult advice"
    );
    let (code, stdout, stderr) = run(home.path(), &["knowledge", "company", "list"], "");
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        serde_json::from_str::<Value>(&stdout).unwrap()["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
