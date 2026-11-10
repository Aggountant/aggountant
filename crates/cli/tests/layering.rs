//! F16 — structural supervision as a test (so `cargo test` enforces it, not
//! only CI). Two rules from `candidate.md §2/§4` and `ADR-1 §Implications`:
//!
//! 1. **Layer dependency direction** points one way (Model → Domain/Wire →
//!    Ledger → CLI). A crate may depend only on its allowed lower layers.
//! 2. **The Store ports live only inside the Ledger crate.** No crate outside
//!    `aggountant-ledger` may name `FiscalStore`/`ArtifactStore`/`MasterDataRefStore`
//!    or the `store` module — the storage boundary is Ledger-internal.
//!
//! A violation is either an accidental architecture regression or a deliberate
//! change that (per the design) must be an ADR. `cargo-deny` + this test are the
//! "cargo-modules or equivalent" checks the S1 brief asks for.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    // crates/cli -> crates -> <root>
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// The `[dependencies]` section text of a crate manifest (excludes dev-deps).
fn dependencies_section(manifest: &str) -> String {
    let mut out = String::new();
    let mut in_deps = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
            continue;
        }
        if in_deps {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

const INTERNAL_CRATES: &[&str] = &[
    "aggountant-model",
    "aggountant-wire",
    "aggountant-domain",
    "aggountant-import",
    "aggountant-ledger",
    "aggountant-broker",
    "aggountant-process",
    "aggountant", // the CLI package
];

#[test]
fn layer_dependency_direction_is_respected() {
    // crate dir -> allowed internal (non-dev) dependencies
    let allowed: BTreeMap<&str, Vec<&str>> = BTreeMap::from([
        ("model", vec![]),
        ("wire", vec!["aggountant-model"]),
        ("domain", vec!["aggountant-model"]),
        (
            // Import is a deterministic format-decoder + lawful projection (F23/F24):
            // it reads a source file into `DraftInput`s using the Model + Domain
            // public APIs only, and never touches Ledger / the Store ports (the CLI
            // composes it with the Ledger record→seal path, above both).
            "import",
            vec!["aggountant-model", "aggountant-domain"],
        ),
        (
            "ledger",
            vec!["aggountant-model", "aggountant-wire", "aggountant-domain"],
        ),
        (
            // Broker is the edge substrate: it talks to model/domain/wire only, and
            // never touches Ledger or the Store ports (the edge and the fiscal write
            // port are kept separate — the `file-vat` Run composes them, above both).
            "broker",
            vec!["aggountant-model", "aggountant-domain", "aggountant-wire"],
        ),
        (
            // Process sits above Ledger + Broker: it drives the Ledger service for the
            // authoritative fiscal transitions and the Broker for edge submits, but
            // never touches the Store ports.
            "process",
            vec![
                "aggountant-model",
                "aggountant-domain",
                "aggountant-ledger",
                "aggountant-broker",
            ],
        ),
        (
            "cli",
            vec![
                "aggountant-model",
                "aggountant-domain",
                "aggountant-import",
                "aggountant-ledger",
                "aggountant-broker",
                "aggountant-process",
            ],
        ),
    ]);

    let root = workspace_root();
    for (dir, allow) in &allowed {
        let manifest_path = root.join("crates").join(dir).join("Cargo.toml");
        let manifest = fs::read_to_string(&manifest_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", manifest_path.display()));
        let deps = dependencies_section(&manifest);
        for internal in INTERNAL_CRATES {
            // A dependency is declared as e.g. `aggountant-model = { path = ... }`.
            let declares = deps
                .lines()
                .any(|l| l.trim_start().starts_with(&format!("{internal} ")));
            if declares {
                assert!(
                    allow.contains(internal),
                    "layer violation: crate `{dir}` depends on `{internal}`, which is not \
                     an allowed lower layer {allow:?}. If intentional, this needs an ADR."
                );
            }
        }
    }
}

#[test]
fn store_ports_are_referenced_only_inside_ledger() {
    let root = workspace_root();
    let needles = ["FiscalStore", "ArtifactStore", "MasterDataRefStore"];

    for dir in [
        "model", "wire", "domain", "import", "broker", "process", "cli",
    ] {
        let src = root.join("crates").join(dir).join("src");
        let mut offending = Vec::new();
        visit_rs_files(&src, &mut |path, contents| {
            for needle in &needles {
                if contents.contains(needle) {
                    offending.push(format!("{} contains `{needle}`", path.display()));
                }
            }
        });
        assert!(
            offending.is_empty(),
            "Store port leaked outside the Ledger crate (F16):\n{}",
            offending.join("\n")
        );
    }
}

fn visit_rs_files(dir: &Path, f: &mut impl FnMut(&Path, &str)) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_rs_files(&path, f);
        } else if path.extension().is_some_and(|e| e == "rs") {
            if let Ok(contents) = fs::read_to_string(&path) {
                f(&path, &contents);
            }
        }
    }
}
