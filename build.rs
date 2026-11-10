use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FALLBACK_VERSION: &str = "unknown-no-git";

fn git_command(workspace: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .current_dir(workspace)
        // Builds launched by Git hooks can inherit repository/index selectors.
        // They must not redirect provenance reads away from this workspace.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR");
    command
}

fn git_run(workspace: &Path, args: &[&str]) -> Option<Output> {
    let output = git_command(workspace).args(args).output().ok()?;
    output.status.success().then_some(output)
}

fn git_output(workspace: &Path, args: &[&str]) -> Option<String> {
    let output = git_run(workspace, args)?;
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn absolute(workspace: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        workspace.join(path)
    }
}

fn is_workspace_repository(workspace: &Path) -> bool {
    // Git searches ancestors. Without this equality check, a source archive
    // unpacked below an unrelated checkout would receive that checkout's SHA.
    let Some(top) = git_output(workspace, &["rev-parse", "--show-toplevel"]) else {
        return false;
    };
    matches!(
        (std::fs::canonicalize(workspace), std::fs::canonicalize(top)),
        (Ok(expected), Ok(actual)) if expected == actual
    )
}

fn watch_git_state(workspace: &Path) {
    // Linked worktrees keep HEAD/index in their worktree git dir while refs and
    // packed-refs live in the common git dir. Cargo recursively fingerprints
    // watched directories, so these cover commit, checkout, stage, and ref moves.
    for args in [
        &["rev-parse", "--git-dir"][..],
        &["rev-parse", "--git-common-dir"][..],
    ] {
        if let Some(path) = git_output(workspace, args) {
            println!(
                "cargo::rerun-if-changed={}",
                absolute(workspace, &path).display()
            );
        }
    }

    // Unstaged tracked edits change no Git metadata. Watch each tracked path so
    // clean↔dirty transitions rerun this script. `-z` disables quotePath escaping
    // and handles spaces; missing paths are omitted after the deletion-triggered
    // rerun, avoiding a permanent Cargo stale-path loop. The repository requires
    // UTF-8 source paths; an invalid path is skipped rather than emitted falsely.
    if let Some(output) = git_run(workspace, &["ls-files", "-z"]) {
        for raw in output.stdout.split(|byte| *byte == 0) {
            if raw.is_empty() {
                continue;
            }
            let Ok(relative) = std::str::from_utf8(raw) else {
                continue;
            };
            let path = workspace.join(relative);
            if path.exists() {
                println!("cargo::rerun-if-changed={}", path.display());
            }
        }
    }
}

fn main() {
    // This shared workspace build script is selected by crates/ledger/Cargo.toml.
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest_dir
        .parent()
        .and_then(Path::parent)
        .unwrap_or(&manifest_dir);

    let version = if is_workspace_repository(workspace) {
        watch_git_state(workspace);
        // `--long` keeps the SHA at an exact tag; `--abbrev=40` makes the commit
        // identity independent of core.abbrev and repository size. `--dirty` is
        // mandatory: an uncommitted build must never look reproducible.
        git_output(
            workspace,
            &["describe", "--always", "--dirty", "--long", "--abbrev=40"],
        )
        .unwrap_or_else(|| FALLBACK_VERSION.to_owned())
    } else {
        // No Git metadata (normal for source archives), or Git found only an
        // ancestor repository: fail closed to a recognizable non-SHA value.
        FALLBACK_VERSION.to_owned()
    };

    println!("cargo::rustc-env=AGGOUNTANT_SOFTWARE_VERSION={version}");
}
