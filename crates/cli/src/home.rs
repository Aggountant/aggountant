use std::path::Path;

use clap::{ArgGroup, Subcommand};
use serde_json::{json, Value};

use aggountant_ledger::{Ledger, UpgradeState};

use crate::CliError;

const HOME_UPGRADE_HELP: &str = r#"Run after updating aggountant. A newer binary can need masterdata an older
`init` never wrote: bundled chart snapshots, VAT account role metadata and
settlement account roles (the bank and tax accounts VAT is paid through).

  aggountant --home ./accounting --json home upgrade --dry-run
  aggountant --home ./accounting --json home upgrade --apply

The upgrade creates missing files under masterdata/reference/. It never
changes the active chart or chart_version. The one file it replaces is
vat-account-roles.json, and only while it is still byte for byte what an
earlier aggountant release wrote and the bundled file keeps every role in it,
adding reverse-charge targets (for example `eu-goods`); that entry has
`"action": "update"` and is listed under `updated` after --apply. Any other
file whose bytes differ from the bundled one (for example a company's own
role metadata) is listed under `skipped` with reason `differs` and left alone.
Role metadata is added only when the bundled roles cover the home's own
active chart; otherwise it is skipped as `unavailable`. A second run is a no-op.

`masterdata_opens` tells whether the home's masterdata opens afterwards (for
--dry-run: once the planned files are added); `masterdata_error` says why not.
Commit the new files in masterdata/reference/ with git: that is the record of
what the upgrade added."#;

#[derive(Subcommand, Debug)]
pub(crate) enum HomeCmd {
    /// Take in the masterdata a newer aggountant needs: add missing bundled chart
    /// snapshots, VAT role metadata and settlement roles, and extend an unedited
    /// VAT role file from an earlier release. `--dry-run` writes nothing.
    #[command(
        after_help = HOME_UPGRADE_HELP,
        group(ArgGroup::new("mode").required(true).args(["dry_run", "apply"]))
    )]
    Upgrade {
        /// List what would be added without writing anything.
        #[arg(long)]
        dry_run: bool,
        /// Add the missing files and extend an unedited earlier role file.
        #[arg(long)]
        apply: bool,
    },
}

pub(crate) fn run(cmd: &HomeCmd, home: &Path) -> Result<Value, CliError> {
    match cmd {
        HomeCmd::Upgrade { apply, .. } => {
            let upgrade = Ledger::upgrade_masterdata(home, *apply)?;
            let mut planned = Vec::new();
            let mut existed = Vec::new();
            let mut skipped = Vec::new();
            for item in &upgrade.items {
                match item.state {
                    UpgradeState::Create
                    | UpgradeState::Created
                    | UpgradeState::Update
                    | UpgradeState::Updated => {
                        let action = match item.state {
                            UpgradeState::Create | UpgradeState::Created => "create",
                            _ => "update",
                        };
                        let mut would = json!({
                            "action": action,
                            "resource": item.resource,
                            "path": item.path,
                        });
                        if let Some(version) = &item.chart_version {
                            would["chart_version"] = json!(version);
                        }
                        if action == "update" {
                            if let Some(detail) = &item.detail {
                                would["detail"] = json!(detail);
                            }
                        }
                        planned.push(would);
                    }
                    UpgradeState::Present => existed.push(json!(item.path)),
                    UpgradeState::Differs | UpgradeState::Unavailable => skipped.push(json!({
                        "resource": item.resource,
                        "path": item.path,
                        "reason": item.state,
                        "detail": item.detail.as_deref().unwrap_or(
                            "existing file differs from the bundled one; left untouched"
                        ),
                    })),
                }
            }
            let mut value = json!({
                "schema_version": 1,
                "dry_run": !upgrade.applied,
                "home": home.display().to_string(),
                "masterdata_path": home.join("masterdata").join("reference").display().to_string(),
                "active_chart_version": upgrade.active_chart_version,
                "existed": existed,
                "skipped": skipped,
                "changed": upgrade.applied && !planned.is_empty(),
                "masterdata_opens": upgrade.masterdata_check.is_none(),
                "masterdata_error": upgrade.masterdata_check.as_ref().map(|e| json!({
                    "error_code": e.error_code(),
                    "message": e.to_string(),
                })),
            });
            if upgrade.applied {
                let paths = |action: &str| {
                    planned
                        .iter()
                        .filter(|p| p["action"] == action)
                        .map(|p| p["path"].clone())
                        .collect::<Vec<_>>()
                };
                value["created"] = json!(paths("create"));
                value["updated"] = json!(paths("update"));
            } else {
                value["would"] = json!(planned);
            }
            Ok(value)
        }
    }
}

pub(crate) fn human_line(cmd: &HomeCmd, value: &Value) -> String {
    match cmd {
        HomeCmd::Upgrade { .. } => {
            let count = |key: &str| value[key].as_array().map_or(0, Vec::len);
            let opens = match value["masterdata_error"]["error_code"].as_str() {
                None => "masterdata opens".to_string(),
                Some(code) => format!("masterdata still does not open ({code})"),
            };
            let dry_run = value["dry_run"] == true;
            let would = |action: &str| {
                value["would"].as_array().map_or(0, |items| {
                    items.iter().filter(|w| w["action"] == action).count()
                })
            };
            let (add, added, update, updated) = if dry_run {
                (
                    "would add",
                    would("create"),
                    "would update",
                    would("update"),
                )
            } else {
                ("added", count("created"), "updated", count("updated"))
            };
            let updates = if updated > 0 {
                format!(", {update} {updated}")
            } else {
                String::new()
            };
            format!(
                "{add} {added} file(s){updates}, {} already present, {} left untouched — {opens}",
                count("existed"),
                count("skipped"),
            )
        }
    }
}
