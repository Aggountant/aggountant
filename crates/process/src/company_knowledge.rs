//! Company-local bookkeeping facts and practices with understandable revision history.
//!
//! Each accounting home owns its knowledge by physical separation. Records live at
//! `<home>/knowledge/company/<id>.json`; there is no company registry or workflow Run.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use aggountant_model::Date;
use serde::{Deserialize, Serialize};

use crate::{ProcessEngine, ProcessError};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanyKnowledgeSource {
    /// A human-readable pointer to the evidence, conversation, decision, or source file.
    pub reference: String,
    /// Date of the source or confirmation, not the filesystem write time.
    pub date: Date,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddCompanyKnowledge {
    pub id: String,
    pub statement: String,
    /// Prose describing when this knowledge is relevant and its important limits.
    pub scope: String,
    pub source: CompanyKnowledgeSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateCompanyKnowledge {
    pub statement: String,
    pub scope: String,
    pub source: CompanyKnowledgeSource,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetireCompanyKnowledge {
    pub source: CompanyKnowledgeSource,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanyKnowledgeStatus {
    Active,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanyKnowledgeRevision {
    pub revision: u32,
    pub status: CompanyKnowledgeStatus,
    pub statement: String,
    pub scope: String,
    pub source: CompanyKnowledgeSource,
    /// Why an earlier understanding changed. Absent only on the first revision.
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanyKnowledgeDocument {
    pub schema_version: u32,
    pub id: String,
    pub revisions: Vec<CompanyKnowledgeRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompanyKnowledgeItem {
    pub id: String,
    pub status: CompanyKnowledgeStatus,
    pub revision: u32,
    pub statement: String,
    pub scope: String,
    pub source: CompanyKnowledgeSource,
}

impl ProcessEngine {
    pub fn add_company_knowledge(
        &self,
        input: &AddCompanyKnowledge,
    ) -> Result<CompanyKnowledgeDocument, ProcessError> {
        validate_id(&input.id)?;
        validate_content(&input.statement, &input.scope, &input.source)?;
        let path = self.company_knowledge_path(&input.id)?;
        if path.exists() {
            return invalid(format!("company knowledge {:?} already exists", input.id));
        }
        let document = CompanyKnowledgeDocument {
            schema_version: SCHEMA_VERSION,
            id: input.id.clone(),
            revisions: vec![CompanyKnowledgeRevision {
                revision: 1,
                status: CompanyKnowledgeStatus::Active,
                statement: input.statement.clone(),
                scope: input.scope.clone(),
                source: input.source.clone(),
                reason: None,
            }],
        };
        write_document(&path, &document)?;
        Ok(document)
    }

    pub fn update_company_knowledge(
        &self,
        id: &str,
        input: &UpdateCompanyKnowledge,
    ) -> Result<CompanyKnowledgeDocument, ProcessError> {
        validate_content(&input.statement, &input.scope, &input.source)?;
        validate_text("reason", &input.reason, 2_000)?;
        let path = self.company_knowledge_path(id)?;
        let mut document = read_document(&path, id)?;
        let current = document.revisions.last().expect("validated history");
        document.revisions.push(CompanyKnowledgeRevision {
            revision: current.revision + 1,
            status: CompanyKnowledgeStatus::Active,
            statement: input.statement.clone(),
            scope: input.scope.clone(),
            source: input.source.clone(),
            reason: Some(input.reason.clone()),
        });
        write_document(&path, &document)?;
        Ok(document)
    }

    pub fn retire_company_knowledge(
        &self,
        id: &str,
        input: &RetireCompanyKnowledge,
    ) -> Result<CompanyKnowledgeDocument, ProcessError> {
        validate_source(&input.source)?;
        validate_text("reason", &input.reason, 2_000)?;
        let path = self.company_knowledge_path(id)?;
        let mut document = read_document(&path, id)?;
        let current = document.revisions.last().expect("validated history");
        if current.status == CompanyKnowledgeStatus::Retired {
            return invalid(format!("company knowledge {id:?} is already retired"));
        }
        document.revisions.push(CompanyKnowledgeRevision {
            revision: current.revision + 1,
            status: CompanyKnowledgeStatus::Retired,
            statement: current.statement.clone(),
            scope: current.scope.clone(),
            source: input.source.clone(),
            reason: Some(input.reason.clone()),
        });
        write_document(&path, &document)?;
        Ok(document)
    }

    pub fn show_company_knowledge(
        &self,
        id: &str,
        revision: Option<u32>,
    ) -> Result<CompanyKnowledgeRevision, ProcessError> {
        let document = read_document(&self.company_knowledge_path(id)?, id)?;
        revision
            .map_or_else(
                || document.revisions.last(),
                |wanted| {
                    document
                        .revisions
                        .iter()
                        .find(|item| item.revision == wanted)
                },
            )
            .cloned()
            .ok_or_else(|| {
                ProcessError::KnowledgeNotFound(format!(
                    "no company knowledge {id:?} revision {revision:?}"
                ))
            })
    }

    pub fn history_company_knowledge(
        &self,
        id: &str,
    ) -> Result<CompanyKnowledgeDocument, ProcessError> {
        read_document(&self.company_knowledge_path(id)?, id)
    }

    pub fn list_company_knowledge(
        &self,
        include_retired: bool,
    ) -> Result<Vec<CompanyKnowledgeItem>, ProcessError> {
        let root = self.company_knowledge_root();
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut items = Vec::new();
        for entry in fs::read_dir(&root).map_err(store)? {
            let entry = entry.map_err(store)?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                // Includes an interrupted writer's `<id>.json.tmp` and editor metadata.
                // Only `.json` files are committed records.
                continue;
            }
            if !path.is_file() {
                return Err(ProcessError::KnowledgeHistoryCorrupt(format!(
                    "company knowledge entry is not a file: {}",
                    path.display()
                )));
            }
            let id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| {
                    ProcessError::KnowledgeHistoryCorrupt(format!(
                        "invalid company knowledge filename: {}",
                        path.display()
                    ))
                })?;
            let document = read_document(&path, id)?;
            let current = document.revisions.last().expect("validated history");
            if include_retired || current.status == CompanyKnowledgeStatus::Active {
                items.push(item(&document.id, current));
            }
        }
        items.sort_by(|left, right| left.id.cmp(&right.id));
        Ok(items)
    }

    pub fn search_company_knowledge(
        &self,
        query: &str,
        include_retired: bool,
    ) -> Result<Vec<CompanyKnowledgeItem>, ProcessError> {
        validate_text("query", query, 1_000)?;
        let query = query.to_lowercase();
        Ok(self
            .list_company_knowledge(include_retired)?
            .into_iter()
            .filter(|entry| {
                [
                    &entry.id,
                    &entry.statement,
                    &entry.scope,
                    &entry.source.reference,
                ]
                .iter()
                .any(|value| value.to_lowercase().contains(&query))
            })
            .collect())
    }

    fn company_knowledge_root(&self) -> PathBuf {
        self.home.join("knowledge").join("company")
    }

    fn company_knowledge_path(&self, id: &str) -> Result<PathBuf, ProcessError> {
        validate_id(id)?;
        Ok(self.company_knowledge_root().join(format!("{id}.json")))
    }
}

fn item(id: &str, revision: &CompanyKnowledgeRevision) -> CompanyKnowledgeItem {
    CompanyKnowledgeItem {
        id: id.to_string(),
        status: revision.status,
        revision: revision.revision,
        statement: revision.statement.clone(),
        scope: revision.scope.clone(),
        source: revision.source.clone(),
    }
}

fn read_document(path: &Path, expected_id: &str) -> Result<CompanyKnowledgeDocument, ProcessError> {
    let bytes = fs::read(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ProcessError::KnowledgeNotFound(format!("no company knowledge {expected_id:?}"))
        } else {
            store(error)
        }
    })?;
    let document: CompanyKnowledgeDocument = serde_json::from_slice(&bytes).map_err(|error| {
        ProcessError::KnowledgeHistoryCorrupt(format!("{}: {error}", path.display()))
    })?;
    validate_document(&document, expected_id, path)?;
    Ok(document)
}

fn validate_document(
    document: &CompanyKnowledgeDocument,
    expected_id: &str,
    path: &Path,
) -> Result<(), ProcessError> {
    let corrupt = |message: String| {
        ProcessError::KnowledgeHistoryCorrupt(format!("{}: {message}", path.display()))
    };
    if document.schema_version != SCHEMA_VERSION {
        return Err(corrupt(format!(
            "unsupported schema_version {}",
            document.schema_version
        )));
    }
    validate_id(&document.id).map_err(|error| corrupt(error.to_string()))?;
    if document.id != expected_id {
        return Err(corrupt(format!(
            "document id {:?} does not match filename {expected_id:?}",
            document.id
        )));
    }
    if document.revisions.is_empty() {
        return Err(corrupt("revision history is empty".to_string()));
    }
    for (index, revision) in document.revisions.iter().enumerate() {
        if revision.revision != index as u32 + 1 {
            return Err(corrupt("revision numbers are not contiguous".to_string()));
        }
        validate_content(&revision.statement, &revision.scope, &revision.source)
            .map_err(|error| corrupt(error.to_string()))?;
        if index == 0 {
            if revision.status != CompanyKnowledgeStatus::Active || revision.reason.is_some() {
                return Err(corrupt(
                    "first revision must be active and have no reason".to_string(),
                ));
            }
        } else {
            validate_text("reason", revision.reason.as_deref().unwrap_or(""), 2_000)
                .map_err(|error| corrupt(error.to_string()))?;
        }
    }
    Ok(())
}

fn write_document(path: &Path, document: &CompanyKnowledgeDocument) -> Result<(), ProcessError> {
    let bytes = serde_json::to_vec_pretty(document).map_err(store)?;
    let parent = path
        .parent()
        .ok_or_else(|| ProcessError::Store(format!("path {} has no parent", path.display())))?;
    fs::create_dir_all(parent).map_err(store)?;
    let tmp = path.with_extension("json.tmp");
    let result = (|| {
        let mut file = fs::File::create(&tmp).map_err(store)?;
        file.write_all(&bytes).map_err(store)?;
        file.sync_all().map_err(store)?;
        fs::rename(&tmp, path).map_err(store)?;
        // Rename is the commit point. Directory sync is best-effort so the command
        // never reports failure after the visible document has already changed.
        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}

fn validate_content(
    statement: &str,
    scope: &str,
    source: &CompanyKnowledgeSource,
) -> Result<(), ProcessError> {
    validate_text("statement", statement, 8_000)?;
    validate_text("scope", scope, 4_000)?;
    validate_source(source)
}

fn validate_source(source: &CompanyKnowledgeSource) -> Result<(), ProcessError> {
    validate_text("source.reference", &source.reference, 2_000)
}

fn validate_id(id: &str) -> Result<(), ProcessError> {
    let valid = !id.is_empty()
        && id.len() <= 128
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(())
    } else {
        invalid(format!(
            "id {id:?} must be lowercase kebab-case without edge or repeated hyphens"
        ))
    }
}

fn validate_text(field: &str, value: &str, max: usize) -> Result<(), ProcessError> {
    if value.is_empty() || value.trim() != value || value.len() > max {
        invalid(format!(
            "{field} must be nonempty, unpadded UTF-8 text of at most {max} bytes"
        ))
    } else {
        Ok(())
    }
}

fn invalid<T>(message: impl Into<String>) -> Result<T, ProcessError> {
    Err(ProcessError::InvalidKnowledge(message.into()))
}

fn store(error: impl std::fmt::Display) -> ProcessError {
    ProcessError::Store(error.to_string())
}
