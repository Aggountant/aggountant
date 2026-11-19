use std::path::Path;

use clap::{Args, Subcommand};
use serde::Deserialize;
use serde_json::{json, Value};

use aggountant_process::{
    AddCompanyKnowledge, CompanyKnowledgeSource, ProcessEngine, RetireCompanyKnowledge,
    UpdateCompanyKnowledge,
};

use crate::{read_stdin_json, CliError};

const ADD_HELP: &str = r#"EXAMPLE
  echo '{"id":"bank-fees-practice","statement":"Bank service fees are booked to 8490.","scope":"All bank accounts.","source":{"reference":"user decision 2026-09-12","date":"2026-09-12"}}' \
    | aggountant --json knowledge company add"#;

const UPDATE_HELP: &str = r#"FACT ID
  Give the id positionally, with --id, or as "id" in the stdin JSON. When the
  JSON id and the argument are both given they must be the same id.

EXAMPLE
  echo '{"statement":"Bank service fees are booked to 8490.","scope":"All bank accounts.","source":{"reference":"user decision 2026-10-01","date":"2026-10-01"},"reason":"Scope widened."}' \
    | aggountant --json knowledge company update bank-fees-practice"#;

const RETIRE_HELP: &str = r#"FACT ID
  Give the id positionally, with --id, or as "id" in the stdin JSON. When the
  JSON id and the argument are both given they must be the same id.

EXAMPLE
  echo '{"source":{"reference":"user decision 2026-11-01","date":"2026-11-01"},"reason":"Bank account closed."}' \
    | aggountant --json knowledge company retire bank-fees-practice"#;

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
    #[command(after_help = ADD_HELP)]
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
    /// Update a fact from stdin JSON: {[id], statement, scope, source:{reference,date}, reason}.
    #[command(after_help = UPDATE_HELP)]
    Update {
        #[command(flatten)]
        id: FactIdArg,
    },
    /// Retire a fact from stdin JSON: {[id], source:{reference,date}, reason}.
    #[command(after_help = RETIRE_HELP)]
    Retire {
        #[command(flatten)]
        id: FactIdArg,
    },
    /// Show the complete revision history for one fact.
    History {
        #[arg(long)]
        id: String,
    },
}

/// The fact id of `update` and `retire`, given positionally or with `--id`.
#[derive(Args, Debug)]
pub(crate) struct FactIdArg {
    /// Fact id (same as --id).
    #[arg(value_name = "ID", conflicts_with = "id_flag")]
    positional: Option<String>,
    /// Fact id (same as the positional ID).
    #[arg(long = "id", id = "id_flag", value_name = "ID")]
    flag: Option<String>,
}

/// stdin JSON of `update`: the engine's input plus an optional `id`. Parsed
/// directly (not through `serde_json::Value`) so duplicate keys stay refused.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateInput {
    id: Option<String>,
    statement: String,
    scope: String,
    source: CompanyKnowledgeSource,
    reason: String,
}

/// stdin JSON of `retire`: the engine's input plus an optional `id`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RetireInput {
    id: Option<String>,
    source: CompanyKnowledgeSource,
    reason: String,
}

/// Resolves the fact id from the argument and the stdin JSON `id`, which must
/// agree when both are given.
fn resolve_id(arg: &FactIdArg, json_id: Option<String>) -> Result<String, CliError> {
    match (arg.positional.as_ref().or(arg.flag.as_ref()), json_id) {
        (Some(arg_id), Some(json_id)) if *arg_id != json_id => Err(CliError::user(
            "FACT_ID_MISMATCH",
            format!(
                "fact id argument {arg_id:?} differs from stdin JSON id {json_id:?}; give one id"
            ),
        )),
        (Some(arg_id), _) => Ok(arg_id.clone()),
        (None, Some(json_id)) => Ok(json_id),
        (None, None) => Err(CliError::user(
            "FACT_ID_REQUIRED",
            "give the fact id positionally, with --id, or as \"id\" in the stdin JSON",
        )),
    }
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
                let input: UpdateInput = read_stdin_json()?;
                let id = resolve_id(id, input.id)?;
                let input = UpdateCompanyKnowledge {
                    statement: input.statement,
                    scope: input.scope,
                    source: input.source,
                    reason: input.reason,
                };
                Ok(json!({
                    "schema_version": 1,
                    "document": engine.update_company_knowledge(&id, &input)?,
                }))
            }
            CompanyKnowledgeCmd::Retire { id } => {
                let input: RetireInput = read_stdin_json()?;
                let id = resolve_id(id, input.id)?;
                let input = RetireCompanyKnowledge {
                    source: input.source,
                    reason: input.reason,
                };
                Ok(json!({
                    "schema_version": 1,
                    "document": engine.retire_company_knowledge(&id, &input)?,
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
