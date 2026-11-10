use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use aggountant_ledger::Ledger;
use serde_json::{json, Value};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
}

fn run(home: &Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut command = bin();
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

fn success(home: &Path, args: &[&str], stdin: &str) -> Value {
    let (code, stdout, stderr) = run(home, args, stdin);
    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stderr.is_empty());
    serde_json::from_str(&stdout).unwrap()
}

fn seal(home: &Path, key: &str, document: &str, posting: &str, attachment: Option<&str>) -> Value {
    let mut draft = json!({
        "document_date": document,
        "posting_date": posting,
        "description": "Fictional office service",
        "postings": [
            {"account":"7690","amount":{"amount":"19.00","currency":"EUR"},"side":"debit"},
            {"account":"1910","amount":{"amount":"19.00","currency":"EUR"},"side":"credit"}
        ]
    });
    if let Some(reference) = attachment {
        draft["attachment_ref"] = json!(reference);
    }
    let input = json!({
        "draft": draft,
        "natural_key": key,
        "actor": "fictional-cli-test"
    });
    success(home, &["voucher", "seal"], &input.to_string())
}

fn tree_bytes(root: &Path) -> Option<BTreeMap<String, Vec<u8>>> {
    if !root.exists() {
        return None;
    }
    fn visit(base: &Path, path: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries = std::fs::read_dir(path)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                visit(base, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(base).unwrap().display().to_string(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    Some(files)
}

fn sqlite_sidecars(home: &Path) -> Vec<String> {
    fn visit(base: &Path, path: &Path, sidecars: &mut Vec<String>) {
        for entry in std::fs::read_dir(path).unwrap().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                visit(base, &path, sidecars);
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".sqlite-shm") || name.ends_with(".sqlite-wal"))
            {
                sidecars.push(path.strip_prefix(base).unwrap().display().to_string());
            }
        }
    }
    let mut sidecars = Vec::new();
    visit(home, home, &mut sidecars);
    sidecars.sort();
    sidecars
}

fn error(home: &Path, args: &[&str]) -> Value {
    let (code, stdout, stderr) = run(home, args, "");
    assert_eq!(code, 1, "stderr: {stderr}");
    assert!(stdout.is_empty());
    serde_json::from_str(&stderr).unwrap_or_else(|_| json!({"parser_text": stderr}))
}

#[test]
fn show_and_list_return_exact_stable_views_without_mutating_domain_state() {
    let home = tempfile::tempdir().unwrap();
    success(home.path(), &["init"], "");
    let source = home.path().join("fictional-source.bin");
    std::fs::write(&source, b"fictional shared attachment").unwrap();
    let attached = success(
        home.path(),
        &["artifact", "attach", "--file", source.to_str().unwrap()],
        "",
    );
    let attachment = attached["artifact_ref"].as_str().unwrap();

    let first = seal(
        home.path(),
        "fictional:one",
        "2026-01-10",
        "2026-01-12",
        Some(attachment),
    );
    seal(
        home.path(),
        "fictional:two",
        "2026-02-10",
        "2026-02-12",
        None,
    );
    seal(
        home.path(),
        "fictional:three",
        "2026-03-10",
        "2026-03-12",
        Some(attachment),
    );

    let ledger = Ledger::open(home.path()).unwrap();
    let before = ledger.verify().unwrap();
    let rejection_before = ledger.rejection_count().unwrap();
    let masterdata_reference_before = tree_bytes(&home.path().join("masterdata/reference"));
    let artifacts_before = tree_bytes(&home.path().join("artifacts"));
    let employees_before = ledger.list_employees().unwrap();
    let runs_before = tree_bytes(&home.path().join("process/runs"));
    let obligations_before = tree_bytes(&home.path().join("process/obligations"));
    let knowledge_before = tree_bytes(&home.path().join("knowledge"));
    drop(ledger);
    // WAL/SHM files are SQLite's transient read machinery, not fiscal-domain
    // records. Observe them separately; the assertions below intentionally make
    // no byte-immutability claim about SQLite headers or sidecars.
    let sidecars_before = sqlite_sidecars(home.path());

    let shown = success(home.path(), &["voucher", "show", "1"], "");
    assert_eq!(shown["schema_version"], 1);
    assert_eq!(shown["number"], first["number"]);
    assert_eq!(shown["voucher_hash"], first["hash"]);
    assert_eq!(shown["body"]["number"], first["number"]);
    assert_eq!(shown["body"]["document_date"], "2026-01-10");
    assert_eq!(shown["body"]["posting_date"], "2026-01-12");
    assert_eq!(shown["body"]["attachment_ref"], attachment);
    assert_eq!(shown["body"]["postings"].as_array().unwrap().len(), 2);
    assert_eq!(shown["provenance"]["natural_key"], "fictional:one");
    assert!(shown.get("duplicate").is_none());

    let by_attachment = success(
        home.path(),
        &["voucher", "list", "--attachment-ref", attachment],
        "",
    );
    assert_eq!(by_attachment["schema_version"], 1);
    assert_eq!(by_attachment["count"], 2);
    assert_eq!(by_attachment["truncated"], false);
    assert_eq!(by_attachment["order"], "number_asc");
    assert_eq!(by_attachment["vouchers"][0]["number"], 1);
    assert_eq!(by_attachment["vouchers"][1]["number"], 3);
    assert_eq!(
        by_attachment["vouchers"][0]["match_status"],
        "possible_source_match"
    );
    assert!(by_attachment.get("duplicate").is_none());

    let limited = success(
        home.path(),
        &[
            "voucher",
            "list",
            "--attachment-ref",
            attachment,
            "--limit",
            "1",
        ],
        "",
    );
    assert_eq!(limited["count"], 1);
    assert_eq!(limited["vouchers"].as_array().unwrap().len(), 1);
    assert_eq!(limited["vouchers"][0]["number"], 1);
    assert_eq!(limited["order"], "number_asc");
    assert_eq!(limited["truncated"], true);

    let by_both = success(
        home.path(),
        &[
            "voucher",
            "list",
            "--attachment-ref",
            attachment,
            "--posting-from",
            "2026-03-01",
            "--posting-to",
            "2026-03-31",
        ],
        "",
    );
    assert_eq!(by_both["count"], 1);
    assert_eq!(by_both["vouchers"][0]["number"], 3);
    assert_eq!(by_both["vouchers"][0]["document_date"], "2026-03-10");
    assert_eq!(by_both["vouchers"][0]["posting_date"], "2026-03-12");

    let zero = success(
        home.path(),
        &[
            "voucher",
            "list",
            "--posting-from",
            "2027-01-01",
            "--posting-to",
            "2027-01-31",
        ],
        "",
    );
    assert_eq!(zero["count"], 0);
    assert_eq!(zero["vouchers"], json!([]));
    assert_eq!(zero["truncated"], false);

    let missing = error(home.path(), &["voucher", "show", "999"]);
    assert_eq!(missing["error_code"], "VOUCHER_NOT_FOUND");

    let ledger = Ledger::open(home.path()).unwrap();
    let after = ledger.verify().unwrap();
    assert_eq!(after.voucher_count, before.voucher_count);
    assert_eq!(after.head_number, before.head_number);
    assert_eq!(after.head_hash, before.head_hash);
    assert_eq!(ledger.rejection_count().unwrap(), rejection_before);
    assert_eq!(ledger.list_employees().unwrap(), employees_before);
    drop(ledger);
    assert_eq!(
        tree_bytes(&home.path().join("masterdata/reference")),
        masterdata_reference_before
    );
    assert_eq!(tree_bytes(&home.path().join("artifacts")), artifacts_before);
    assert_eq!(tree_bytes(&home.path().join("process/runs")), runs_before);
    assert_eq!(
        tree_bytes(&home.path().join("process/obligations")),
        obligations_before
    );
    assert_eq!(tree_bytes(&home.path().join("knowledge")), knowledge_before);
    let sidecars_after = sqlite_sidecars(home.path());
    let allowed_sqlite_sidecars = [
        "fiscal.sqlite-shm",
        "fiscal.sqlite-wal",
        "masterdata/pii.sqlite-shm",
        "masterdata/pii.sqlite-wal",
    ];
    assert!(sidecars_before
        .iter()
        .chain(sidecars_after.iter())
        .all(|name| allowed_sqlite_sidecars.contains(&name.as_str())));
}

