//! The **`knowledge` namespace** (S5). A git-native, read-mostly, versioned free-text
//! store (`candidate.md §2`): a once-reviewed understanding ("Apple monthly = reverse-
//! charge") is written here and reused as a suggestion basis thereafter. **New knowledge
//! enters only through the `ApproveKnowledgePR` review gate** — never an auto-cache — and
//! **PII never enters knowledge** (the write path takes a `KnowledgeCandidate`, which is
//! PII-free by construction).
//!
//! Maturity scope for S5: a free-text markdown document per slug under
//! `<home>/knowledge/<slug>.md`, no semantic search (the `candidate.md §2` maturity path
//! starts here). Git provides the versioning.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::ProcessError;

/// The git-native `knowledge` store (`<home>/knowledge`).
pub struct Knowledge {
    root: PathBuf,
}

impl Knowledge {
    pub fn open(home: &Path) -> Self {
        Knowledge {
            root: home.join("knowledge"),
        }
    }

    fn path(&self, slug: &str) -> Result<PathBuf, ProcessError> {
        // A slug is a filename component, not a path — reject traversal / separators so a
        // review-approved candidate can never write outside the namespace.
        let ok = !slug.is_empty()
            && slug.len() <= 128
            && slug
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !ok {
            return Err(ProcessError::InvalidAnswer(format!(
                "knowledge slug {slug:?} must be lowercase kebab-case [a-z0-9-]"
            )));
        }
        Ok(self.root.join(format!("{slug}.md")))
    }

    /// Commit a reviewed knowledge document (the `ApproveKnowledgePR` write). Overwrites
    /// the slug's current version; git keeps the history.
    pub fn put(&self, slug: &str, content: &str) -> Result<(), ProcessError> {
        let path = self.path(slug)?;
        fs::create_dir_all(&self.root).map_err(|e| ProcessError::Store(e.to_string()))?;
        let tmp = path.with_extension("md.tmp");
        {
            let mut f = fs::File::create(&tmp).map_err(|e| ProcessError::Store(e.to_string()))?;
            f.write_all(content.as_bytes())
                .map_err(|e| ProcessError::Store(e.to_string()))?;
            f.sync_all()
                .map_err(|e| ProcessError::Store(e.to_string()))?;
        }
        fs::rename(&tmp, &path).map_err(|e| ProcessError::Store(e.to_string()))?;
        Ok(())
    }

    /// Read a knowledge document, if present.
    pub fn get(&self, slug: &str) -> Result<Option<String>, ProcessError> {
        let path = self.path(slug)?;
        match fs::read_to_string(&path) {
            Ok(s) => Ok(Some(s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ProcessError::Store(e.to_string())),
        }
    }

    /// List known knowledge slugs (id-sorted).
    pub fn list(&self) -> Result<Vec<String>, ProcessError> {
        if !self.root.exists() {
            return Ok(Vec::new());
        }
        let mut slugs: Vec<String> = fs::read_dir(&self.root)
            .map_err(|e| ProcessError::Store(e.to_string()))?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter_map(|n| n.strip_suffix(".md").map(str::to_string))
            .collect();
        slugs.sort();
        Ok(slugs)
    }
}
