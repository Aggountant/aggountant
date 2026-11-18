use super::*;
use aggountant_broker::tulorekisteri::{production_broker, read_wage_profile};
use aggountant_broker::ytj::HttpYtj;
use aggountant_domain::{
    payroll::{MonthPayments, ReportMonth},
    WageReport,
};
use aggountant_process::{EmployerReportInput, EmployerReportReceipt};
use chrono::{DateTime, NaiveDate, Utc};

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
    /// Monthly employer's separate report (työnantajan erillisilmoitus): preview, approve-and-submit once, settle an unknown outcome, list missing months, check the employer-register start in YTJ, import transcript 630 of months reported elsewhere.
    #[command(subcommand)]
    EmployerReport(EmployerReportCmd),
}

/// What was paid in the month. A month with wages is refused: its contribution is not computed yet.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum Payments {
    /// Nothing at all: reported as "No wages payable".
    None,
    /// Only tax-free reimbursements (kilometre or daily allowances): EUR 0 health insurance contribution.
    TaxFreeReimbursementsOnly,
    /// Wages or other pay; refused with EMPLOYER_REPORT_WAGES_UNSUPPORTED.
    Wages,
}

#[derive(clap::Args, Debug)]
pub struct EmployerReportArgs {
    /// Reporting month, YYYY-MM.
    #[arg(long, value_parser = parse_month)]
    month: ReportMonth,
    #[arg(long, value_enum)]
    payments: Payments,
    /// Record creation time; it also sets the report date. Use the same value in preview and submit.
    #[arg(long)]
    timestamp: DateTime<Utc>,
    /// Replace the month's accepted report instead of sending a new one.
    #[arg(long)]
    replace: bool,
}