#[test]
fn invalid_arguments_fail_before_a_missing_or_uninitialized_home_can_be_fabricated() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("missing-home");
    let uppercase_hash = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    let cases: &[&[&str]] = &[
        &["voucher", "show", "0"],
        &["voucher", "list"],
        &["voucher", "list", "--attachment-ref", "abcd"],
        &["voucher", "list", "--attachment-ref", uppercase_hash],
        &["voucher", "list", "--posting-from", "2026-01-01"],
        &[
            "voucher",
            "list",
            "--posting-from",
            "2026-1-1",
            "--posting-to",
            "2026-01-31",
        ],
        &[
            "voucher",
            "list",
            "--posting-from",
            "2026-02-01",
            "--posting-to",
            "2026-01-01",
        ],
        &[
            "voucher",
            "list",
            "--posting-from",
            "2026-01-01",
            "--posting-to",
            "2026-01-31",
            "--limit",
            "0",
        ],
        &[
            "voucher",
            "list",
            "--posting-from",
            "2026-01-01",
            "--posting-to",
            "2026-01-31",
            "--limit",
            "101",
        ],
    ];
    for args in cases {
        let result = error(&missing, args);
        assert!(
            result["error_code"] == "INVALID_VOUCHER_READ"
                || result["error_code"] == "INVALID_HASH"
                || result["error_code"] == "INVALID_DATE",
            "unexpected error for {args:?}: {result}"
        );
        assert!(
            !missing.exists(),
            "validation must not create the home for {args:?}"
        );
    }

    let (code, stdout, _stderr) = run(&missing, &["voucher", "show"], "");
    assert_eq!(code, 1, "missing positional is a caller usage error");
    assert!(stdout.is_empty());
    assert!(!missing.exists());

    let uninitialized = root.path().join("uninitialized");
    std::fs::create_dir(&uninitialized).unwrap();
    let before = std::fs::read_dir(&uninitialized).unwrap().count();
    let result = error(&uninitialized, &["voucher", "show", "1"]);
    assert_eq!(result["error_code"], "HOME_NOT_INITIALIZED");
    assert_eq!(std::fs::read_dir(&uninitialized).unwrap().count(), before);

    let result = error(
        &missing,
        &[
            "voucher",
            "list",
            "--posting-from",
            "2026-01-01",
            "--posting-to",
            "2026-01-31",
        ],
    );
    assert_eq!(result["error_code"], "HOME_NOT_INITIALIZED");
    assert!(!missing.exists());
}

#[test]
fn read_help_is_neutral_and_states_filter_and_integrity_limits() {
    let output = bin().args(["voucher", "list", "--help"]).output().unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("possible source match"));
    assert!(help.contains("AND semantics"));
    assert!(help.contains("1..=100"));
    assert!(!help.contains("duplicate: true"));

    let output = bin().args(["voucher", "show", "--help"]).output().unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("does not verify the full chain"));
    assert!(help.contains("./accounting"));
}
