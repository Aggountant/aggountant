//! Only `init` creates a company home (issue `read-commands-create-home`).
//!
//! Every other command that opens a home refuses, writes nothing, and names the
//! path it looked at and how that path was chosen, when the resolved path holds
//! no initialized home: the `./accounting` fallback in an empty directory, the
//! same fallback run from inside a home, a mistyped `--home`, and a bare
//! `.aggountant` marker.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

/// A command with the platform configuration isolated to `config_home`, no
/// `$AGGOUNTANT_HOME`, run in `cwd`.
fn isolated(cwd: &Path, config_home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aggountant"));
    command
        .current_dir(cwd)
        .env("HOME", config_home)
        .env("APPDATA", config_home)
        .env("USERPROFILE", config_home)
        .env_remove("AGGOUNTANT_HOME")
        .env_remove("AGGOUNTANT_VERO_CREDENTIALS_FILE")
        .env_remove("AGGOUNTANT_BROKER_SCRIPT")
        .env_remove("XDG_CONFIG_HOME")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn run(command: &mut Command, args: &[&str], stdin: &str) -> (i32, String, Value) {
    let mut child = command
        .arg("--json")
        .args(args)
        .stdin(Stdio::piped())
        .spawn()
        .expect("spawn aggountant");
    common::feed_stdin(&mut child, stdin.as_bytes());
    let output = child.wait_with_output().unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    let error = serde_json::from_str(stderr.trim()).unwrap_or(Value::Null);
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).unwrap(),
        error,
    )
}

