//! Canon §15/§16 companion-skill discovery, installation, and pure printing.

use std::process::{Command, Output};

fn invoke(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(args)
        .output()
        .expect("run aggountant")
}

#[test]
fn list_exposes_bundled_skill_metadata_in_text_and_json() {
    let text = invoke(&["skill", "list"]);
    assert_eq!(text.status.code(), Some(0));
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.starts_with("capture-receipt — "));

    let output = invoke(&["--json", "skill", "list"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["schema_version"], 1);
    assert_eq!(payload["skills"][0]["name"], "capture-receipt");
    assert_eq!(payload["skills"].as_array().unwrap().len(), 3);
    assert_eq!(payload["skills"][1]["name"], "aggountant-init");
    assert_eq!(payload["skills"][2]["name"], "vero-fi");
    assert_eq!(
        payload["skills"][0]["cli_version"],
        env!("CARGO_PKG_VERSION")
    );
    assert_eq!(payload["skills"][0]["schema_version_skill"], 1);
}

#[test]
fn print_is_byte_identical_to_the_bundled_file_and_json_carries_metadata() {
    let expected = include_bytes!("../../../skills/capture-receipt/SKILL.md");
    let output = invoke(&["skill", "print", "capture-receipt"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, expected);

    let output = invoke(&["skill", "print", "capture-receipt", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["schema_version"], 1);
    assert_eq!(payload["name"], "capture-receipt");
    assert_eq!(payload["cli_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(payload["schema_version_skill"], 1);
    assert_eq!(payload["path_in_repo"], "skills/capture-receipt/SKILL.md");
    assert_eq!(payload["content"].as_str().unwrap().as_bytes(), expected);
}

#[test]
fn install_supports_an_explicit_runtime_root_and_all_by_default() {
    let target = tempfile::tempdir().unwrap();
    let target_arg = target.path().to_str().unwrap();
    let output = invoke(&["--json", "skill", "install", "--target", target_arg]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["count"], 3);
    assert_eq!(payload["installed"][0]["action"], "created");

    let installed = target.path().join("capture-receipt").join("SKILL.md");
    assert_eq!(
        std::fs::read(installed).unwrap(),
        include_bytes!("../../../skills/capture-receipt/SKILL.md")
    );
    assert_eq!(
        std::fs::read(target.path().join("vero-fi/SKILL.md")).unwrap(),
        include_bytes!("../../../skills/vero-fi/SKILL.md")
    );
}

#[test]
fn install_defaults_to_claude_skill_home() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(["skill", "install", "capture-receipt", "--json"])
        .env("HOME", home.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(home
        .path()
        .join(".claude/skills/capture-receipt/SKILL.md")
        .is_file());
}

#[test]
fn unknown_skill_is_actionable_even_without_home() {
    let output = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(["--json", "skill", "install", "not-a-skill"])
        .env_remove("HOME")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let payload: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(payload["error_code"], "SKILL_NOT_FOUND");
    assert_eq!(payload["error_kind"], "user");
}

#[test]
fn successful_print_has_no_side_effects_in_the_agent_home() {
    let home = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .args(["skill", "print", "capture-receipt"])
        .env("HOME", home.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert!(!home.path().join(".claude").exists());
}

#[test]
fn reinstall_reports_unchanged_without_rewriting_content() {
    let target = tempfile::tempdir().unwrap();
    let target_arg = target.path().to_str().unwrap();
    assert_eq!(
        invoke(&["skill", "install", "--target", target_arg])
            .status
            .code(),
        Some(0)
    );
    let output = invoke(&[
        "--json",
        "skill",
        "install",
        "capture-receipt",
        "--target",
        target_arg,
    ]);
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["installed"][0]["action"], "unchanged");
}

#[test]
fn selected_capture_installs_its_shared_role_and_repairs_it_on_reinstall() {
    let target = tempfile::tempdir().unwrap();
    let args = [
        "--json",
        "skill",
        "install",
        "capture-receipt",
        "--target",
        target.path().to_str().unwrap(),
    ];
    let output = invoke(&args);
    assert_eq!(output.status.code(), Some(0));
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["count"], 2);
    let capture = target.path().join("capture-receipt");
    let text = std::fs::read_to_string(capture.join("SKILL.md")).unwrap();
    let role_link = text
        .split_once("[SOUL.md](")
        .unwrap()
        .1
        .split_once(')')
        .unwrap()
        .0;
    let role = capture.join(role_link);
    assert_eq!(
        std::fs::read(&role).unwrap(),
        include_bytes!("../../../skills/aggountant-init/SOUL.md")
    );
    assert_eq!(
        std::fs::read(target.path().join("aggountant-init/SKILL.md")).unwrap(),
        include_bytes!("../../../skills/aggountant-init/SKILL.md")
    );

    std::fs::write(&role, "outdated role").unwrap();
    let repaired = invoke(&args);
    assert_eq!(repaired.status.code(), Some(0));
    let payload: serde_json::Value = serde_json::from_slice(&repaired.stdout).unwrap();
    assert_eq!(payload["installed"][1]["action"], "updated");
    assert_eq!(
        std::fs::read(&role).unwrap(),
        include_bytes!("../../../skills/aggountant-init/SOUL.md")
    );

    std::fs::remove_file(&role).unwrap();
    assert_eq!(invoke(&args).status.code(), Some(0));
    assert!(role.is_file());
    let output = invoke(&args);
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(payload["installed"]
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["action"] == "unchanged"));
}

#[test]
fn selected_vero_fi_installs_its_shared_role() {
    let target = tempfile::tempdir().unwrap();
    let output = invoke(&[
        "--json",
        "skill",
        "install",
        "vero-fi",
        "--target",
        target.path().to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["count"], 2);
    assert_eq!(
        std::fs::read(target.path().join("vero-fi/SKILL.md")).unwrap(),
        include_bytes!("../../../skills/vero-fi/SKILL.md")
    );
    assert_eq!(
        std::fs::read(target.path().join("aggountant-init/SOUL.md")).unwrap(),
        include_bytes!("../../../skills/aggountant-init/SOUL.md")
    );
}

#[test]
fn init_can_be_printed_and_installed_without_capture() {
    let output = invoke(&["skill", "print", "aggountant-init"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        output.stdout,
        include_bytes!("../../../skills/aggountant-init/SKILL.md")
    );
    let target = tempfile::tempdir().unwrap();
    let output = invoke(&[
        "--json",
        "skill",
        "install",
        "aggountant-init",
        "--target",
        target.path().to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(0));
    let payload: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["count"], 1);
    assert_eq!(
        payload["installed"][0]["files"].as_array().unwrap().len(),
        2
    );
    assert_eq!(
        std::fs::read(target.path().join("aggountant-init/SOUL.md")).unwrap(),
        include_bytes!("../../../skills/aggountant-init/SOUL.md")
    );
    assert!(!target.path().join("capture-receipt").exists());
}
