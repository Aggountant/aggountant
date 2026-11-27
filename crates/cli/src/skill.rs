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
    fn path_in_repo(&self) -> String {
        format!("skills/{}/{}", self.name, self.files[0].0)
    }
}

/// The file a relative link in `skill`'s files names, as `skill install` writes
/// it: one of the skill's own files (`SOUL.md`) or a required skill's
/// (`../aggountant-init/SOUL.md`). `None` when the install would not write it.
fn linked_file(
    skill: &'static BundledSkill,
    link: &str,
) -> Option<(&'static BundledSkill, &'static str, &'static str)> {
    let (owner, file) = match link.strip_prefix("../") {
        Some(rest) => rest.split_once('/')?,
        None => (skill.name, link),
    };
    let owner = with_requirements(skill.name)
        .ok()?
        .into_iter()
        .find(|candidate| candidate.name == owner)?;
    let (name, content) = owner.files.iter().find(|(name, _)| *name == file)?;
    Some((owner, name, content))
}

const SKILLS: &[BundledSkill] = &[
    BundledSkill {
        name: "capture-receipt",
        description: "Keep a month's books: receipts, the bank statement, special entries, reconciliation and the month close.",
        files: skill_files!("capture-receipt": "SKILL.md", "matkaennakot.md"),
        requires: &["aggountant-init"],
        schema_version: 1,
    },
    BundledSkill {
        name: "aggountant-init",
        description: "Prepare a company accounting home and its bookkeeping starting point.",
        files: skill_files!(
            "aggountant-init": "SKILL.md",
            "SOUL.md",
            "avaavat-saldot.md",
            "aineistopyynto.md"
        ),
        requires: &[],
        schema_version: 1,
    },
    BundledSkill {
        name: "vero-fi",
        description: "File the company's VAT return with the Tax Administration, directly or by hand, and settle an uncertain filing.",
        files: skill_files!("vero-fi": "SKILL.md", "kayttoonotto.md", "hairiot.md"),
        // `tulorekisteri` held the Incomes Register half of this skill until the
        // split: an install that names only `vero-fi` must still receive it.
        requires: &["aggountant-init", "e-services", "tulorekisteri"],
        schema_version: 1,
    },
    BundledSkill {
        name: "tulorekisteri",
        description: "Report tax-free travel allowances and the monthly employer's separate report to the Incomes Register.",
        files: skill_files!("tulorekisteri": "SKILL.md", "kayttoonotto.md", "kiinnikurominen.md"),
        // `vero-fi` describes the credential file both channels share.
        requires: &["aggountant-init", "e-services", "vero-fi"],
        schema_version: 1,
    },
    BundledSkill {
        name: "e-services",
        description: "Work the authorities' web services (OmaVero, the Incomes Register) in the user's browser.",
        files: skill_files!("e-services": "SKILL.md", "omavero.md", "tulorekisteri.md", "selain.md"),
        requires: &["aggountant-init"],
        schema_version: 1,
    },
    BundledSkill {
        name: "close-year",
        description: "Close a fiscal year: year-end entries, the four approvals and the year seal.",
        files: skill_files!("close-year": "SKILL.md"),
        requires: &["aggountant-init"],
        schema_version: 1,
    },
    BundledSkill {
        name: "migrate",
        description: "Move a company from another program domain by domain, each on its own cutover date: the books, VAT, the Incomes Register and expense claims.",
        files: skill_files!(
            "migrate": "SKILL.md",
            "kirjanpito.md",
            "alv.md",
            "tulorekisteri.md",
            "kululaskut.md",
            "briox.md",
            "holvi.md",
        ),
        // The VAT and Incomes Register domains hand over to the filing skills.
        requires: &["aggountant-init", "vero-fi", "tulorekisteri"],
        schema_version: 1,
    },
];

/// A skill that was renamed: `(old name, new name)`. Installing never deletes,
/// so an agent that had the old skill installed would keep reading its stale
/// text next to the new one; installing the new skill replaces the old
/// `SKILL.md` with a pointer instead, and the old name is refused by name.
const RENAMED: &[(&str, &str)] = &[("migrate-books", "migrate")];

/// What an old skill's `SKILL.md` becomes once its successor is installed.
fn renamed_pointer(old: &str, new: &str) -> String {
    format!(
        "---\n\
name: {old}\n\
description: >-\n  \
Vanha nimi skillille {new}. Avaa {new} tämän sijaan.\n\
---\n\
\n\
# {old} on nyt {new}\n\
\n\
Tämän skillin sisältö on skillissä [{new}](../{new}/SKILL.md). Avaa se. Tämä hakemisto\n\
on jäänne aiemmasta asennuksesta, sen muut tiedostot ovat vanhentuneita, ja sen voi\n\
poistaa.\n"
    )
}

