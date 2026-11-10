use super::*;
use aggountant_broker::tulorekisteri::{production_broker, read_wage_profile};
use aggountant_domain::WageReport;
use chrono::{DateTime, Utc};

#[derive(Subcommand, Debug)]
pub enum WageCmd {
    /// Read report JSON on stdin. Offline; returns lines, explicit period/ownership, deadline and unsigned canonical record hash. Example: file-wage preview --timestamp 2026-10-25T12:00:00Z < report.json
    Preview {
        #[arg(long)]
        timestamp: DateTime<Utc>,
    },
    /// Persist the offline proposal and open approval gate. Use the preview timestamp. Example: file-wage start --timestamp 2026-10-25T12:00:00Z < report.json
    Start {
        #[arg(long)]
        timestamp: DateTime<Utc>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Approve exactly the record hash and dispatch once. Example: file-wage approve wr-DIGEST --record-hash HASH
    Approve {
        run_id: String,
        #[arg(long)]
        record_hash: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Signed status query; never repeats SendWageReport. Example: file-wage reconcile wr-DIGEST
    Reconcile {
        run_id: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Explicit e-service fallback evidence; check for existing API delivery first. Example: file-wage complete-manual wr-DIGEST --ir-delivery-id GUID --ir-report-id GUID
    CompleteManual {
        run_id: String,
        #[arg(long)]
        ir_delivery_id: String,
        #[arg(long)]
        ir_report_id: String,
        #[arg(long)]
        dry_run: bool,
    },
}
pub fn execute(cmd: &WageCmd, home: &Path) -> Result<Value, CliError> {
    let engine = ProcessEngine::open(home);
    let value = match cmd {
        WageCmd::Preview { timestamp } => serde_json::to_value(
            engine.preview_wage(&read_stdin_json::<WageReport>()?, *timestamp)?,
        ),
        WageCmd::Start { timestamp, dry_run } => {
            let report: WageReport = read_stdin_json()?;
            if *dry_run {
                return Ok(
                    json!({"schema_version":1,"dry_run":true,"would":[{"action":"create","resource":"file-wage","preview":engine.preview_wage(&report,*timestamp)?}]}),
                );
            }
            serde_json::to_value(engine.start_wage(&report, *timestamp)?)
        }
        WageCmd::Approve {
            run_id,
            record_hash,
            dry_run,
        } => {
            refuse_dry_run(*dry_run)?;
            serde_json::to_value(engine.approve_wage(run_id, record_hash, &broker(home)?)?)
        }
        WageCmd::Reconcile { run_id, dry_run } => {
            refuse_dry_run(*dry_run)?;
            serde_json::to_value(engine.reconcile_wage(run_id, &broker(home)?, Utc::now())?)
        }
        WageCmd::CompleteManual {
            run_id,
            ir_delivery_id,
            ir_report_id,
            dry_run,
        } => {
            refuse_dry_run(*dry_run)?;
            serde_json::to_value(engine.complete_wage_manual(
                run_id,
                ir_delivery_id,
                ir_report_id,
            )?)
        }
    };
    value
        .map(|mut v| {
            v["schema_version"] = json!(1);
            v
        })
        .map_err(|e| CliError::system("INTERNAL_ERROR", e.to_string()))
}
fn refuse_dry_run(dry_run: bool) -> Result<(), CliError> {
    if dry_run {
        Err(CliError::user(
            "dry_run_unsupported",
            "use offline preview/start --dry-run; this state transition requires live evidence",
        ))
    } else {
        Ok(())
    }
}
fn broker(home: &Path) -> Result<Broker, CliError> {
    let profile =
        read_wage_profile(home).map_err(|s| CliError::user("TULOREKISTERI_NOT_READY", s))?;
    let config = read_config(&config_path()?)?;
    let path = if let Some(path) = &profile.credentials_file {
        path.clone()
    } else {
        resolve_credential_path(&config, None)?
            .value
            .ok_or_else(|| {
                CliError::user(
                    "MISSING_CREDENTIAL",
                    "configure Vero credential path or a separate Tulorekisteri credentials_file",
                )
            })?
    };
    if !path.is_absolute() {
        return Err(CliError::user(
            "INVALID_CREDENTIAL_PATH",
            "credential path must be absolute",
        ));
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| CliError::system("IO_ERROR", "credential path is unavailable"))?;
    let canonical_home = home
        .canonicalize()
        .map_err(|_| CliError::system("IO_ERROR", "accounting home is unavailable"))?;
    if canonical.starts_with(canonical_home) {
        return Err(CliError::user(
            "CREDENTIAL_IN_ACCOUNTING_HOME",
            "credential must remain outside the git-versioned accounting home",
        ));
    }
    let secret = std::fs::read_to_string(&path).map_err(|_| {
        CliError::system("IO_ERROR", "could not read the configured credential file")
    })?;
    production_broker(home, profile, secret, Utc::now())
        .map_err(|e| CliError::user("TULOREKISTERI_NOT_READY", e.to_string()))
}
