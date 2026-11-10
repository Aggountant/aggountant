use std::path::Path;

use clap::Subcommand;
use serde_json::{json, Value};

use aggountant_process::{
    AddCompanyKnowledge, ProcessEngine, RetireCompanyKnowledge, UpdateCompanyKnowledge,
};

use crate::{read_stdin_json, CliError};

#[derive(Subcommand, Debug)]
pub(crate) enum KnowledgeCmd {
    /// List consult-generated knowledge documents.
    List,
    /// Show one consult-generated knowledge document (`--slug <slug>`).
    Show {
        #[arg(long)]
        slug: String,
    },
    /// Read and maintain facts and practices local to this accounting home.
    Company {
        #[command(subcommand)]
        cmd: CompanyKnowledgeCmd,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum CompanyKnowledgeCmd {
    /// Add a fact from stdin JSON: {id, statement, scope, source:{reference,date}}.
    Add,
    /// Show the latest or a numbered revision.
    Show {
        #[arg(long)]
        id: String,
        #[arg(long)]
        revision: Option<u32>,
    },
    /// List current facts (active by default).
    List {
        #[arg(long)]
        include_retired: bool,
    },
    /// Search IDs, statements, scopes and source references.
    Search {
        #[arg(long)]
        query: String,
        #[arg(long)]
        include_retired: bool,
    },
    /// Update a fact from stdin JSON: {statement, scope, source:{reference,date}, reason}.
    Update {
        #[arg(long)]
        id: String,
    },
    /// Retire a fact from stdin JSON: {source:{reference,date}, reason}.
    Retire {
        #[arg(long)]
        id: String,
    },
    /// Show the complete revision history for one fact.
    History {
        #[arg(long)]
        id: String,
    },
}

pub(crate) fn run(cmd: &KnowledgeCmd, home: &Path) -> Result<Value, CliError> {
    let engine = ProcessEngine::open(home);
    match cmd {
        KnowledgeCmd::List => Ok(json!({ "knowledge": engine.knowledge().list()? })),
        KnowledgeCmd::Show { slug } => {
            let document = engine.knowledge().get(slug)?;
            Ok(json!({ "slug": slug, "document": document }))
        }
        KnowledgeCmd::Company { cmd } => match cmd {
            CompanyKnowledgeCmd::Add => {
                let input: AddCompanyKnowledge = read_stdin_json()?;
                Ok(json!({
                    "schema_version": 1,
                    "document": engine.add_company_knowledge(&input)?,
                }))
            }
            CompanyKnowledgeCmd::Show { id, revision } => Ok(json!({
                "schema_version": 1,
                "id": id,
                "revision": engine.show_company_knowledge(id, *revision)?,
            })),
            CompanyKnowledgeCmd::List { include_retired } => Ok(json!({
                "schema_version": 1,
                "items": engine.list_company_knowledge(*include_retired)?,
            })),
            CompanyKnowledgeCmd::Search {
                query,
                include_retired,
            } => Ok(json!({
                "schema_version": 1,
                "query": query,
                "items": engine.search_company_knowledge(query, *include_retired)?,
            })),
            CompanyKnowledgeCmd::Update { id } => {
                let input: UpdateCompanyKnowledge = read_stdin_json()?;
                Ok(json!({
                    "schema_version": 1,
                    "document": engine.update_company_knowledge(id, &input)?,
                }))
            }
            CompanyKnowledgeCmd::Retire { id } => {
                let input: RetireCompanyKnowledge = read_stdin_json()?;
                Ok(json!({
                    "schema_version": 1,
                    "document": engine.retire_company_knowledge(id, &input)?,
                }))
            }
            CompanyKnowledgeCmd::History { id } => Ok(json!({
                "schema_version": 1,
                "document": engine.history_company_knowledge(id)?,
            })),
        },
    }
}

pub(crate) fn human_line(cmd: &KnowledgeCmd, value: &Value) -> String {
    match cmd {
        KnowledgeCmd::List => format!("knowledge: {}", value["knowledge"]),
        KnowledgeCmd::Show { slug } => format!("knowledge/{slug}"),
        KnowledgeCmd::Company { cmd } => match cmd {
            CompanyKnowledgeCmd::Add
            | CompanyKnowledgeCmd::Update { .. }
            | CompanyKnowledgeCmd::Retire { .. } => {
                let id = value["document"]["id"].as_str().unwrap_or("unknown");
                let latest = value["document"]["revisions"]
                    .as_array()
                    .and_then(|items| items.last());
                match latest {
                    Some(latest) => format!(
                        "company knowledge {id} — {} (revision {})",
                        latest["status"].as_str().unwrap_or("unknown"),
                        latest["revision"].as_u64().unwrap_or(0)
                    ),
                    None => format!("company knowledge {id}"),
                }
            }
            CompanyKnowledgeCmd::Show { .. } => format!(
                "company knowledge {} — {} (revision {})",
                value["id"].as_str().unwrap_or("unknown"),
                value["revision"]["status"].as_str().unwrap_or("unknown"),
                value["revision"]["revision"].as_u64().unwrap_or(0)
            ),
            CompanyKnowledgeCmd::List { .. } | CompanyKnowledgeCmd::Search { .. } => format!(
                "company knowledge: {} item(s)",
                value["items"].as_array().map(Vec::len).unwrap_or(0)
            ),
            CompanyKnowledgeCmd::History { .. } => format!(
                "company knowledge {}: {} revision(s)",
                value["document"]["id"].as_str().unwrap_or("unknown"),
                value["document"]["revisions"]
                    .as_array()
                    .map(Vec::len)
                    .unwrap_or(0)
            ),
        },
    }
}
