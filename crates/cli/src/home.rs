use std::path::Path;

use clap::{ArgGroup, Subcommand};
use serde_json::{json, Value};

use aggountant_ledger::{Ledger, UpgradeState};

use crate::CliError;

const HOME_UPGRADE_HELP: &str = r#"Run after updating aggountant. A newer binary can need masterdata an older
`init` never wrote: bundled chart snapshots and VAT account role metadata.

  aggountant --home ./accounting --json home upgrade --dry-run
  aggountant --home ./accounting --json home upgrade --apply

The upgrade only creates missing files under masterdata/reference/. It never
changes the active chart or chart_version and never rewrites an existing file:
a file whose bytes differ from the bundled one (for example a company's own
role metadata) is listed under `skipped` with reason `differs` and left alone.
Role metadata is added only when the bundled roles activate the home's own
active chart; otherwise it is skipped as `unavailable`. A second run is a no-op.

`masterdata_opens` tells whether the home's masterdata opens afterwards (for
--dry-run: once the planned files are added); `masterdata_error` says why not.
Commit the new files in masterdata/reference/ with git: that is the record of
what the upgrade added."#;

#[derive(Subcommand, Debug)]
pub(crate) enum HomeCmd {
    /// Take in the masterdata a newer aggountant needs: add only missing bundled
    /// chart snapshots and VAT role metadata. `--dry-run` writes nothing.
    #[command(
        after_help = HOME_UPGRADE_HELP,
        group(ArgGroup::new("mode").required(true).args(["dry_run", "apply"]))
    )]
    Upgrade {
        /// List what would be added without writing anything.
        #[arg(long)]
        dry_run: bool,
        /// Add the missing files.
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
                    UpgradeState::Create | UpgradeState::Created => {
                        let mut would = json!({
                            "action": "create",
                            "resource": item.resource,
                            "path": item.path,
                        });
                        if let Some(version) = &item.chart_version {
                            would["chart_version"] = json!(version);
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
                value["created"] = json!(planned
                    .iter()
                    .map(|p| p["path"].clone())
                    .collect::<Vec<_>>());
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
            let (verb, added) = if value["dry_run"] == true {
                ("would add", count("would"))
            } else {
                ("added", count("created"))
            };
            format!(
                "{verb} {added} file(s), {} already present, {} left untouched — {opens}",
                count("existed"),
                count("skipped"),
            )
        }
    }
}