#[derive(Subcommand, Debug)]
pub enum EmployerReportCmd {
    /// Offline, writes nothing: the exact report, its deadline and the record_hash an approval names. Example: file-wage employer-report preview --month 2026-10 --payments none --timestamp 2026-11-02T08:00:00Z
    Preview {
        #[command(flatten)]
        args: EmployerReportArgs,
    },
    /// Approve the previewed record_hash and send once. Refused when the month already has an accepted report or an unsettled send. Example: file-wage employer-report submit --month 2026-10 --payments none --timestamp 2026-11-02T08:00:00Z --record-hash HASH
    Submit {
        #[command(flatten)]
        args: EmployerReportArgs,
        #[arg(long)]
        record_hash: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// The month's state and receipt. An unknown outcome is settled with the signed status query; nothing is ever resent. Example: file-wage employer-report status --month 2026-10
    Status {
        #[arg(long, value_parser = parse_month)]
        month: ReportMonth,
    },
    /// Read-only: months since the employer-register entry with no accepted report, each with its deadline. Months an imported transcript 630 shows reported are left out and listed under filed_outside with their evidence. Offline. Example: file-wage employer-report missing
    Missing {
        /// Date to judge against (default: today, UTC).
        #[arg(long)]
        as_of: Option<NaiveDate>,
    },
    /// Record e-service evidence: the answer to an unsettled send, or a report given outside aggountant. Example: file-wage employer-report complete-manual --month 2026-07 --ir-report-id GUID
    CompleteManual {
        #[arg(long, value_parser = parse_month)]
        month: ReportMonth,
        #[arg(long)]
        ir_report_id: String,
        #[arg(long)]
        ir_delivery_id: Option<String>,
        #[arg(long)]
        report_version: Option<u32>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Read-only, online: the employer-register (työnantajarekisteri) entries PRH/YTJ open data holds for the profile's business_id, compared with employer_report.registered_from (match, profile_later_than_register, profile_earlier_than_register, register_ended, not_in_register, profile_not_set). implied_registered_from is the value to write on the user's say-so; nothing is written. A network failure is YTJ_UNAVAILABLE, never "not registered". Example: file-wage employer-report registration
    Registration {
        /// Date to judge the entries against (default: today, UTC).
        #[arg(long)]
        as_of: Option<NaiveDate>,
    },
    /// Import Incomes Register transcript 630 "Maksajan palkkatietojen yhteenveto" (CSV, ordered in Finnish in the e-service without a reception time range): stores the file as an artifact and records each month with at least one employer's separate report as filed outside aggountant. Months aggountant holds are not overwritten; conflicts are reported, and a month two transcripts disagree on becomes transcript_conflict and refuses every send; the same file again is a no-op; an unknown header is refused (TRANSCRIPT_UNKNOWN_HEADER). Such months have no report reference, so --replace for them is refused. Example: file-wage employer-report import-transcript raportti-630.csv
    ImportTranscript {
        /// The CSV file downloaded from the e-service (unzip a ZIP download first).
        file: std::path::PathBuf,
        /// Show what would be recorded; write nothing.
        #[arg(long)]
        dry_run: bool,
    },
}

fn parse_month(s: &str) -> Result<ReportMonth, String> {
    ReportMonth::parse(s).map_err(|e| e.to_string())
}

impl EmployerReportArgs {
    fn input(&self) -> EmployerReportInput {
        EmployerReportInput {
            month: self.month,
            payments: match self.payments {
                Payments::None => MonthPayments::None,
                Payments::TaxFreeReimbursementsOnly => MonthPayments::TaxFreeReimbursementsOnly,
                Payments::Wages => MonthPayments::Wages,
            },
            replace: self.replace,
        }
    }
}

fn employer_report(cmd: &EmployerReportCmd, home: &Path) -> Result<Value, CliError> {
    let engine = ProcessEngine::open(home);
    let value = match cmd {
        EmployerReportCmd::Preview { args } => {
            serde_json::to_value(engine.preview_employer_report(args.input(), args.timestamp)?)
        }
        EmployerReportCmd::Submit {
            args,
            record_hash,
            dry_run,
        } => {
            let preview = engine.preview_employer_report(args.input(), args.timestamp)?;
            if *dry_run {
                return Ok(
                    json!({"dry_run":true,"would":[{"action":"submit","resource":"file-wage employer-report","matches_approval":preview.record_hash == *record_hash,"preview":preview}]}),
                );
            }
            serde_json::to_value(engine.submit_employer_report(
                args.input(),
                args.timestamp,
                record_hash,
                &broker(home)?,
                Utc::now(),
            )?)
        }
        EmployerReportCmd::Status { month } => {
            if engine.employer_report_in_doubt(*month)? {
                serde_json::to_value(engine.reconcile_employer_report(
                    *month,
                    &broker(home)?,
                    Utc::now(),
                )?)
            } else {
                serde_json::to_value(engine.employer_report_status(*month)?)
            }
        }
        EmployerReportCmd::Missing { as_of } => serde_json::to_value(
            engine.missing_employer_reports(as_of.unwrap_or_else(|| Utc::now().date_naive()))?,
        ),
        EmployerReportCmd::CompleteManual {
            month,
            ir_report_id,
            ir_delivery_id,
            report_version,
            dry_run,
        } => {
            refuse_dry_run(*dry_run)?;
            serde_json::to_value(engine.complete_employer_report_manual(
                *month,
                EmployerReportReceipt {
                    ir_delivery_id: ir_delivery_id.clone(),
                    ir_report_id: ir_report_id.clone(),
                    version: *report_version,
                    source: "e_service".into(),
                },
                Utc::now(),
            )?)
        }
        EmployerReportCmd::Registration { as_of } => {
            let lookup =
                HttpYtj::new().map_err(|e| CliError::system("YTJ_UNAVAILABLE", e.to_string()))?;
            serde_json::to_value(
                engine.employer_registration(
                    &lookup,
                    as_of.unwrap_or_else(|| Utc::now().date_naive()),
                )?,
            )
        }
        EmployerReportCmd::ImportTranscript { file, dry_run } => {
            let bytes = std::fs::read(file).map_err(|e| {
                CliError::user(
                    "TRANSCRIPT_UNREADABLE",
                    format!("could not read {}: {e}", file.display()),
                )
            })?;
            serde_json::to_value(engine.import_employer_transcript(&bytes, Utc::now(), *dry_run)?)
        }
    };
    value.map_err(|e| CliError::system("INTERNAL_ERROR", e.to_string()))
}
pub fn execute(cmd: &WageCmd, home: &Path) -> Result<Value, CliError> {
    if let WageCmd::EmployerReport(cmd) = cmd {
        return employer_report(cmd, home).map(|mut v| {
            v["schema_version"] = json!(1);
            v
        });
    }
    let engine = ProcessEngine::open(home);
    let value = match cmd {
        WageCmd::EmployerReport(_) => unreachable!("handled above"),
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