/// Every path below `root`, relative and sorted.
fn tree(root: &Path) -> Vec<PathBuf> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            out.push(path.strip_prefix(root).unwrap().to_path_buf());
            if path.is_dir() {
                walk(root, &path, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// Commands that read a home, as the issue lists them and beyond.
const READ_COMMANDS: &[&[&str]] = &[
    &["employee", "list"],
    &["employee", "show", "--employee", "e-1"],
    &["run", "list"],
    &["run", "show", "--run-id", "cp-2026-01"],
    &["run", "preview", "close-period", "--period", "2026-01"],
    &["period", "show", "--period", "2026-01"],
    &["voucher", "verify"],
    &["voucher", "show", "1"],
    &[
        "voucher",
        "list",
        "--posting-from",
        "2026-01-01",
        "--posting-to",
        "2026-01-31",
    ],
    &["file-vat", "preview", "--period", "2026-01"],
    &["file-vat", "status", "--period", "2026-01"],
    &[
        "report",
        "statements",
        "--period",
        "2026-01",
        "--company",
        "Esimerkki Oy",
    ],
    &["chart", "list"],
    &["fiscal-year", "list"],
    &["fiscal-year", "show", "--year", "2026"],
    &["obligation", "list"],
    &["intake", "list"],
    &["knowledge", "list"],
    &["knowledge", "company", "list"],
    &["home", "upgrade", "--dry-run"],
    &["vero-fi", "status"],
    &["tulorekisteri", "status"],
    &["file-wage", "employer-report", "missing"],
    &[
        "import",
        "camt053",
        "observations",
        "--attachment-ref",
        "0000000000000000000000000000000000000000000000000000000000000000",
    ],
];

/// Commands that write into a home. They must not create one either.
const WRITE_COMMANDS: &[(&[&str], &str)] = &[
    (&["employee", "add"], r#"{"employee_id":"e-1"}"#),
    (&["voucher", "seal"], "{}"),
    (
        &[
            "artifact",
            "attach",
            "--file",
            concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"),
        ],
        "",
    ),
    (
        &[
            "fiscal-year",
            "define",
            "--start",
            "2026-01-01",
            "--end",
            "2026-12-31",
            "--actor",
            "test",
        ],
        "",
    ),
    (&["run", "start", "close-period", "--period", "2026-01"], ""),
    (&["home", "upgrade", "--apply"], ""),
    (
        &["knowledge", "company", "add"],
        r#"{"id":"x","statement":"s","scope":"s","source":{"reference":"r","date":"2026-01-01"}}"#,
    ),
    (&["vero-fi", "configure"], r#"{"mode":"disabled"}"#),
    (&["tulorekisteri", "configure"], r#"{"schema_version":1}"#),
];

fn assert_refused(args: &[&str], code: i32, stdout: &str, error: &Value) {
    assert_eq!(code, 1, "{args:?}: {error}");
    assert!(stdout.is_empty(), "{args:?}: {stdout}");
    assert_eq!(
        error["error_code"], "HOME_NOT_INITIALIZED",
        "{args:?}: {error}"
    );
    assert_eq!(error["error_kind"], "user");
}

#[test]
fn commands_in_an_empty_directory_leave_it_empty_and_name_the_fallback() {
    let config = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let expected = std::path::absolute(cwd.path().join("accounting")).unwrap();

    for args in READ_COMMANDS {
        let (code, stdout, error) = run(&mut isolated(cwd.path(), config.path()), args, "");
        assert_refused(args, code, &stdout, &error);
        assert_eq!(error["details"]["source"], "default", "{args:?}");
        assert_eq!(
            error["details"]["home"],
            expected.display().to_string(),
            "{args:?}"
        );
        let message = error["message"].as_str().unwrap();
        assert!(
            message.contains(&expected.display().to_string()) && message.contains("./accounting"),
            "{args:?}: {message}"
        );
        assert!(
            tree(cwd.path()).is_empty(),
            "{args:?} wrote {:?}",
            tree(cwd.path())
        );
    }

    for (args, stdin) in WRITE_COMMANDS {
        let (code, stdout, error) = run(&mut isolated(cwd.path(), config.path()), args, stdin);
        assert_ne!(code, 0, "{args:?} succeeded without a home: {error}");
        assert!(stdout.is_empty(), "{args:?}: {stdout}");
        assert!(
            tree(cwd.path()).is_empty(),
            "{args:?} wrote {:?}",
            tree(cwd.path())
        );
    }
}

#[test]
fn the_fallback_from_inside_a_home_does_not_create_a_nested_home() {
    let config = tempfile::tempdir().unwrap();
    let home = common::initialized_home();
    let before = tree(home.path());

    for args in [
        &["run", "show", "--run-id", "cp-2026-01"][..],
        &["run", "list"],
    ] {
        let (code, stdout, error) = run(&mut isolated(home.path(), config.path()), args, "");
        assert_refused(args, code, &stdout, &error);
        assert_eq!(error["details"]["source"], "default");
    }
    assert!(!home.path().join("accounting").exists());
    assert_eq!(tree(home.path()), before);
}

#[test]
fn a_mistyped_home_flag_is_named_and_not_created() {
    let config = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let missing = cwd.path().join("acounting");

    let mut command = isolated(cwd.path(), config.path());
    command.arg("--home").arg(&missing);
    let (code, stdout, error) = run(&mut command, &["employee", "list"], "");
    assert_refused(&["employee", "list"], code, &stdout, &error);
    assert_eq!(error["details"]["source"], "flag");
    assert_eq!(error["details"]["home"], missing.display().to_string());
    assert!(error["message"].as_str().unwrap().contains("--home"));
    assert!(tree(cwd.path()).is_empty());

    // An empty directory given as the home is refused the same way.
    std::fs::create_dir(&missing).unwrap();
    let mut command = isolated(cwd.path(), config.path());
    command.arg("--home").arg(&missing);
    let (code, stdout, error) = run(&mut command, &["voucher", "verify"], "");
    assert_refused(&["voucher", "verify"], code, &stdout, &error);
    assert!(tree(&missing).is_empty());

    // The environment variable is named as the source.
    let mut command = isolated(cwd.path(), config.path());
    command.env("AGGOUNTANT_HOME", &missing);
    let (_, _, error) = run(&mut command, &["period", "show", "--period", "2026-01"], "");
    assert_eq!(error["error_code"], "HOME_NOT_INITIALIZED");
    assert_eq!(error["details"]["source"], "env");
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains("$AGGOUNTANT_HOME"));
    assert!(tree(&missing).is_empty());
}

#[test]
fn a_bare_marker_is_named_and_left_alone() {
    let config = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let marker = project.path().join(".aggountant");
    std::fs::create_dir(&marker).unwrap();

    let (code, stdout, error) = run(
        &mut isolated(project.path(), config.path()),
        &["voucher", "verify"],
        "",
    );
    assert_refused(&["voucher", "verify"], code, &stdout, &error);
    assert_eq!(error["details"]["source"], "discovered");
    assert_eq!(
        error["details"]["marker_path"],
        marker.display().to_string()
    );
    assert_eq!(tree(project.path()), vec![PathBuf::from(".aggountant")]);
}

#[test]
fn a_cloned_home_without_its_voucher_database_is_not_opened_as_empty_books() {
    // A company repository tracks only masterdata/reference; opening a fresh
    // clone must not start new, empty books next to it.
    let config = tempfile::tempdir().unwrap();
    let home = common::initialized_home();
    std::fs::remove_file(home.path().join("fiscal.sqlite")).unwrap();
    std::fs::remove_dir_all(home.path().join("artifacts")).unwrap();
    std::fs::remove_file(home.path().join("masterdata").join("pii.sqlite")).unwrap();
    let before = tree(home.path());

    let mut command = isolated(home.path(), config.path());
    command.arg("--home").arg(home.path());
    let (code, stdout, error) = run(&mut command, &["voucher", "verify"], "");
    assert_refused(&["voucher", "verify"], code, &stdout, &error);
    assert_eq!(tree(home.path()), before);
}

#[test]
fn a_home_without_an_empty_artifacts_directory_still_opens() {
    // git does not keep an empty artifacts/; such a home is still a home.
    let config = tempfile::tempdir().unwrap();
    let home = common::initialized_home();
    std::fs::remove_dir_all(home.path().join("artifacts")).unwrap();

    let mut command = isolated(home.path(), config.path());
    command.arg("--home").arg(home.path());
    let (code, stdout, error) = run(&mut command, &["voucher", "verify"], "");
    assert_eq!(code, 0, "{error}");
    assert!(!stdout.is_empty());
}

#[test]
fn a_configured_home_names_the_configuration_file() {
    let config = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let (code, stdout, _) = run(
        &mut isolated(cwd.path(), config.path()),
        &["config", "path"],
        "",
    );
    assert_eq!(code, 0);
    let config_path: Value = serde_json::from_str(&stdout).unwrap();
    let config_path = PathBuf::from(config_path["config_path"].as_str().unwrap());
    let configured = cwd.path().join("books");
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(
        &config_path,
        serde_json::json!({ "home": configured }).to_string(),
    )
    .unwrap();

    let (code, stdout, error) = run(
        &mut isolated(cwd.path(), config.path()),
        &["employee", "list"],
        "",
    );
    assert_refused(&["employee", "list"], code, &stdout, &error);
    assert_eq!(error["details"]["source"], "file");
    assert!(error["message"]
        .as_str()
        .unwrap()
        .contains(&config_path.display().to_string()));
    assert!(!configured.exists());
}
