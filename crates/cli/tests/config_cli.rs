use std::process::{Command, Stdio};

use serde_json::Value;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
}

fn isolated_command(config_home: &std::path::Path) -> Command {
    let mut command = bin();
    command
        .env("HOME", config_home)
        .env("APPDATA", config_home)
        .env("USERPROFILE", config_home)
        .env_remove("AGGOUNTANT_HOME")
        .env_remove("AGGOUNTANT_VERO_CREDENTIALS_FILE")
        .env_remove("XDG_CONFIG_HOME")
        .stdin(Stdio::null());
    command
}

fn run(config_home: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let output = isolated_command(config_home).args(args).output().unwrap();
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

fn write_home_config(config_path: &std::path::Path, home: &std::path::Path) {
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(
        config_path,
        serde_json::to_string(&serde_json::json!({ "home": home })).unwrap(),
    )
    .unwrap();
}

#[test]
fn config_path_is_inspectable_and_show_reports_per_key_provenance() {
    let config_home = tempfile::tempdir().unwrap();
    let (code, stdout, stderr) = run(config_home.path(), &["--json", "config", "path"]);
    assert_eq!(code, 0, "{stderr}");
    let path: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(path["schema_version"], 1);
    let config_path = std::path::PathBuf::from(path["config_path"].as_str().unwrap());
    let configured_home = config_home.path().join("from-config");
    write_home_config(&config_path, &configured_home);

    let (code, stdout, stderr) = run(config_home.path(), &["--json", "config", "show"]);
    assert_eq!(code, 0, "{stderr}");
    let shown: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(shown["config_path"], path["config_path"]);
    assert_eq!(
        shown["config"]["home"]["value"],
        configured_home.display().to_string()
    );
    assert_eq!(shown["config"]["home"]["source"], "file");
    assert!(shown["config"]["home"]["marker_path"].is_null());
}

#[test]
fn relative_vero_credential_override_is_rejected_by_show_and_status() {
    let config_home = tempfile::tempdir().unwrap();

    for args in [
        vec!["--json", "config", "show"],
        vec!["--json", "vero-fi", "status"],
    ] {
        let output = isolated_command(config_home.path())
            .env("AGGOUNTANT_VERO_CREDENTIALS_FILE", "relative.json")
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error_code"], "INVALID_CREDENTIAL_PATH");
        assert_eq!(
            error["message"],
            "platform-local Vero credential path must be absolute"
        );
    }
}

#[test]
fn nearest_marker_is_reported_as_the_discovered_home() {
    let config_home = tempfile::tempdir().unwrap();
    let project = tempfile::tempdir().unwrap();
    let marker = project.path().join(".aggountant");
    let nested = project.path().join("records").join("2026");
    std::fs::create_dir_all(&marker).unwrap();
    std::fs::create_dir_all(&nested).unwrap();

    let output = isolated_command(config_home.path())
        .args(["--json", "config", "show"])
        .current_dir(&nested)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let shown: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        shown["config"]["home"]["value"],
        std::fs::canonicalize(project.path())
            .unwrap()
            .display()
            .to_string()
    );
    assert_eq!(shown["config"]["home"]["source"], "discovered");
    assert_eq!(
        shown["config"]["home"]["marker_path"],
        std::fs::canonicalize(marker).unwrap().display().to_string()
    );
}

#[test]
fn home_flag_and_environment_override_config_file() {
    let config_home = tempfile::tempdir().unwrap();
    let (_, stdout, _) = run(config_home.path(), &["--json", "config", "path"]);
    let path: Value = serde_json::from_str(&stdout).unwrap();
    let config_path = std::path::PathBuf::from(path["config_path"].as_str().unwrap());
    write_home_config(&config_path, &config_home.path().join("from-config"));

    let env_home = config_home.path().join("from-env");
    let output = isolated_command(config_home.path())
        .args(["--json", "config", "show"])
        .env("AGGOUNTANT_HOME", &env_home)
        .output()
        .unwrap();
    assert!(output.status.success());
    let shown: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        shown["config"]["home"]["value"],
        env_home.display().to_string()
    );
    assert_eq!(shown["config"]["home"]["source"], "env");

    let flag_home = config_home.path().join("from-flag");
    let flag_home_arg = flag_home.display().to_string();
    let (code, stdout, stderr) = run(
        config_home.path(),
        &["--json", "--home", &flag_home_arg, "config", "show"],
    );
    assert_eq!(code, 0, "{stderr}");
    let shown: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        shown["config"]["home"]["value"],
        flag_home.display().to_string()
    );
    assert_eq!(shown["config"]["home"]["source"], "flag");
}
