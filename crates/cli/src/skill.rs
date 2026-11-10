use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::CliError;

const INIT: &str = include_str!("../../../skills/aggountant-init/SKILL.md");
const SOUL: &str = include_str!("../../../skills/aggountant-init/SOUL.md");

const CAPTURE_RECEIPT: &str = include_str!("../../../skills/capture-receipt/SKILL.md");
const VERO_FI: &str = include_str!("../../../skills/vero-fi/SKILL.md");

struct BundledSkill {
    name: &'static str,
    description: &'static str,
    content: &'static str,
    path_in_repo: &'static str,
    schema_version: u64,
}

const SKILLS: &[BundledSkill] = &[
    BundledSkill {
        name: "capture-receipt",
        description: "Process supplied bookkeeping material and explain what remains open.",
        content: CAPTURE_RECEIPT,
        path_in_repo: "skills/capture-receipt/SKILL.md",
        schema_version: 1,
    },
    BundledSkill {
        name: "aggountant-init",
        description: "Prepare a company accounting home and its bookkeeping starting point.",
        content: INIT,
        path_in_repo: "skills/aggountant-init/SKILL.md",
        schema_version: 1,
    },
    BundledSkill {
        name: "vero-fi",
        description: "Prepare, verify, and safely operate direct own-organisation Vero.fi filing.",
        content: VERO_FI,
        path_in_repo: "skills/vero-fi/SKILL.md",
        schema_version: 1,
    },
];

fn find(name: &str) -> Result<&'static BundledSkill, CliError> {
    SKILLS
        .iter()
        .find(|skill| skill.name == name)
        .ok_or_else(|| {
            CliError::user(
                "SKILL_NOT_FOUND",
                format!(
                    "unknown skill {name:?}; available skills: {}",
                    SKILLS
                        .iter()
                        .map(|skill| skill.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        })
}

pub(super) fn list() -> Value {
    json!({
        "schema_version": 1,
        "skills": SKILLS.iter().map(|skill| json!({
            "name": skill.name,
            "description": skill.description,
            "cli_version": env!("CARGO_PKG_VERSION"),
            "schema_version_skill": skill.schema_version,
            "path_in_repo": skill.path_in_repo,
        })).collect::<Vec<_>>(),
    })
}

pub(super) fn printable(name: &str) -> Result<Value, CliError> {
    let skill = find(name)?;
    Ok(json!({
        "schema_version": 1,
        "name": skill.name,
        "cli_version": env!("CARGO_PKG_VERSION"),
        "schema_version_skill": skill.schema_version,
        "content": skill.content,
        "path_in_repo": skill.path_in_repo,
    }))
}

pub(super) fn ensure_known(name: Option<&str>) -> Result<(), CliError> {
    if let Some(name) = name {
        find(name)?;
    }
    Ok(())
}

pub(super) fn default_targets() -> Result<Vec<PathBuf>, CliError> {
    #[cfg(windows)]
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    #[cfg(not(windows))]
    let home = std::env::var_os("HOME");

    let home = home
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| {
            CliError::system(
                "SKILL_TARGET_UNRESOLVED",
                "the user home must resolve to an absolute, non-empty path when --target is omitted",
            )
        })?;
    Ok(vec![home.join(".claude").join("skills")])
}

/// Install into a list of roots even though the current CLI exposes one target.
/// The upcoming pi.dev dual-home support can add roots without changing copy semantics.
pub(super) fn install(name: Option<&str>, target_roots: &[PathBuf]) -> Result<Value, CliError> {
    let selected = match name {
        Some("capture-receipt") => vec![find("capture-receipt")?, find("aggountant-init")?],
        Some("vero-fi") => vec![find("vero-fi")?, find("aggountant-init")?],
        Some(name) => vec![find(name)?],
        None => SKILLS.iter().collect(),
    };

    for target in target_roots {
        validate_target(target)?;
    }

    let mut installed = Vec::new();
    for target in target_roots {
        for skill in &selected {
            let directory = target.join(skill.name);
            std::fs::create_dir_all(&directory).map_err(|error| {
                CliError::system(
                    "SKILL_INSTALL_IO_ERROR",
                    format!(
                        "could not create skill directory {}: {error}",
                        directory.display()
                    ),
                )
            })?;
            let destination = directory.join("SKILL.md");
            let mut files = vec![(destination.clone(), skill.content)];
            if skill.name == "aggountant-init" {
                files.push((directory.join("SOUL.md"), SOUL));
            }
            let mut file_results = Vec::new();
            let mut action = "unchanged";
            for (path, content) in files {
                let file_action = install_file(&path, content)?;
                if file_action == "created" || (file_action == "updated" && action == "unchanged") {
                    action = file_action;
                }
                file_results.push(json!({
                    "path": path.display().to_string(),
                    "action": file_action,
                }));
            }
            installed.push(json!({
                "name": skill.name,
                "path": destination.display().to_string(),
                "action": action,
                "files": file_results,
            }));
        }
    }

    Ok(json!({
        "schema_version": 1,
        "installed": installed,
        "count": installed.len(),
        "targets": target_roots.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
    }))
}

fn install_file(destination: &Path, content: &str) -> Result<&'static str, CliError> {
    let action = match std::fs::read(destination) {
        Ok(existing) if existing == content.as_bytes() => "unchanged",
        Ok(_) => "updated",
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => "created",
        Err(error) => {
            return Err(CliError::system(
                "SKILL_INSTALL_IO_ERROR",
                format!(
                    "could not inspect skill file {}: {error}",
                    destination.display()
                ),
            ));
        }
    };
    if action != "unchanged" {
        std::fs::write(destination, content.as_bytes()).map_err(|error| {
            CliError::system(
                "SKILL_INSTALL_IO_ERROR",
                format!(
                    "could not write skill file {}: {error}",
                    destination.display()
                ),
            )
        })?;
    }
    Ok(action)
}

fn validate_target(target: &Path) -> Result<(), CliError> {
    if target.as_os_str().to_string_lossy().trim().is_empty() {
        return Err(CliError::user(
            "INVALID_SKILL_TARGET",
            "--target must be a non-empty directory path",
        ));
    }
    if target.exists() && !target.is_dir() {
        return Err(CliError::user(
            "INVALID_SKILL_TARGET",
            format!("--target must be a directory, got {}", target.display()),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::SKILLS;

    #[test]
    fn bundled_metadata_and_print_newline_invariant_are_pinned() {
        for skill in SKILLS {
            assert!(skill.content.starts_with("---\n"));
            assert!(skill.content.contains(&format!("\nname: {}\n", skill.name)));
            assert!(skill.content.contains(&format!(
                "\ncli_version: \"{}\"\n",
                env!("CARGO_PKG_VERSION")
            )));
            assert!(skill
                .content
                .contains(&format!("\nschema_version: {}\n", skill.schema_version)));
            assert!(skill.content.ends_with('\n'));
            assert!(!skill.content.ends_with("\n\n"));
        }
    }
}
