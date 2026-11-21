use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use super::CliError;

/// A skill's files, installed under `<target>/<name>/`. `SKILL.md` comes first:
/// it is what `skill print` shows. Every file of `skills/<name>/` that the skill
/// links to must be listed here, or the installed link dangles.
macro_rules! skill_files {
    ($name:literal: $($file:literal),+ $(,)?) => {
        &[$(($file, include_str!(concat!("../../../skills/", $name, "/", $file)))),+]
    };
}

struct BundledSkill {
    name: &'static str,
    description: &'static str,
    files: &'static [(&'static str, &'static str)],
    /// Skills this one links to, installed with it when it is selected by name.
    requires: &'static [&'static str],
    schema_version: u64,
}

impl BundledSkill {
    fn content(&self) -> &'static str {
        self.files[0].1
    }

    fn path_in_repo(&self) -> String {
        format!("skills/{}/{}", self.name, self.files[0].0)
    }
}

const SKILLS: &[BundledSkill] = &[
    BundledSkill {
        name: "capture-receipt",
        description: "Process supplied bookkeeping material and explain what remains open.",
        files: skill_files!("capture-receipt": "SKILL.md"),
        requires: &["aggountant-init"],
        schema_version: 1,
    },
    BundledSkill {
        name: "aggountant-init",
        description: "Prepare a company accounting home and its bookkeeping starting point.",
        files: skill_files!("aggountant-init": "SKILL.md", "SOUL.md"),
        requires: &[],
        schema_version: 1,
    },
    BundledSkill {
        name: "vero-fi",
        description: "Prepare, verify, and safely operate direct own-organisation Vero.fi filing.",
        files: skill_files!("vero-fi": "SKILL.md"),
        requires: &["aggountant-init", "e-services"],
        schema_version: 1,
    },
    BundledSkill {
        name: "e-services",
        description: "Work the authorities' web services (OmaVero, the Incomes Register) in the user's browser.",
        files: skill_files!("e-services": "SKILL.md", "omavero.md", "tulorekisteri.md", "selain.md"),
        requires: &["aggountant-init"],
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

/// The named skill followed by everything it requires, transitively, each once.
fn with_requirements(name: &str) -> Result<Vec<&'static BundledSkill>, CliError> {
    let mut selected = vec![find(name)?];
    let mut next = 0;
    while next < selected.len() {
        for required in selected[next].requires {
            let skill = find(required)?;
            if !selected.iter().any(|known| known.name == skill.name) {
                selected.push(skill);
            }
        }
        next += 1;
    }
    Ok(selected)
}

pub(super) fn list() -> Value {
    json!({
        "schema_version": 1,
        "skills": SKILLS.iter().map(|skill| json!({
            "name": skill.name,
            "description": skill.description,
            "cli_version": env!("CARGO_PKG_VERSION"),
            "schema_version_skill": skill.schema_version,
            "path_in_repo": skill.path_in_repo(),
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
        "content": skill.content(),
        "path_in_repo": skill.path_in_repo(),
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
        Some(name) => with_requirements(name)?,
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
            let destination = directory.join(skill.files[0].0);
            let mut file_results = Vec::new();
            let mut action = "unchanged";
            for (file, content) in skill.files {
                let path = directory.join(file);
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
    use super::{with_requirements, SKILLS};
    use aggountant_model::Date;

    #[test]
    fn bundled_metadata_and_print_newline_invariant_are_pinned() {
        for skill in SKILLS {
            assert_eq!(skill.files[0].0, "SKILL.md");
            let content = skill.content();
            assert!(content.starts_with("---\n"));
            assert!(content.contains(&format!("\nname: {}\n", skill.name)));
            assert!(content.contains(&format!(
                "\ncli_version: \"{}\"\n",
                env!("CARGO_PKG_VERSION")
            )));
            assert!(content.contains(&format!("\nschema_version: {}\n", skill.schema_version)));
            for (file, content) in skill.files {
                assert!(
                    !file.contains('/') && !file.contains('\\') && !file.starts_with('.'),
                    "{}: {file} must be a plain file name inside the skill directory",
                    skill.name
                );
                assert!(content.ends_with('\n'), "{}/{file}", skill.name);
                assert!(!content.ends_with("\n\n"), "{}/{file}", skill.name);
            }
        }
    }

    /// A relative link in an installed file must resolve to a file the same
    /// install writes: one of the skill's own files or a required skill's.
    #[test]
    fn relative_links_resolve_within_what_the_install_ships() {
        for skill in SKILLS {
            let shipped = with_requirements(skill.name).unwrap();
            for (file, content) in skill.files {
                for target in content.split("](").skip(1) {
                    let target = target.split(')').next().unwrap();
                    if target.contains("://") || target.starts_with('#') {
                        continue;
                    }
                    let target = target.split('#').next().unwrap();
                    let (owner, name) = match target.strip_prefix("../") {
                        Some(rest) => rest.split_once('/').unwrap_or_else(|| {
                            panic!("{}/{file}: link {target:?} names no skill", skill.name)
                        }),
                        None => (skill.name, target),
                    };
                    assert!(
                        shipped.iter().any(|candidate| candidate.name == owner
                            && candidate.files.iter().any(|(listed, _)| *listed == name)),
                        "{}/{file}: link {target:?} is not installed with the skill",
                        skill.name
                    );
                }
            }
        }
    }

    #[test]
    fn requirements_are_transitive_and_listed_once() {
        let names = |name| {
            with_requirements(name)
                .unwrap()
                .iter()
                .map(|skill| skill.name)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names("vero-fi"),
            ["vero-fi", "aggountant-init", "e-services"]
        );
        assert_eq!(
            names("capture-receipt"),
            ["capture-receipt", "aggountant-init"]
        );
        assert_eq!(names("aggountant-init"), ["aggountant-init"]);
    }

    /// Every e-services entry carries the header lines from its SKILL.md
    /// ("Merkinnän lukeminen"). Dates are checked for form and order only,
    /// never against today: an old entry is a lead, not a broken build.
    #[test]
    fn e_services_entries_have_well_formed_headers() {
        let skill = SKILLS
            .iter()
            .find(|skill| skill.name == "e-services")
            .unwrap();
        let mut ids = Vec::new();
        for (file, content) in &skill.files[1..] {
            let prefix = format!("{}-", file.trim_end_matches(".md"));
            let mut entries = 0;
            let mut lines = content.lines().peekable();
            while let Some(line) = lines.next() {
                let Some(heading) = line.strip_prefix("## ") else {
                    continue;
                };
                entries += 1;
                let (id, title) = heading
                    .split_once(" — ")
                    .unwrap_or_else(|| panic!("{file}: {heading:?} lacks \"<id> — <title>\""));
                assert!(
                    id.starts_with(&prefix)
                        && id.len() > prefix.len()
                        && id
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                    "{file}: id {id:?} must be lowercase ASCII starting with {prefix:?}"
                );
                assert!(!title.trim().is_empty(), "{file}: {id} has no title");
                assert!(!ids.contains(&id), "{file}: duplicate id {id}");
                ids.push(id);

                assert_eq!(
                    lines.next(),
                    Some(""),
                    "{file}: {id}: blank line after heading"
                );
                let mut header = |label: &str| {
                    let line = lines.next().unwrap_or_default();
                    let value = line
                        .strip_prefix(&format!("- **{label}:** "))
                        .unwrap_or_else(|| panic!("{file}: {id}: expected {label}, got {line:?}"));
                    assert!(!value.trim().is_empty(), "{file}: {id}: empty {label}");
                    value
                };
                header("Palvelu ja sivu");
                let source = header("Lähde");
                assert!(
                    source.starts_with("havaittu") || source.starts_with("dokumentoitu"),
                    "{file}: {id}: Lähde must be havaittu, dokumentoitu or both"
                );
                if source
                    .replace("ei dokumentoitu", "")
                    .contains("dokumentoitu")
                {
                    assert!(
                        source.contains("](https://"),
                        "{file}: {id}: documented without a link"
                    );
                }
                let dates = header("Havaittu");
                let (seen, verified) = dates
                    .split_once(" · **Vahvistettu:** ")
                    .unwrap_or_else(|| panic!("{file}: {id}: dates {dates:?}"));
                let seen = Date::parse(seen).unwrap_or_else(|e| panic!("{file}: {id}: {e}"));
                let verified =
                    Date::parse(verified).unwrap_or_else(|e| panic!("{file}: {id}: {e}"));
                assert!(
                    seen <= verified,
                    "{file}: {id}: Vahvistettu precedes Havaittu"
                );
                header("Rajapinta");
                assert_eq!(
                    lines.next(),
                    Some(""),
                    "{file}: {id}: blank line after headers"
                );
                assert!(
                    lines
                        .peek()
                        .is_some_and(|line| !line.trim().is_empty() && !line.starts_with('#')),
                    "{file}: {id}: entry has no prose"
                );
            }
            assert!(entries > 0, "{file} has no entries");
        }
    }
}