fn find(name: &str) -> Result<&'static BundledSkill, CliError> {
    if let Some((old, new)) = RENAMED.iter().find(|(old, _)| *old == name) {
        return Err(CliError::user(
            "SKILL_RENAMED",
            format!(
                "skill {old:?} was renamed to {new:?}; use `aggountant skill install {new}` \
                 or `aggountant skill print {new}`"
            ),
        )
        .with_details(json!({ "renamed_from": old, "renamed_to": new })));
    }
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
            "files": skill.files.iter().map(|(file, _)| *file).collect::<Vec<_>>(),
            "requires": skill.requires,
        })).collect::<Vec<_>>(),
        "renamed": RENAMED.iter().map(|(old, new)| json!({
            "name": old,
            "renamed_to": new,
        })).collect::<Vec<_>>(),
    })
}

/// `file` is a path as the skill's own text links it: `SKILL.md` (the default),
/// another file of the skill, or `../<required skill>/<file>`.
pub(super) fn printable(name: &str, file: Option<&str>) -> Result<Value, CliError> {
    let skill = find(name)?;
    let file = file.unwrap_or(skill.files[0].0);
    let (owner, owner_file, content) = linked_file(skill, file).ok_or_else(|| {
        let available = with_requirements(skill.name)
            .into_iter()
            .flatten()
            .flat_map(|owner| {
                owner.files.iter().map(move |(listed, _)| {
                    if owner.name == skill.name {
                        (*listed).to_string()
                    } else {
                        format!("../{}/{listed}", owner.name)
                    }
                })
            })
            .collect::<Vec<_>>();
        CliError::user(
            "SKILL_FILE_NOT_FOUND",
            format!(
                "skill {:?} ships no file {file:?}; available: {}",
                skill.name,
                available.join(", ")
            ),
        )
    })?;
    Ok(json!({
        "schema_version": 1,
        "name": skill.name,
        "cli_version": env!("CARGO_PKG_VERSION"),
        "schema_version_skill": skill.schema_version,
        "file": file,
        "content": content,
        "path_in_repo": format!("skills/{}/{owner_file}", owner.name),
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

    let mut renamed = Vec::new();
    for target in target_roots {
        for (old, new) in RENAMED {
            let pointer = target.join(old).join("SKILL.md");
            // Only an earlier install of the old skill gets the pointer; a
            // target that never had it gets no new directory.
            if !selected.iter().any(|skill| skill.name == *new) || !pointer.is_file() {
                continue;
            }
            renamed.push(json!({
                "name": old,
                "renamed_to": new,
                "path": pointer.display().to_string(),
                "action": install_file(&pointer, &renamed_pointer(old, new))?,
            }));
        }
    }

    Ok(json!({
        "schema_version": 1,
        "installed": installed,
        "renamed": renamed,
        "count": installed.len(),
        "targets": target_roots.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
    }))
}

/// The text-output note on old skill directories that now point to their
/// new name, empty when there are none.
pub(super) fn renamed_line(value: &Value) -> String {
    value["renamed"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|entry| {
            format!(
                "; {} now points to {} ({})",
                entry["path"].as_str().unwrap_or("?"),
                entry["renamed_to"].as_str().unwrap_or("?"),
                entry["action"].as_str().unwrap_or("?")
            )
        })
        .collect()
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
    use super::{linked_file, with_requirements, SKILLS};
    use aggountant_model::Date;

    #[test]
    fn bundled_metadata_and_print_newline_invariant_are_pinned() {
        for skill in SKILLS {
            assert_eq!(skill.files[0].0, "SKILL.md");
            let content = skill.files[0].1;
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
            for (file, content) in skill.files {
                for target in content.split("](").skip(1) {
                    let target = target.split(')').next().unwrap();
                    if target.contains("://") || target.starts_with('#') {
                        continue;
                    }
                    let target = target.split('#').next().unwrap();
                    assert!(
                        linked_file(skill, target).is_some(),
                        "{}/{file}: link {target:?} is not installed with the skill",
                        skill.name
                    );
                }
            }
        }
    }

    fn is_command_word(token: &str) -> bool {
        token.starts_with(|c: char| c.is_ascii_lowercase())
            && token
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }

    /// The first word of `tokens` that names no command, walking down from
    /// `command`. The walk ends at a leaf command, and at anything that does
    /// not start a word (an option, a placeholder, punctuation): what follows
    /// a command path is arguments, which `--help` owns. A word under a
    /// command that has subcommands must be one of them, however it is
    /// spelled (`file_vat`).
    fn first_unknown_command(mut command: &clap::Command, tokens: &[&str]) -> Option<String> {
        let mut path = command.get_name().to_string();
        for token in tokens {
            if !command.has_subcommands() || !token.starts_with(|c: char| c.is_alphanumeric()) {
                break;
            }
            path = format!("{path} {token}");
            command = match command.find_subcommand(token) {
                Some(subcommand) => subcommand,
                None => return Some(path),
            };
        }
        None
    }

    /// Command paths in `text` that the binary does not have. A path is what
    /// follows the word `aggountant` in a code span or a fenced block, and
    /// also an inline code span that starts with two bare words, which is how
    /// the skills name a command in running text (`voucher seal`).
    fn unknown_command_paths(root: &clap::Command, text: &str) -> Vec<String> {
        let mut unknown = Vec::new();
        let mut check = |code: &str, inline: bool| {
            let tokens: Vec<&str> = code.split_whitespace().collect();
            let mut named = false;
            for (at, _) in tokens
                .iter()
                .enumerate()
                .filter(|(_, token)| **token == root.get_name())
            {
                named = true;
                let mut rest = &tokens[at + 1..];
                // Global options may precede the command; skip them and the
                // value of one that takes a value.
                while let Some(option) = rest.first().and_then(|token| token.strip_prefix("--")) {
                    let takes_value = root
                        .get_arguments()
                        .find(|arg| arg.get_long() == Some(option))
                        .is_some_and(|arg| arg.get_action().takes_values());
                    rest = &rest[(1 + usize::from(takes_value)).min(rest.len())..];
                }
                unknown.extend(first_unknown_command(root, rest));
            }
            if !named
                && inline
                && tokens.len() >= 2
                && tokens[..2].iter().all(|t| is_command_word(t))
            {
                unknown.extend(first_unknown_command(root, &tokens));
            }
        };
        // Fenced blocks alternate with prose; an inline span may wrap a line.
        for (index, block) in text.split("```").enumerate() {
            if index % 2 == 1 {
                // A continued shell line is one command; a comment is prose.
                block
                    .replace("\\\n", " ")
                    .lines()
                    .filter(|line| !line.trim_start().starts_with('#'))
                    .for_each(|line| check(line, false));
            } else {
                block
                    .split('`')
                    .skip(1)
                    .step_by(2)
                    .for_each(|span| check(span, true));
            }
        }
        unknown
    }

    /// Built, so that it has clap's own `help` subcommands.
    fn command_tree() -> clap::Command {
        let mut root = <crate::Cli as clap::CommandFactory>::command();
        root.build();
        root
    }

    /// A command a skill names must exist: an agent that is told to run a
    /// command the binary does not have is sent to a dead end.
    #[test]
    fn named_commands_exist_in_the_command_tree() {
        let root = command_tree();
        for skill in SKILLS {
            for (file, content) in skill.files {
                let unknown = unknown_command_paths(&root, content);
                assert!(
                    unknown.is_empty(),
                    "{}/{file} names commands the binary does not have: {unknown:?}. \
                     Write a command path from the top-level command down; an inline \
                     code span that starts with two bare words is read as one.",
                    skill.name
                );
            }
        }
    }

    #[test]
    fn made_up_and_partial_command_paths_are_reported() {
        let root = command_tree();
        let text = "\
Hylkäys näkyy komennolla `journal show`, ei komennolla `aggountant journal`.
Tila kysytään komennolla `aggountant --json --home /tmp/esimerkki file-vat tila --period 2026-08`.
Polku kirjoitetaan juuresta: `employer-report missing` ei riitä, eikä rivin yli `run
nayta` auta.

```sh
jq . receipt.json | aggountant --json capture-receipt tallenna \\
  --dry-run
aggountant --json --home /tmp/esimerkki \\
  file_vat status
aggountant init && aggountant Journal show
```
";
        assert_eq!(
            unknown_command_paths(&root, text),
            [
                "aggountant journal",
                "aggountant journal",
                "aggountant file-vat tila",
                "aggountant employer-report",
                "aggountant run nayta",
                "aggountant capture-receipt tallenna",
                "aggountant file_vat",
                "aggountant Journal",
            ]
        );

        let fine = "\
`aggountant`, `aggountant --help`, `aggountant file-vat --help`, `aggountant <komento>`,
`aggountant --json run show --run-id fv-…`, `file-wage employer-report missing`,
`voucher show <numero>`, `skill print aggountant-init --file SOUL.md`,
`status --month`, `user statement: conversation 2026-09-12`, `aggountant-init`.

```sh
# aggountant kertoo tilan
aggountant help file-vat
aggountant file-vat help status
```

```json
{\"type\": \"unsupported\", \"reason\": \"journal show\"}
```
";
        assert_eq!(unknown_command_paths(&root, fine), [] as [&str; 0]);
    }

    /// A company without expense claims must meet no claim questions while
    /// it migrates: everything about claims is in `kululaskut.md`, which only
    /// the domain table of `SKILL.md` opens, and no other file of the skill
    /// names a claim command.
    #[test]
    fn migrate_keeps_expense_claims_in_their_own_domain_file() {
        let skill = SKILLS.iter().find(|skill| skill.name == "migrate").unwrap();
        for (file, content) in skill.files {
            if *file == "kululaskut.md" {
                assert!(content.contains("`aggountant claimant add`"));
                continue;
            }
            for command in ["aggountant claim", "claimant "] {
                assert!(
                    !content.contains(command),
                    "migrate/{file} names {command:?}; claim material belongs in kululaskut.md"
                );
            }
            let links = content.matches("](kululaskut.md)").count();
            assert_eq!(
                links,
                usize::from(*file == "SKILL.md"),
                "migrate/{file}: only the domain table of SKILL.md opens kululaskut.md"
            );
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
            ["vero-fi", "aggountant-init", "e-services", "tulorekisteri"]
        );
        // The two filing skills link to each other; the cycle ends.
        assert_eq!(
            names("tulorekisteri"),
            ["tulorekisteri", "aggountant-init", "e-services", "vero-fi"]
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
